# Apple Notes Source Plugin Research

**Date:** 2026-02-06
**Purpose:** Implementation plan for LocalPush Apple Notes source plugin
**Scope:** Metadata-only capture (no note contents)

---

## Executive Summary

Apple Notes data can be accessed via two approaches:

1. **Direct SQLite access** — `/Users/[user]/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite`
2. **AppleScript/JXA bridge** — `osascript` subprocess calling Notes application API

**Recommendation:** **Hybrid approach** — AppleScript for metadata extraction, with SQLite file watching for change detection.

**Key constraint:** Note contents are stored as gzipped protocol buffers in SQLite, making direct parsing complex. AppleScript provides clean metadata access without reverse-engineering proprietary formats.

---

## Database Location and Structure

### File Paths

| Type | Path |
|------|------|
| **Database** | `~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite` |
| **Attachments** | `~/Library/Group Containers/group.com.apple.notes/Media/[UUID]/` |

### Key Tables

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `ZICCLOUDSYNCINGOBJECT` | Note metadata | Title, creation date, modification date, folder |
| `ZICNOTEDATA` | Note contents | `ZDATA` (gzipped protobuf blob) |
| `Z_*` tables | Relationships | Cloud sync, folders, accounts |

### Content Format

- **Storage format:** Gzipped protocol buffers in `ZICNOTEDATA.ZDATA` column
- **Not human-readable** — Requires reverse-engineering or Apple's internal APIs
- **Metadata is accessible** — Timestamps, titles, folder names available in clear text

**References:**
- [Yogesh Khatri's forensic blog: Reading Notes database on macOS](http://www.swiftforensics.com/2018/02/reading-notes-database-on-macos.html)
- [GitHub - dogsheep/apple-notes-to-sqlite](https://github.com/dogsheep/apple-notes-to-sqlite)
- [Notes on Notes.app - Simon Willison](https://simonwillison.net/2021/Dec/9/notes-on-notesapp/)

---

## Access Methods Comparison

### 1. Direct SQLite Access

**Pros:**
- Direct file system access (no subprocess overhead)
- Can read raw timestamps and metadata
- File watch can detect changes immediately

**Cons:**
- Database may be locked while Notes.app is running
- Content parsing requires gzip + protobuf decoding
- Schema is proprietary and may change across macOS versions
- Requires Full Disk Access permission (TCC)

**TCC Requirements:**
- Application must request Full Disk Access
- User must manually approve in System Settings > Privacy & Security > Full Disk Access
- Protected by System Integrity Protection (SIP)

**References:**
- [macOS Catalina & Osquery - TCC Permissions](https://www.kolide.com/blog/macos-catalina-osquery)
- [Understanding TCC](https://www.angelystor.com/posts/macos_tcc/)
- [A deep dive into macOS TCC.db](https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive)

### 2. AppleScript/JXA Bridge

**Pros:**
- Clean API for metadata: `Application("Notes").notes()`
- No content parsing needed — Apple handles format
- No TCC Full Disk Access required (uses Notes.app's permissions)
- Stable API across macOS versions

**Cons:**
- Subprocess overhead (spawn `osascript` per query)
- Requires Notes.app to be available (may not work if Notes is disabled)
- Limited to what AppleScript dictionary exposes
- Performance degradation with large note collections

**Available Metadata via AppleScript:**
```applescript
tell application "Notes"
    repeat with n in notes
        name of n           -- Title
        creation date of n  -- Created timestamp
        modification date of n -- Modified timestamp
        container of n      -- Folder name
        body of n           -- HTML content (we won't capture)
    end repeat
end tell
```

**References:**
- [AppleScript: The Notes Application](https://www.macosxautomation.com/applescript/notes/index.html)
- [macOS JavaScript for Automation (JXA) Notes](https://www.galvanist.com/posts/2020-03-28-jxa_notes/)
- [MacScripter thread: Check Notes content](https://www.macscripter.net/t/check-notes-content-solved/75419)

### 3. Hybrid Approach (Recommended)

**Strategy:**
1. **Watch** `NoteStore.sqlite` file for modifications (file system events)
2. **Query** metadata via AppleScript when changes detected
3. **Parse** AppleScript output to extract metadata only

**Why this works:**
- File watching is lightweight and immediate
- AppleScript query only runs on change (not polling)
- No complex protobuf parsing required
- No Full Disk Access permission needed
- Stable across macOS versions

---

## Rust Implementation Plan

### Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
notify = "6.0"  # File system watching
tokio = { version = "1.0", features = ["process"] }  # Async subprocess
```

**Rust Crates for AppleScript:**
- [`osascript`](https://docs.rs/osascript/) — Execute AppleScript/JXA via `osascript` command
- [`osakit`](https://docs.rs/osakit/) — Direct OSAKit framework bindings (more complex, requires ObjC)

**Recommendation:** Use `osascript` crate for simplicity. It wraps `std::process::Command` with AppleScript helpers.

**References:**
- [osascript - Rust](https://docs.rs/osascript/)
- [GitHub - mitsuhiko/rust-osascript](https://github.com/mitsuhiko/rust-osascript)
- [osakit - Rust](https://docs.rs/osakit/)

### Source Trait Implementation

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::process::Command;

pub struct AppleNotesSource;

impl Source for AppleNotesSource {
    fn id(&self) -> &str {
        "apple-notes"
    }

    fn name(&self) -> &str {
        "Apple Notes"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        // Watch NoteStore.sqlite for changes
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(format!(
            "{}/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite",
            home
        )))
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        // Execute AppleScript to get metadata
        let script = r#"
            tell application "Notes"
                set notesList to {}
                repeat with n in notes
                    set noteData to {
                        title: name of n,
                        created: creation date of n as string,
                        modified: modification date of n as string,
                        folder: name of container of n
                    }
                    set end of notesList to noteData
                end repeat
                return notesList
            end tell
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| SourceError::ParseError(format!("osascript failed: {}", e)))?;

        if !output.status.success() {
            return Err(SourceError::ParseError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Parse AppleScript output (list format)
        let raw_output = String::from_utf8_lossy(&output.stdout);
        let notes = parse_applescript_list(&raw_output)?;

        // Calculate aggregate stats
        let payload = json!({
            "total_notes": notes.len(),
            "recent_notes": notes.iter()
                .take(5)
                .map(|n| {
                    json!({
                        "title": n.title,
                        "folder": n.folder,
                        "modified": n.modified,
                        "word_count_estimate": estimate_word_count(&n.title)
                    })
                })
                .collect::<Vec<_>>(),
            "folders": count_by_folder(&notes),
            "most_recent_activity": notes.first().map(|n| n.modified.clone())
        });

        Ok(payload)
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let data = self.parse()?;
        let total = data["total_notes"].as_u64().unwrap_or(0);

        Ok(SourcePreview {
            title: format!("{} notes", total),
            description: format!(
                "Recent: {}",
                data["recent_notes"]
                    .as_array()
                    .map(|arr| arr.iter()
                        .map(|n| n["title"].as_str().unwrap_or("Untitled"))
                        .collect::<Vec<_>>()
                        .join(", ")
                    )
                    .unwrap_or_default()
            ),
            icon: "📝".to_string(),
        })
    }
}

#[derive(Deserialize, Serialize)]
struct NoteMetadata {
    title: String,
    created: String,
    modified: String,
    folder: String,
}

fn parse_applescript_list(output: &str) -> Result<Vec<NoteMetadata>, SourceError> {
    // AppleScript returns lists in format:
    // {title:"Note 1", created:"...", modified:"...", folder:"Notes"}, ...
    // Parse this into structured data
    // (Implementation depends on osascript output format)
    todo!("Parse AppleScript list output")
}

fn estimate_word_count(title: &str) -> usize {
    title.split_whitespace().count()
}

fn count_by_folder(notes: &[NoteMetadata]) -> serde_json::Value {
    let mut folder_counts: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();

    for note in notes {
        *folder_counts.entry(&note.folder).or_insert(0) += 1;
    }

    json!(folder_counts)
}
```

### Alternative: JXA (JavaScript for Automation)

Instead of AppleScript, use JXA for structured JSON output:

```javascript
// notes.jxa
const Notes = Application('Notes');
Notes.includeStandardAdditions = true;

const notes = Notes.notes().map(note => ({
    title: note.name(),
    created: note.creationDate().toISOString(),
    modified: note.modificationDate().toISOString(),
    folder: note.container().name()
}));

JSON.stringify(notes);
```

Execute with:
```rust
Command::new("osascript")
    .arg("-l")
    .arg("JavaScript")
    .arg("-e")
    .arg(include_str!("notes.jxa"))
    .output()
```

**Advantage:** Native JSON output, no custom parser needed.

---

## Sample JSON Payload

**Metadata-only capture (no note contents):**

```json
{
  "total_notes": 247,
  "recent_notes": [
    {
      "title": "LocalPush feature ideas",
      "folder": "Work",
      "modified": "2026-02-06T14:32:01Z",
      "word_count_estimate": 4
    },
    {
      "title": "Weekly review checklist",
      "folder": "Personal",
      "modified": "2026-02-06T09:15:22Z",
      "word_count_estimate": 3
    },
    {
      "title": "Meeting notes - Feb 5",
      "folder": "Work",
      "modified": "2026-02-05T16:45:33Z",
      "word_count_estimate": 5
    }
  ],
  "folders": {
    "Work": 89,
    "Personal": 124,
    "Archive": 34
  },
  "most_recent_activity": "2026-02-06T14:32:01Z"
}
```

**What is NOT captured:**
- Note body/content
- Attachments
- Formatting
- Internal links
- Tags (if using folders-as-tags pattern)

**Why metadata-only:**
- Respects user privacy (titles are less sensitive than content)
- Smaller payload size
- Faster parsing
- Still useful for activity tracking

---

## Privacy & Risk Assessment

### Privacy Concerns (Even Metadata)

**What metadata reveals:**
- **Note titles** — Often descriptive enough to infer content ("Meeting with [person]", "Ideas for [project]")
- **Folder names** — Organizational structure reveals priorities
- **Timestamps** — Activity patterns (when you write, how often)
- **Volume** — How much you're capturing in Notes

**Sensitivity tiers:**

| Data | Sensitivity | Risk |
|------|-------------|------|
| Note content | HIGH | Direct exposure of thoughts, passwords, personal info |
| Note titles | MEDIUM | Infers topics, projects, relationships |
| Folder names | MEDIUM | Reveals organizational structure |
| Timestamps | LOW | Activity patterns (but combined with titles = MEDIUM) |
| Total count | LOW | General activity level |

### Transparency Requirements

**LocalPush MUST clearly communicate:**

1. **What is collected:**
   - Note titles (verbatim)
   - Folder names
   - Creation/modification timestamps
   - Note count

2. **What is NOT collected:**
   - Note contents/body
   - Attachments
   - Formatting
   - Internal links

3. **Where data goes:**
   - User-configured webhook URL
   - User is responsible for endpoint security
   - No LocalPush cloud storage

4. **User control:**
   - Enable/disable source
   - Preview before sending
   - Clear visual indicator when active

### Recommended UI Transparency

**Source card "Coming Soon" state:**

```
📝 Apple Notes (Coming Soon)

Track your note-taking activity:
• Note titles and folder organization
• Creation and modification timestamps
• Recent activity summary

⚠️ Privacy: Note TITLES will be sent to your webhook.
   Titles often reveal what you're working on.
   Note CONTENTS are never accessed.

[Enable] [Learn More]
```

**Active state warning:**

```
📝 Apple Notes [●]
247 notes · Last activity: 2 min ago

⚠️ Note titles are being sent to your webhook
```

### Risk Mitigation

**For users:**
- Don't use sensitive titles (e.g., "Password for [service]")
- Use generic titles if privacy is critical
- Review payload preview before enabling

**For LocalPush:**
- Show payload preview prominently
- Require explicit consent per source
- Visual indicator when source is active
- Easy disable mechanism

---

## Known Limitations & Gotchas

### 1. AppleScript Performance

**Issue:** Iterating all notes via AppleScript is slow for large collections (1000+ notes).

**Impact:**
- First parse may take 5-10 seconds
- Blocks UI thread if synchronous

**Mitigation:**
- Run parse in background thread
- Cache results, only refresh on file change
- Consider pagination or limiting to recent N notes

### 2. Notes.app Availability

**Issue:** AppleScript requires Notes.app to be available on system.

**Impact:**
- Fails if Notes is disabled or removed
- May not work on minimal macOS installations

**Mitigation:**
- Check for Notes.app presence before enabling source
- Graceful error message if unavailable

### 3. iCloud Sync Timing

**Issue:** NoteStore.sqlite may update before cloud sync completes.

**Impact:**
- File watcher triggers before note is fully written
- Metadata may be incomplete

**Mitigation:**
- Debounce file watch events (wait 1-2 seconds after last change)
- Validate parsed data before sending

### 4. Database Locking

**Issue:** SQLite database may be locked by Notes.app during writes.

**Impact:**
- Direct SQLite reads may fail with SQLITE_BUSY

**Mitigation:**
- Use AppleScript approach (doesn't touch database directly)
- If using direct SQLite: retry with exponential backoff

### 5. macOS Version Differences

**Issue:** AppleScript dictionary may vary across macOS versions.

**Impact:**
- Scripts may break on older/newer macOS

**Mitigation:**
- Test on multiple macOS versions (10.15+, 11+, 12+, 13+)
- Use conservative API subset
- Check macOS version and adapt if needed

### 6. Permissions Dialog

**Issue:** First run may prompt user to allow osascript to control Notes.

**Impact:**
- User must approve in System Settings > Privacy & Security > Automation
- Silent failure until approved

**Mitigation:**
- Detect permission denial (check stderr output)
- Show clear instructions to user on how to grant permission

---

## Implementation Phases

### Phase 1: Basic AppleScript Integration (MVP)

**Goal:** Prove metadata extraction works

**Tasks:**
1. Add `osascript` dependency
2. Implement `AppleNotesSource` with JXA script
3. Parse JSON output from JXA
4. Return sample metadata payload
5. Test on small note collection (<100 notes)

**Deliverable:** Working metadata extraction, no file watching yet

### Phase 2: File Watching

**Goal:** Detect changes automatically

**Tasks:**
1. Add `notify` dependency for file system events
2. Watch `NoteStore.sqlite` for modifications
3. Debounce events (1-2 second delay)
4. Trigger parse on change
5. Test with rapid note edits

**Deliverable:** Automatic updates on note changes

### Phase 3: Performance Optimization

**Goal:** Handle large note collections (1000+ notes)

**Tasks:**
1. Benchmark AppleScript performance
2. Implement caching (store last known state)
3. Consider incremental updates (diff notes)
4. Add pagination or recent-only mode
5. Test with 1000+ note collection

**Deliverable:** Sub-second updates for typical use

### Phase 4: UI Polish

**Goal:** Transparent, user-friendly source

**Tasks:**
1. Privacy warning in UI
2. Payload preview before enable
3. Permission troubleshooting guide
4. Active indicator icon
5. Error messages for common failures

**Deliverable:** Production-ready user experience

---

## Recommended Approach

### For LocalPush MVP

**Use:** Hybrid approach (file watch + AppleScript)

**Rationale:**
- No Full Disk Access permission required
- Stable across macOS versions
- Clean metadata extraction
- Respects user privacy (metadata-only)

**Implementation:**
1. Watch `NoteStore.sqlite` for changes
2. Execute JXA script to get metadata
3. Parse JSON output
4. Send to webhook

**Estimated effort:** 2-3 hours for basic implementation

### Future Enhancements

**If needed later:**
- Direct SQLite read for faster access (requires Full Disk Access)
- Protobuf parsing for content extraction (privacy risk)
- Folder filtering (only watch specific folders)
- Note tagging integration (if Notes supports)

---

## References

### Apple Notes Database Structure
- [Yogesh Khatri's forensic blog: Reading Notes database on macOS](http://www.swiftforensics.com/2018/02/reading-notes-database-on-macos.html)
- [GitHub - ChrLipp/notes-import: Parses Apple notes SQLite databases](https://github.com/ChrLipp/notes-import)
- [GitHub - dogsheep/apple-notes-to-sqlite](https://github.com/dogsheep/apple-notes-to-sqlite)
- [Notes on Notes.app - Simon Willison](https://simonwillison.net/2021/Dec/9/notes-on-notesapp/)
- [Where are Notes saved on Mac?](https://cleanmymac.com/blog/where-are-notes-stored-on-mac)

### AppleScript/JXA
- [AppleScript: The Notes Application](https://www.macosxautomation.com/applescript/notes/index.html)
- [macOS JavaScript for Automation (JXA) Notes](https://www.galvanist.com/posts/2020-03-28-jxa_notes/)
- [MacScripter thread: Check Notes content](https://www.macscripter.net/t/check-notes-content-solved/75419)
- [JavaScript for Automation (JXA) Resources · GitHub](https://gist.github.com/JMichaelTX/d29adaa18088572ce6d4)

### TCC Permissions and Security
- [macOS Catalina & Osquery](https://www.kolide.com/blog/macos-catalina-osquery)
- [Bypassing macOS TCC User Privacy Protections](https://www.sentinelone.com/labs/bypassing-macos-tcc-user-privacy-protections-by-accident-and-design/)
- [Understanding TCC](https://www.angelystor.com/posts/macos_tcc/)
- [Full Transparency: Controlling Apple's TCC | Huntress](https://www.huntress.com/blog/full-transparency-controlling-apples-tcc)
- [A deep dive into macOS TCC.db](https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive)

### Rust Libraries
- [osascript - Rust](https://docs.rs/osascript/)
- [GitHub - mitsuhiko/rust-osascript](https://github.com/mitsuhiko/rust-osascript)
- [osakit - Rust](https://docs.rs/osakit/)
- [GitHub - mdevils/rust-osakit: Mac OS OSAKit adapted for Rust](https://github.com/mdevils/rust-osakit)

---

## Next Steps

1. **Prototype JXA script** — Test metadata extraction manually
2. **Add osascript dependency** — Update LocalPush Cargo.toml
3. **Implement AppleNotesSource** — Follow hybrid approach pattern
4. **Test permissions flow** — Document user approval process
5. **Design privacy UI** — Mockup warning and preview screens
6. **Performance test** — Benchmark with large note collection

