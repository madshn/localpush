# LocalPush Sources Reference

All available data sources, their configurable properties, and real payload examples.

---

## Source Index

| Source ID | Name | Watch Mechanism | Properties |
|-----------|------|----------------|------------|
| `claude-stats` | Claude Code Statistics | FSEvents on `~/.claude/projects/` (recursive) | `daily_breakdown`, `model_totals` |
| `claude-sessions` | Claude Code Sessions | FSEvents on `~/.claude/projects/` (recursive) | `sessions`, `cache_efficiency`, `model_distribution`, `git_branches`, `first_prompt_preview` |
| `codex-stats` | Codex Statistics | FSEvents on `~/.codex/sessions/` (recursive) | `metrics` |
| `codex-sessions` | Codex Sessions | FSEvents on `~/.codex/sessions/` (recursive) | `sessions`, `summary` |
| `apple-podcasts` | Apple Podcasts | FSEvents on Podcasts SQLite DB | `recent_episodes`, `episode_links`, `transcript_snippets`, `podcast_metadata` |
| `apple-notes` | Apple Notes | FSEvents on NoteStore.sqlite, JXA for reads | `recent_notes`, `folder_stats` |
| `apple-photos` | Apple Photos | FSEvents on Photos.sqlite | `library_stats`, `recent_photos`, `photo_location`, `photo_faces`, `photo_labels` |
| `desktop-activity` | Desktop Activity | IOKit idle time polling (no file watch) | *(none)* |

---

## claude-stats

Aggregates Claude Code usage across all projects over a configurable recent-day window (default 30 days, max 30). Scans `.jsonl` session files in `~/.claude/projects/` recursively, filtering by message timestamp (not file mtime).

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `daily_breakdown` | Daily Breakdown | on | no | Daily rolling stats with messages and tokens across the configured data window |
| `model_totals` | Model Totals | on | no | Per-model token counts and usage across the configured data window |

### Sample Payload

```json
{
  "version": 2,
  "last_computed_date": "2026-02-22",
  "today": null,
  "yesterday": null,
  "daily_breakdown": [
    {
      "date": "2026-02-27",
      "messages": 0,
      "sessions": 0,
      "tool_calls": 0,
      "tokens_by_model": {},
      "total_tokens": 0
    }
  ],
  "model_totals": [
    {
      "model": "claude-opus-4-6",
      "input_tokens": 1346333,
      "output_tokens": 3890466,
      "cache_creation_tokens": 207972559,
      "cache_read_tokens": 3501761748,
      "total_tokens": 5236799
    },
    {
      "model": "claude-sonnet-4-6",
      "input_tokens": 4828,
      "output_tokens": 69651,
      "cache_creation_tokens": 2672046,
      "cache_read_tokens": 77354459,
      "total_tokens": 74479
    }
  ],
  "summary": {
    "total_sessions": 1116,
    "total_messages": 328372,
    "first_session_date": "2025-12-23T22:45:40.170Z",
    "days_active": 60,
    "peak_hour": 13
  },
  "metadata": {
    "source": "localpush",
    "generated_at": "2026-03-11T23:01:43.580432Z",
    "file_path": "/Users/madsnissen/.claude/stats-cache.json"
  }
}
```

`daily_breakdown` contains one entry per day in the configured window (oldest→newest), zero-filled for inactive days. `tool_calls` is always 0 (unavailable from JSONL). `total_tokens` in `model_totals` is input+output only; cache tokens are tracked separately.

---

## claude-sessions

Lists individual Claude Code sessions from a configurable recent-day window (default 7 days, max 30). Two discovery strategies: primary scans `.jsonl` files by mtime, fallback reads `sessions-index.json`. JSONL-discovered sessions take precedence on dedup.

When LocalPush builds the delivery payload, it excludes Claude sessions rooted inside a Cloud Agent Host sandbox repo so CAH-owned telemetry is not double-counted downstream. Override the sandbox repo root with `CAH_REPOS_ROOT`; otherwise LocalPush uses `$SANDBOX_ROOT/repos` when `SANDBOX_ROOT` is set, falling back to `~/local-cloud-agent-host/repos`. Local preview remains unchanged and still shows all local sessions.

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `sessions` | Sessions | on | no | Session list with metadata from the configured data window |
| `cache_efficiency` | Cache Efficiency | on | no | Cache hit rate and prompt caching metrics |
| `model_distribution` | Model Distribution | on | no | Which Claude models were used across sessions |
| `git_branches` | Git Branches | on | no | Git branch context per session |
| `first_prompt_preview` | First Prompt Preview | **off** | **yes** | First 120 chars of each session's opening prompt |

### Sample Payload

```json
{
  "source": "claude_code_sessions",
  "timestamp": "2026-03-16T17:30:00Z",
  "sessions": [
    {
      "id": "9c2f545b-c23f-42fc-b96e-728e5234c609",
      "project_path": "/Users/madsnissen/ops/bob",
      "git_branch": "main",
      "title": "Standup v2 protocol rollout to associates",
      "start_time": "2026-03-16T08:32:08.211Z",
      "end_time": "2026-03-16T17:25:11.029Z",
      "duration_seconds": 31982,
      "message_count": 488,
      "model": "claude-opus-4-6",
      "tokens": {
        "input": 953,
        "output": 119327,
        "cache_creation": 1259126,
        "cache_read": 140862354
      }
    },
    {
      "id": "a397e45e-efed-423c-899a-5b481bbac7a5",
      "project_path": "/Users/madsnissen/local-cloud-agent-host/repos/associate/rex",
      "git_branch": "main",
      "title": "Stripe events schema design",
      "start_time": "2026-03-16T16:11:43.743Z",
      "end_time": "2026-03-16T16:12:25.231Z",
      "duration_seconds": 41,
      "message_count": 7,
      "model": "claude-sonnet-4-6",
      "tokens": {
        "input": 20,
        "output": 1922,
        "cache_creation": 94086,
        "cache_read": 198833
      }
    }
  ],
  "summary": {
    "sessions_in_window": 14,
    "total_tokens_in_window": 620000,
    "total_duration_in_window_seconds": 87300
  },
  "window_days": 7
}
```

`title` comes from session `summary` message type, falling back to first 120 chars of the first user prompt. `project_path` decoded from directory name convention when `cwd` is unavailable.

---

## codex-stats

Reports token usage for Codex (OpenAI) sessions. Aggregates a configurable UTC day window (default 1 day, max 30) from cumulative token snapshot deltas across `.jsonl` files in `~/.codex/sessions/`. LocalPush emits one metric per day in the selected window, including zero-activity days, so you can backfill gaps cleanly.

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `metrics` | Metrics | on | no | Leaf KPI metrics for each day in the configured UTC data window |

### Sample Payload

```json
{
  "metrics": [
    {
      "metric_key": "token.openai.codex",
      "period_from": "2026-03-14T00:00:00Z",
      "period_to": "2026-03-15T00:00:00Z",
      "value": 0,
      "source": "localpush",
      "cost_model": "subscription",
      "tags": {
        "input": 0,
        "cached_input": 0,
        "output": 0,
        "reasoning_output": 0
      }
    }
  ]
}
```

Only emits the unversioned `token.openai.codex` leaf metric. Per-model versioned metrics are intentionally withheld pending reliable per-model attribution. Token totals use `saturating_delta` on cumulative counters to avoid double-counting.

---

## codex-sessions

Lists individual Codex sessions from a configurable recent-day window (default 7 days, max 30). Model names are normalized to canonical keys (e.g. `gpt-5.3-codex` → `openai.codex.5_3`).

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `sessions` | Sessions | on | **yes** | Session list with token totals and context |
| `summary` | Summary | on | no | Aggregated token and session totals |

### Sample Payload

```json
{
  "source": "codex_sessions",
  "schema_version": 1,
  "source_family": "codex",
  "source_type": "sessions",
  "semantics": {
    "token_count_basis": "session_max_of_event_msg.token_count.info.total_token_usage",
    "message_count_basis": "count(event_msg.user_message)",
    "duration_basis": "session_meta.timestamp_to_last_event_timestamp",
    "dedupe_basis": "one_record_per_jsonl_session_file",
    "window": { "mode": "recent_days", "days": 7 },
    "unsupported_metrics": ["cache_creation_tokens"],
    "notes": [
      "cache_read maps to Codex cached_input_tokens",
      "reasoning_output is included inside tokens for schema parity",
      "agentic_seconds is an estimate using token_count event gaps < 5 minutes"
    ]
  },
  "sessions": [
    {
      "id": "019c9e87-2000-7b43-b2ff-3c11c95e5504",
      "project_path": "/Users/madsnissen/dev/localpush",
      "git_branch": "main",
      "title": "AGENTS.md instructions for /Users/madsnissen/dev/localpush...",
      "start_time": "2026-02-27T09:56:21.632+00:00",
      "end_time": "2026-02-27T10:55:53.755+00:00",
      "session_span_seconds": 3572,
      "agentic_seconds": 691,
      "message_count": 5,
      "model": "openai.codex.5_3",
      "tokens": {
        "input": 5430926,
        "output": 15160,
        "cache_read": 5179776,
        "cache_creation": 0,
        "reasoning_output": 4325
      }
    }
  ],
  "summary": {
    "sessions_count": 6,
    "total_tokens": 142500,
    "total_duration_seconds": 54000,
    "total_agentic_seconds": 43200,
    "total_input_tokens": 98000,
    "total_output_tokens": 32500,
    "total_cached_input_tokens": 12000,
    "total_reasoning_output_tokens": 8100
  }
}
```

`agentic_seconds` estimates active coding time using event gaps < 5 min. `cache_creation` is always 0 (unavailable in Codex). Token totals use maximum cumulative snapshot, not sum of deltas.

---

## apple-podcasts

Reads Apple Podcasts Core Data SQLite database directly (read-only). Requires Full Disk Access. Returns episodes from the last 7 days with play data, extracted links, and optional transcript snippets.

**Watch path:** `~/Library/Group Containers/243LU875E5.groups.com.apple.podcasts/Documents/MTLibrary.sqlite`

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `recent_episodes` | Recent Episodes | on | no | Episode list with play data from the last 7 days |
| `episode_links` | Episode Links | on | no | URLs extracted from episode descriptions |
| `transcript_snippets` | Transcript Snippets | **off** | no | Preview text from episode transcripts |
| `podcast_metadata` | Podcast Metadata | on | no | Podcast-level metadata (show name, counts) |

### Sample Payload

```json
{
  "source": "apple_podcasts",
  "timestamp": "2026-03-16T08:20:00Z",
  "recent_episodes": [
    {
      "episode_title": "Executive Briefing: One solo founder just sold for $80M in 6 months...",
      "podcast_name": "Nate's Notebook (private feed for Mads.nissen@gmail.com)",
      "duration_seconds": 2259.0,
      "play_count": 0,
      "last_played": "2026-03-16T08:16:26+00:00",
      "episode_url": "https://natesnewsletter.substack.com/p/executive-briefing-one-solo-founder",
      "has_transcript": false,
      "transcript_snippet": null,
      "links": [
        {
          "source": "anchor",
          "url": "https://support.substack.com/hc/en-us/articles/360044105731-..."
        },
        {
          "source": "anchor",
          "url": "https://natesnewsletter.substack.com/p/executive-briefing-..."
        }
      ]
    }
  ],
  "stats": {
    "total_episodes": 1842,
    "total_podcasts": 23,
    "recent_count": 7
  }
}
```

Returns up to 50 episodes. Links extracted from HTML descriptions: `href="..."` anchors first, then bare `https://` URLs. `acast.com/privacy` URLs are filtered as boilerplate.

---

## apple-notes

Watches NoteStore.sqlite for change detection, then collects data via JXA (JavaScript for Automation) to avoid direct access to the encrypted Notes database. Returns **metadata only** — no note content.

**Watch path:** `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite`

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `recent_notes` | Recent Notes | on | no | Note titles and folders from the last 7 days |
| `folder_stats` | Folder Statistics | on | no | Per-folder note counts |

### Sample Payload

```json
{
  "source": "apple_notes",
  "timestamp": "2026-03-11T23:01:45.111763+00:00",
  "recent_notes": [],
  "stats": {
    "total_notes": 28,
    "recent_count": 0,
    "folders": {
      "Notes": 28
    }
  }
}
```

`recent_notes` filtered to notes modified in last 7 days. `total_notes` is full library count from JXA. Folder counts cover the 50-note JXA fetch window, not full library. Note content is never included.

---

## apple-photos

Reads Photos.sqlite directly (read-only). Schema-adaptive — handles column variations across macOS versions for filenames, faces, and person names. Requires Full Disk Access.

**Watch path:** `~/Pictures/Photos Library.photoslibrary/database/Photos.sqlite`

### Properties

| Key | Label | Default | Sensitive | Description |
|-----|-------|---------|-----------|-------------|
| `library_stats` | Library Statistics | on | no | Aggregate counts of photos, videos, albums |
| `recent_photos` | Recent Photos | **off** | **yes** | New photos with metadata (filenames, dates) from the last 7 days |
| `photo_location` | Photo Locations | **off** | **yes** | GPS coordinates where photos were taken |
| `photo_faces` | Detected Faces | **off** | **yes** | Named faces recognized in photos |
| `photo_labels` | ML Content Labels | **off** | **yes** | ML-detected content labels (stub — always empty) |

### Sample Payload

```json
{
  "source": "apple_photos",
  "timestamp": "2026-03-16T18:23:30.626734+00:00",
  "library": {
    "total_photos": 44120,
    "total_videos": 6355,
    "total_assets": 50475,
    "favorites": 300,
    "recent_imports_7d": 34,
    "albums": 67
  },
  "recent_photos": [
    {
      "uuid": "3DF01301-B9F2-4DE4-858D-29A9C13B5CDA",
      "filename": "3DF01301-B9F2-4DE4-858D-29A9C13B5CDA.heic",
      "date_created": "2026-02-23T07:11:43+00:00",
      "date_added": "2026-02-23T07:11:43+00:00",
      "photo_type": "other",
      "latitude": 59.94363833333333,
      "longitude": 10.742945,
      "faces": ["Mikkel"],
      "labels": []
    }
  ]
}
```

Returns up to 50 recent photos. Only non-trashed assets (`ZTRASHEDSTATE = 0`). Album count uses `ZKIND = 2` (user albums). `-180.0` coordinates indicate missing GPS data. `labels` always empty pending implementation.

---

## desktop-activity

Tracks active desktop sessions using macOS IOKit idle time. No file watch — uses a polling state machine. Sessions start when idle time drops below 180s, end when it exceeds 180s.

**Watch path:** None (polling-based)

### Properties

None — no configurable properties.

### Sample Payload

```json
{
  "type": "desktop_session",
  "start_timestamp": 1773683413,
  "end_timestamp": 1773683893,
  "duration_minutes": 8.0,
  "idle_threshold_seconds": 180.0,
  "metadata": {
    "source": "localpush",
    "source_id": "desktop-activity",
    "generated_at": "2026-03-16T17:58:43.934145+00:00"
  }
}
```

Timestamps are Unix epoch. Sessions are drained on `parse()` — each session appears in exactly one delivery (consume-once). `session_count` and `total_minutes` fields appear only when multiple sessions are batched. No Accessibility permissions required.
