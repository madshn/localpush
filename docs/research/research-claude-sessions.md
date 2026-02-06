# Claude Code Session Data Research

**Date:** 2026-02-06
**Purpose:** Design a "Claude Code Sessions" source for LocalPush that captures session activity logs

---

## Session Data Locations

### Primary Session Storage

**Per-project session data:**
- **Location:** `~/.claude/projects/{project-path-encoded}/{sessionId}.jsonl`
- **Format:** JSONL (one JSON object per line)
- **Structure:** Each line is an event with type, timestamp, message content, etc.

**Example path:**
```
~/.claude/projects/-Users-madsnissen-ops-bob/60394d86-f4f4-4ea1-a2b9-d2480ce8c3ec.jsonl
```

### Session Index

**Location:** `~/.claude/projects/{project-path-encoded}/sessions-index.json`

**Contains:**
- Session metadata aggregated across all sessions for that project
- Fields: sessionId, firstPrompt, summary, messageCount, created, modified, gitBranch, projectPath

**Sample entry:**
```json
{
  "sessionId": "3d94df65-c74f-4652-8374-0de8919453f3",
  "fullPath": "/Users/madsnissen/.claude/projects/-Users-madsnissen-ops-bob/3d94df65-c74f-4652-8374-0de8919453f3.jsonl",
  "fileMtime": 1768979655292,
  "firstPrompt": "No prompt",
  "summary": "Mira adopted: Runtime ops agent setup complete",
  "messageCount": 6,
  "created": "2026-01-20T20:39:37.130Z",
  "modified": "2026-01-20T20:45:09.062Z",
  "gitBranch": "main",
  "projectPath": "/Users/madsnissen/ops/bob",
  "isSidechain": false
}
```

### Global Activity Tracking

**Location:** `~/.claude/stats-cache.json`

**Contains:**
- Daily aggregated stats (messageCount, sessionCount, toolCallCount)
- Model token usage by day and by model
- Total sessions, messages
- Longest session metadata
- Hour-of-day distribution

**Limitations:**
- Aggregated by day only (no per-session detail)
- No session titles/topics
- No project-specific breakdown

### Command History

**Location:** `~/.claude/history.jsonl`

**Format:** JSONL with entries like:
```json
{
  "display": "continue work on localpush...",
  "pastedContents": {},
  "timestamp": 1770333452570,
  "project": "/Users/madsnissen/ops/bob",
  "sessionId": "60394d86-f4f4-4ea1-a2b9-d2480ce8c3ec"
}
```

**Contains:**
- User input commands/messages
- Timestamp
- Project path
- Session ID

**Use case:** Could correlate with session data for full context.

---

## Session JSONL Structure

**Event types in session JSONL:**
- `type: "user"` — User messages
- `type: "assistant"` — Assistant responses
- `type: "progress"` — Hook execution, tool calls
- `type: "file-history-snapshot"` — File state snapshots
- `type: "summary"` — Session summary (appended by SessionEnd hook)

**Key fields per event:**
```json
{
  "type": "user",
  "sessionId": "60394d86-f4f4-4ea1-a2b9-d2480ce8c3ec",
  "timestamp": "2026-02-05T21:45:11.525Z",
  "cwd": "/Users/madsnissen/ops/bob",
  "gitBranch": "main",
  "version": "2.1.32",
  "message": {
    "role": "user",
    "content": "continue work on localpush..."
  },
  "uuid": "f3e2d005-eb6d-4e56-8ecc-56a7343eb760"
}
```

**Token usage in assistant messages:**
```json
{
  "type": "assistant",
  "message": {
    "usage": {
      "input_tokens": 3,
      "cache_creation_input_tokens": 22038,
      "cache_read_input_tokens": 21770,
      "output_tokens": 12
    }
  }
}
```

---

## Hook System

**Hook directory:** `~/.claude/hooks/`

**Available hooks:**
- `auto-rename-session.sh` — SessionEnd hook that generates session titles
- `ntfy-complete.sh` — Completion notifications
- `ntfy-notify.sh` — General notifications
- `set-terminal-title.sh` — Terminal title updates

### SessionEnd Hook Pattern

**Hook:** `auto-rename-session.sh`

**Receives:**
```json
{
  "transcript_path": "/path/to/session.jsonl"
}
```

**Behavior:**
1. Reads session JSONL
2. Extracts conversation snippets (last 8 assistant messages)
3. Uses haiku model to generate a title
4. Appends summary entry to JSONL:
```json
{
  "type": "summary",
  "summary": "Generated session title",
  "leafUuid": "last-message-uuid"
}
```

**Key insight:** Hook writes BACK to session JSONL. This is the pattern for LocalPush integration.

---

## Recommended Watch Paths

### Option 1: Watch sessions-index.json (simplest)

**Path:** `~/.claude/projects/*/sessions-index.json`

**Pros:**
- Single file per project with all session metadata
- Already aggregated (messageCount, created, modified, summary)
- Easy to parse

**Cons:**
- Only updates when Claude Code updates the index (may lag)
- No real-time session events

### Option 2: Watch session JSONL files

**Path:** `~/.claude/projects/*/*.jsonl`

**Pros:**
- Real-time session events
- Full event stream (user, assistant, progress)
- Can extract token usage per session

**Cons:**
- High volume (every message = new line)
- Need to parse JSONL incrementally
- Requires filtering to session end events

### Option 3: Hybrid (recommended)

**Primary:** Watch `sessions-index.json` for session summaries
**Secondary:** Read session JSONL on-demand for token details

---

## What Can Be Extracted (Without Hooks)

From `sessions-index.json`:
- Session start time (`created`)
- Session end time (`modified`)
- Duration (calculated: `modified - created`)
- Message count (`messageCount`)
- Session title/topic (`summary` — if SessionEnd hook ran)
- Project path (`projectPath`)
- Git branch (`gitBranch`)

From session JSONL (requires parsing):
- Token count per session (sum all `message.usage.input_tokens` + `output_tokens`)
- Model used (`message.model`)
- Cache usage (`cache_read_input_tokens`, `cache_creation_input_tokens`)
- Tool call count (count `type: "progress"` entries)

From `stats-cache.json`:
- Daily aggregates (useful for validation)

---

## What Requires Hook Integration

**Session-end triggered push:**

To push session data IMMEDIATELY when a session ends (not on next file watch), a SessionEnd hook is needed.

**Hook approach:**
1. Create `~/.claude/hooks/localpush-session-end.sh`
2. Hook receives `transcript_path` (session JSONL path)
3. Hook reads session JSONL, extracts metadata + token counts
4. Hook writes to a known location (e.g., `~/.localpush/sessions.jsonl`)
5. LocalPush watches `~/.localpush/sessions.jsonl` and pushes new entries

**Benefit:** Zero-latency session capture. Data available immediately after session ends.

**Drawback:** Requires hook installation (but Claude Code supports this natively).

---

## Sample JSON Payload for LocalPush Sessions Source

```json
{
  "source": "claude_code_sessions",
  "timestamp": "2026-02-06T10:05:16Z",
  "session": {
    "id": "60394d86-f4f4-4ea1-a2b9-d2480ce8c3ec",
    "project_path": "/Users/madsnissen/ops/bob",
    "git_branch": "main",
    "title": "LocalPush Layer 2 planning and source design",
    "start_time": "2026-02-05T21:44:48Z",
    "end_time": "2026-02-06T10:05:16Z",
    "duration_seconds": 44428,
    "message_count": 47,
    "tokens": {
      "input": 12589,
      "output": 8234,
      "cache_read": 456789,
      "cache_creation": 23456
    },
    "model": "claude-opus-4-6",
    "tool_calls": 23,
    "first_prompt": "continue work on localpush...",
    "summary": "LocalPush Layer 2 planning and source design"
  }
}
```

---

## Implementation Recommendations

### Phase 1: File Watch (No Hooks)

**Watch:** `~/.claude/projects/*/sessions-index.json`

**On change:**
1. Read sessions-index.json
2. Identify new/updated sessions (compare `modified` timestamp)
3. For each new session, read corresponding `.jsonl` file
4. Parse JSONL to extract token counts
5. Build payload and push to configured targets

**Pros:**
- No hook installation required
- Works with existing Claude Code setup

**Cons:**
- Not real-time (sessions-index updates may lag)
- Requires JSONL parsing for token details

### Phase 2: SessionEnd Hook (Real-time)

**Create hook:** `~/.claude/hooks/localpush-session-end.sh`

**Hook logic:**
```bash
#!/bin/bash
INPUT=$(cat)
TRANSCRIPT=$(echo "$INPUT" | jq -r '.transcript_path')

# Parse session JSONL
SESSION_DATA=$(python3 parse_session.py "$TRANSCRIPT")

# Write to LocalPush queue
echo "$SESSION_DATA" >> ~/.localpush/sessions.jsonl
```

**LocalPush watches:** `~/.localpush/sessions.jsonl`

**Pros:**
- Real-time session capture
- Zero polling overhead
- Clean separation (hook writes, LocalPush reads)

**Cons:**
- Requires hook installation (but this is standard Claude Code)

### Phase 3: Advanced Metrics (Future)

**Additional data sources:**
- Sub-agent sessions (`{sessionId}/subagents/agent-*.jsonl`)
- Task outputs (if Task tool is used)
- File history snapshots (detect which files were modified)

---

## Limitations and Gaps

### No Global Session Database

Claude Code does NOT have a centralized session database. All session data is:
- Stored per-project in separate directories
- Aggregated only in per-project `sessions-index.json`
- No cross-project session view

**Impact:** LocalPush must watch MULTIPLE `sessions-index.json` files (one per project).

### Token Data Not in sessions-index.json

`sessions-index.json` does NOT include token counts. Token usage is ONLY in session JSONL.

**Impact:** Must parse JSONL files to get token counts for each session.

### Session End Detection

Without a hook, detecting "session ended" requires:
- Polling `sessions-index.json` for `modified` timestamp changes
- OR watching JSONL file for `type: "summary"` entry

**Recommendation:** Use SessionEnd hook for real-time capture.

### Summary Generation is Optional

The `auto-rename-session.sh` hook generates session titles. If this hook is NOT enabled:
- `summary` field in sessions-index.json will be empty or generic
- LocalPush would need to generate titles (or show sessionId)

**Mitigation:** Encourage users to enable auto-rename hook OR implement title generation in LocalPush.

---

## Conclusion

**Best approach for LocalPush Sessions source:**

1. **Layer 1 (MVP):** Watch `~/.claude/projects/*/sessions-index.json`, parse corresponding JSONL for tokens
2. **Layer 2 (Real-time):** Add SessionEnd hook that writes to `~/.localpush/sessions.jsonl`
3. **Layer 3 (Advanced):** Correlate with `history.jsonl`, extract sub-agent data, file change tracking

**Data quality:**
- Session metadata (start, end, duration, project, branch) = HIGH
- Token counts = HIGH (requires JSONL parsing)
- Session titles = MEDIUM (depends on auto-rename hook)
- Real-time capture = MEDIUM (requires hook) or LOW (file watch polling)

**Integration complexity:**
- Layer 1: Medium (file watching + JSONL parsing)
- Layer 2: Low (hook writes, LocalPush reads simple JSONL)
- Layer 3: High (correlation across multiple data sources)

