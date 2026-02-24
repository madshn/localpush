#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path


ISO_TS_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
UUIDISH_RE = re.compile(
    r"^[0-9a-f]{8,}-[0-9a-f-]{8,}$",
    re.IGNORECASE,
)

PRESERVE_STRING_KEYS = {
    "type",
    "role",
    "id",
    "session_id",
    "sessionId",
    "model",
    "model_provider",
    "source",
    "originator",
    "cli_version",
    "commit_hash",
    "branch",
    "state",
    "status",
    "repository_url",  # handled separately by URL sanitizer
    "cwd",  # handled separately by path sanitizer
    "timestamp",
    "createdAt",
    "updatedAt",
}

PATHISH_KEYS = {
    "cwd",
    "workdir",
    "path",
    "file",
    "filePath",
    "entrypoint_path",
}

URLISH_KEYS = {
    "repository_url",
    "url",
    "href",
    "webhookPath",
}

TEXTISH_KEYS = {
    "message",
    "text",
    "content",
    "prompt",
    "justification",
    "description",
    "body",
    "summary",
    "question",
    "cmd",
    "instruction",
    "developer_instructions",
    "user_instructions",
    "base_instructions",
    "input",
    "arguments",
    "output",
    "last_agent_message",
    "new_str",
    "selection_with_ellipsis",
}


class Sanitizer:
    def __init__(self) -> None:
        self.path_map: dict[str, str] = {}
        self.url_map: dict[str, str] = {}
        self.string_map: dict[str, str] = {}

    def _hash(self, s: str, n: int = 10) -> str:
        return hashlib.sha256(s.encode("utf-8")).hexdigest()[:n]

    def sanitize_path(self, value: str) -> str:
        if value in self.path_map:
            return self.path_map[value]
        token = f"/redacted/path/{self._hash(value)}"
        self.path_map[value] = token
        return token

    def sanitize_url(self, value: str) -> str:
        if value.startswith("https://") or value.startswith("http://") or value.startswith("git@"):
            if value in self.url_map:
                return self.url_map[value]
            token = f"redacted://url/{self._hash(value)}"
            self.url_map[value] = token
            return token
        return value

    def sanitize_text(self, key: str, value: str) -> str:
        key_prefix = key or "text"
        digest = self._hash(value, 8)
        return f"[REDACTED_{key_prefix.upper()} len={len(value)} sha={digest}]"

    def sanitize_generic_string(self, key: str, value: str) -> str:
        if key in PATHISH_KEYS:
            return self.sanitize_path(value)
        if key in URLISH_KEYS:
            return self.sanitize_url(value)

        if ISO_TS_RE.match(value) or DATE_RE.match(value):
            return value

        if key.lower().endswith("timestamp") or key.lower().endswith("_timestamp"):
            return value

        if key in {"model", "model_provider", "cli_version", "type", "role", "source", "originator"}:
            return value

        if key in {"id", "session_id", "sessionId"} and UUIDISH_RE.match(value):
            return value

        if key in TEXTISH_KEYS:
            return self.sanitize_text(key, value)

        # Heuristic path redaction
        if value.startswith("/Users/") or value.startswith("~/") or value.startswith("/"):
            return self.sanitize_path(value)

        # Heuristic URL redaction
        if "://" in value or value.startswith("git@"):
            return self.sanitize_url(value)

        # Keep short enum-like strings, redact long free text.
        if len(value) <= 40 and re.match(r"^[A-Za-z0-9._:/-]+$", value):
            return value

        return self.sanitize_text(key or "string", value)

    def sanitize(self, obj, parent_key: str = ""):
        if isinstance(obj, dict):
            out = {}
            for k, v in obj.items():
                out[k] = self.sanitize(v, k)
            return out
        if isinstance(obj, list):
            return [self.sanitize(v, parent_key) for v in obj]
        if isinstance(obj, str):
            return self.sanitize_generic_string(parent_key, obj)
        return obj


def main() -> int:
    parser = argparse.ArgumentParser(description="Capture and sanitize Codex JSONL fixture")
    parser.add_argument("--input-dir", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--fixture-date", required=True)
    args = parser.parse_args()

    input_dir = Path(args.input_dir).expanduser().resolve()
    output_dir = Path(args.output_dir).resolve()
    raw_out = output_dir / "raw" / "sessions"
    expected_out = output_dir / "expected"
    raw_out.mkdir(parents=True, exist_ok=True)
    expected_out.mkdir(parents=True, exist_ok=True)

    sanitizer = Sanitizer()
    session_files = sorted(input_dir.glob("*.jsonl"))
    line_count_total = 0
    type_counts: Counter[str] = Counter()

    for src in session_files:
        dst = raw_out / src.name
        with src.open("r", encoding="utf-8") as f_in, dst.open("w", encoding="utf-8") as f_out:
            for line in f_in:
                if not line.strip():
                    continue
                line_count_total += 1
                try:
                    obj = json.loads(line)
                except json.JSONDecodeError:
                    # Preserve malformed lines as-is but redact aggressively to keep parser edge cases.
                    f_out.write(json.dumps({"type": "malformed_line", "raw": sanitizer.sanitize_text("raw", line.rstrip())}) + "\n")
                    type_counts["malformed_line"] += 1
                    continue

                msg_type = obj.get("type")
                if isinstance(msg_type, str):
                    type_counts[msg_type] += 1
                else:
                    type_counts["<missing>"] += 1

                sanitized = sanitizer.sanitize(obj)
                f_out.write(json.dumps(sanitized, separators=(",", ":")) + "\n")

    readme = output_dir / "README.md"
    readme.write_text(
        "\n".join(
            [
                f"# Codex Fixture {args.fixture_date}",
                "",
                "Sanitized fixture derived from real Codex session JSONL logs.",
                "",
                "Sanitization guarantees:",
                "- token counts preserved",
                "- timestamps preserved",
                "- event ordering preserved",
                "- models preserved",
                "- free-text content redacted",
                "- local paths and URLs pseudonymized",
                "",
                "Notes:",
                "- `expected/` files are placeholders until parser/schema outputs are finalized.",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    manifest_path = output_dir / "manifest.json"
    manifest = {
        "manifest_version": 1,
        "source_family": "codex",
        "fixture_date": args.fixture_date,
        "capture_status": "sanitized_raw_captured_unverified",
        "day_boundary": {
            "mode": "local",
            "timezone": "TO_BE_FILLED",
            "start_inclusive": "TO_BE_FILLED",
            "end_exclusive": "TO_BE_FILLED",
        },
        "sanitization": {
            "text_redacted": True,
            "paths_pseudonymized": True,
            "urls_pseudonymized": True,
            "ids_pseudonymized": False,
            "models_preserved": True,
            "token_counts_preserved": True,
            "timestamps_preserved": True,
        },
        "input_files": {
            "session_file_count": len(session_files),
            "jsonl_line_count_total": line_count_total,
            "source_directory": "/redacted/codex/sessions/2026/02/23",
        },
        "observed_line_types": dict(type_counts),
        "verification": {
            "status": "pending_manual_verification",
            "sessions_in_scope": None,
            "malformed_lines_skipped": None,
            "duplicate_events_detected": None,
            "duplicate_events_collapsed": None,
            "first_event_timestamp": None,
            "last_event_timestamp": None,
            "models_used": [],
            "token_totals": {
                "input": None,
                "output": None,
                "total": None,
                "cache_read": None,
                "cache_creation": None,
            },
            "notes": [
                "Populate after manual verification against raw source logs for 2026-02-23."
            ],
        },
        "expected_outputs": {
            "codex_sessions": "expected/codex-sessions.json",
            "codex_stats": "expected/codex-stats.json",
        },
    }
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    # Placeholder golden files to establish fixture layout now.
    for p in [expected_out / "codex-sessions.json", expected_out / "codex-stats.json"]:
        if not p.exists():
            p.write_text('{\n  "_status": "pending"\n}\n', encoding="utf-8")

    print(f"Captured {len(session_files)} files, {line_count_total} JSONL lines")
    print(f"Output: {output_dir}")
    print("Observed line types:")
    for k, v in sorted(type_counts.items()):
        print(f"  {k}: {v}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
