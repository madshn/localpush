use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, warn};

/// JXA script that queries Apple Notes via Automation API.
/// Returns metadata only (titles, dates, folders) — no note content.
/// Limited to 50 most recent notes for performance.
const JXA_SCRIPT: &str = r#"
const Notes = Application('Notes');
const allNotes = Notes.notes();
const total = allNotes.length;
const notes = allNotes.slice(0, 50).map(note => ({
    title: note.name(),
    created: note.creationDate().toISOString(),
    modified: note.modificationDate().toISOString(),
    folder: note.container().name()
}));
JSON.stringify({ notes: notes, total: total });
"#;

/// Raw response from the JXA script
#[derive(Debug, Deserialize)]
struct JxaResponse {
    notes: Vec<NoteEntry>,
    total: u64,
}

/// A single note's metadata (no content)
#[derive(Debug, Deserialize)]
struct NoteEntry {
    title: String,
    created: String,
    modified: String,
    folder: String,
}

/// Apple Notes source using JXA (JavaScript for Automation) for metadata queries
/// and NoteStore.sqlite watching for change detection.
pub struct AppleNotesSource {
    watch_db_path: PathBuf,
}

impl AppleNotesSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME").map_err(|_| {
            SourceError::ParseError("Could not determine home directory".to_string())
        })?;

        let watch_db_path = PathBuf::from(home)
            .join("Library/Group Containers/group.com.apple.notes/NoteStore.sqlite");

        Ok(Self { watch_db_path })
    }

    /// Constructor with custom path (for testing)
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            watch_db_path: path.into(),
        }
    }

    /// Execute JXA script via osascript and parse the response
    fn execute_jxa(&self) -> Result<JxaResponse, SourceError> {
        debug!("Executing JXA script for Apple Notes metadata");

        let output = Command::new("osascript")
            .arg("-l")
            .arg("JavaScript")
            .arg("-e")
            .arg(JXA_SCRIPT)
            .output()
            .map_err(|e| SourceError::ParseError(format!("osascript failed to launch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("JXA script failed: {}", stderr);
            return Err(SourceError::ParseError(format!("JXA error: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: JxaResponse = serde_json::from_str(&stdout)
            .map_err(|e| SourceError::ParseError(format!("JXA response parse error: {}", e)))?;

        info!(
            "Loaded Apple Notes: {} total, {} fetched",
            response.total,
            response.notes.len()
        );

        Ok(response)
    }

    /// Filter notes modified within the last 7 days
    fn recent_notes(notes: &[NoteEntry]) -> Vec<&NoteEntry> {
        let cutoff = Utc::now() - Duration::days(7);

        notes
            .iter()
            .filter(|note| {
                DateTime::parse_from_rfc3339(&note.modified)
                    .map(|dt| dt.with_timezone(&Utc) >= cutoff)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Count notes per folder
    fn folder_counts(notes: &[NoteEntry]) -> HashMap<String, u64> {
        let mut counts = HashMap::new();
        for note in notes {
            *counts.entry(note.folder.clone()).or_insert(0) += 1;
        }
        counts
    }
}

impl Source for AppleNotesSource {
    fn id(&self) -> &str {
        "apple-notes"
    }

    fn name(&self) -> &str {
        "Apple Notes"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.watch_db_path.clone())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let data = self.execute_jxa()?;
        let recent = Self::recent_notes(&data.notes);
        let folders = Self::folder_counts(&data.notes);

        let recent_notes: Vec<serde_json::Value> = recent
            .iter()
            .map(|n| {
                serde_json::json!({
                    "title": n.title,
                    "folder": n.folder,
                    "created": n.created,
                    "modified": n.modified,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "source": "apple_notes",
            "timestamp": Utc::now().to_rfc3339(),
            "recent_notes": recent_notes,
            "stats": {
                "total_notes": data.total,
                "recent_count": recent.len(),
                "folders": folders,
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let data = self.execute_jxa()?;
        let recent = Self::recent_notes(&data.notes);

        let summary = format!(
            "{} notes total, {} modified recently",
            data.total,
            recent.len()
        );

        let folder_count = data
            .notes
            .iter()
            .map(|n| n.folder.as_str())
            .collect::<HashSet<_>>()
            .len();

        let mut fields = vec![
            PreviewField {
                label: "Total Notes".to_string(),
                value: data.total.to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Recent (7d)".to_string(),
                value: recent.len().to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Folders".to_string(),
                value: folder_count.to_string(),
                sensitive: false,
            },
        ];

        if let Some(note) = recent.first() {
            fields.push(PreviewField {
                label: "Latest".to_string(),
                value: format!("{} ({})", note.title, note.folder),
                sensitive: true,
            });
        }

        let last_updated = std::fs::metadata(&self.watch_db_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| DateTime::<Utc>::from(t).into());

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary,
            fields,
            last_updated,
        })
    }

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![
            PropertyDef {
                key: "recent_notes".to_string(),
                label: "Recent Notes".to_string(),
                description: "Note titles and folders from the last 7 days".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "folder_stats".to_string(),
                label: "Folder Statistics".to_string(),
                description: "Per-folder note counts".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_trait_impl() {
        let source = AppleNotesSource::new_with_path("/tmp/fake-notestore.sqlite");
        assert_eq!(source.id(), "apple-notes");
        assert_eq!(source.name(), "Apple Notes");
        assert!(source.watch_path().is_some());
    }

    #[test]
    fn test_watch_path_matches_constructor() {
        let path = PathBuf::from("/custom/path/NoteStore.sqlite");
        let source = AppleNotesSource::new_with_path(path.clone());
        assert_eq!(source.watch_path(), Some(path));
    }

    #[test]
    fn test_recent_notes_filters_old() {
        let now = Utc::now();
        let recent_date = (now - Duration::hours(1)).to_rfc3339();
        let old_date = (now - Duration::days(30)).to_rfc3339();

        let notes = vec![
            NoteEntry {
                title: "Recent".to_string(),
                created: recent_date.clone(),
                modified: recent_date,
                folder: "Notes".to_string(),
            },
            NoteEntry {
                title: "Old".to_string(),
                created: old_date.clone(),
                modified: old_date,
                folder: "Notes".to_string(),
            },
        ];

        let recent = AppleNotesSource::recent_notes(&notes);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "Recent");
    }

    #[test]
    fn test_recent_notes_handles_invalid_dates() {
        let notes = vec![NoteEntry {
            title: "Bad Date".to_string(),
            created: "not-a-date".to_string(),
            modified: "not-a-date".to_string(),
            folder: "Notes".to_string(),
        }];

        let recent = AppleNotesSource::recent_notes(&notes);
        assert!(recent.is_empty());
    }

    #[test]
    fn test_folder_counts() {
        let notes = vec![
            NoteEntry {
                title: "A".to_string(),
                created: String::new(),
                modified: String::new(),
                folder: "Work".to_string(),
            },
            NoteEntry {
                title: "B".to_string(),
                created: String::new(),
                modified: String::new(),
                folder: "Work".to_string(),
            },
            NoteEntry {
                title: "C".to_string(),
                created: String::new(),
                modified: String::new(),
                folder: "Personal".to_string(),
            },
        ];

        let counts = AppleNotesSource::folder_counts(&notes);
        assert_eq!(counts.get("Work"), Some(&2));
        assert_eq!(counts.get("Personal"), Some(&1));
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn test_jxa_response_deserialization() {
        let json = r#"{
            "notes": [
                {
                    "title": "Test Note",
                    "created": "2026-01-01T00:00:00.000Z",
                    "modified": "2026-01-15T12:00:00.000Z",
                    "folder": "Notes"
                }
            ],
            "total": 42
        }"#;

        let response: JxaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.total, 42);
        assert_eq!(response.notes.len(), 1);
        assert_eq!(response.notes[0].title, "Test Note");
        assert_eq!(response.notes[0].folder, "Notes");
    }

    #[test]
    fn test_jxa_response_empty_notes() {
        let json = r#"{"notes": [], "total": 0}"#;

        let response: JxaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.total, 0);
        assert!(response.notes.is_empty());
    }
}
