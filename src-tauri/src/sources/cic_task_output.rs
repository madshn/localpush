use super::{PostDeliveryAction, PreviewField, Source, SourceError, SourcePreview};
use crate::config::AppConfig;
use chrono::{DateTime, Utc};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SOURCE_ID: &str = "cic-task-output";
const CLAIMED_PATH_KEY: &str = "source_runtime.cic-task-output.claimed_path";
const EVENT_PATH_PREFIX: &str = "source_runtime.cic-task-output.event_path.";
const DEFAULT_ARCHIVE_DIR_NAME: &str = ".claude-task-archive";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostProcessMode {
    Archive,
    Delete,
}

pub struct CicTaskOutputSource {
    downloads_dir: PathBuf,
    archive_dir: PathBuf,
    post_process: PostProcessMode,
    config: Arc<AppConfig>,
}

impl CicTaskOutputSource {
    pub fn new(config: Arc<AppConfig>) -> Result<Self, SourceError> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| {
                SourceError::ParseError("Could not determine home directory".to_string())
            })?;

        let downloads_dir = PathBuf::from(home).join("Downloads");
        let archive_dir = config
            .get("source.cic-task-output.archive_path")
            .ok()
            .flatten()
            .map(|path| Self::expand_home_path(&path))
            .unwrap_or_else(|| downloads_dir.join(DEFAULT_ARCHIVE_DIR_NAME));
        let post_process = match config
            .get("source.cic-task-output.post_process")
            .ok()
            .flatten()
            .as_deref()
        {
            Some("delete") => PostProcessMode::Delete,
            _ => PostProcessMode::Archive,
        };

        Ok(Self {
            downloads_dir,
            archive_dir,
            post_process,
            config,
        })
    }

    #[cfg(test)]
    pub fn new_with_paths(
        downloads_dir: impl Into<PathBuf>,
        archive_dir: impl Into<PathBuf>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            downloads_dir: downloads_dir.into(),
            archive_dir: archive_dir.into(),
            post_process: PostProcessMode::Archive,
            config,
        }
    }

    fn home_dir() -> Option<PathBuf> {
        env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
    }

    fn expand_home_path(path: &str) -> PathBuf {
        if path == "~" {
            return Self::home_dir().unwrap_or_else(|| PathBuf::from(path));
        }

        if let Some(rest) = path.strip_prefix("~/") {
            if let Some(home) = Self::home_dir() {
                return home.join(rest);
            }
        }

        PathBuf::from(path)
    }

    fn is_task_file_name(name: &str) -> bool {
        name.starts_with("claude-task-")
            && name.ends_with(".json")
            && !name.ends_with("_state.json")
    }

    fn is_task_file_path(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(Self::is_task_file_name)
            .unwrap_or(false)
    }

    fn read_payload(path: &Path) -> Result<serde_json::Value, SourceError> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(SourceError::from)
    }

    fn claimed_path(&self) -> Option<PathBuf> {
        self.config
            .get(CLAIMED_PATH_KEY)
            .ok()
            .flatten()
            .map(PathBuf::from)
    }

    fn set_claimed_path(&self, path: &Path) -> Result<(), SourceError> {
        self.config
            .set(CLAIMED_PATH_KEY, &path.display().to_string())
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }

    fn clear_claimed_path(&self) -> Result<(), SourceError> {
        self.config
            .delete(CLAIMED_PATH_KEY)
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }

    fn event_key(event_id: &str) -> String {
        format!("{EVENT_PATH_PREFIX}{event_id}")
    }

    fn remember_event_path(&self, event_id: &str, path: &Path) -> Result<(), SourceError> {
        self.config
            .set(&Self::event_key(event_id), &path.display().to_string())
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }

    fn path_for_event(&self, event_id: &str) -> Option<PathBuf> {
        self.config
            .get(&Self::event_key(event_id))
            .ok()
            .flatten()
            .map(PathBuf::from)
    }

    fn clear_event_path(&self, event_id: &str) -> Result<(), SourceError> {
        self.config
            .delete(&Self::event_key(event_id))
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }

    fn clear_all_event_paths_for(&self, path: &Path) -> Result<(), SourceError> {
        for (key, value) in self
            .config
            .get_by_prefix(EVENT_PATH_PREFIX)
            .map_err(|e| SourceError::ParseError(e.to_string()))?
        {
            if PathBuf::from(value) == path {
                self.config
                    .delete(&key)
                    .map_err(|e| SourceError::ParseError(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn matching_files(&self) -> Result<Vec<PathBuf>, SourceError> {
        let entries = match fs::read_dir(&self.downloads_dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(SourceError::IoError(err)),
        };

        let mut files = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && Self::is_task_file_path(path))
            .collect::<Vec<_>>();

        files.sort_by(|a, b| {
            let a_name = a
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let b_name = b
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            a_name.cmp(b_name)
        });

        Ok(files)
    }

    fn next_available_file(&self) -> Result<Option<PathBuf>, SourceError> {
        let claimed = self.claimed_path();
        Ok(self
            .matching_files()?
            .into_iter()
            .find(|path| Some(path.clone()) != claimed))
    }

    fn current_or_next_file(&self) -> Result<Option<PathBuf>, SourceError> {
        if let Some(path) = self.claimed_path() {
            if path.exists() {
                return Ok(Some(path));
            }
            self.clear_claimed_path()?;
        }

        self.next_available_file()
    }

    fn archive_destination(&self, source_path: &Path) -> PathBuf {
        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("claude-task-output.json");
        let mut destination = self.archive_dir.join(file_name);

        if destination.exists() {
            let stem = source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("claude-task-output");
            let extension = source_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("json");
            destination = self.archive_dir.join(format!(
                "{stem}-{}.{}",
                Utc::now().timestamp_millis(),
                extension
            ));
        }

        destination
    }

    fn post_process_label(&self) -> &'static str {
        match self.post_process {
            PostProcessMode::Archive => "Archive",
            PostProcessMode::Delete => "Delete",
        }
    }

    fn task_type_for_path(path: &Path) -> Option<String> {
        let file_name = path.file_name()?.to_str()?;
        let slug = file_name
            .strip_prefix("claude-task-")?
            .strip_suffix(".json")?;
        let task_type = slug.split('-').next()?.trim();
        if task_type.is_empty() {
            None
        } else {
            Some(task_type.to_string())
        }
    }
}

impl Source for CicTaskOutputSource {
    fn id(&self) -> &str {
        SOURCE_ID
    }

    fn name(&self) -> &str {
        "CiC Task File Output"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.downloads_dir.clone())
    }

    fn watch_recursive(&self) -> bool {
        true
    }

    fn should_process_event(&self, path: &Path) -> bool {
        Self::is_task_file_path(path)
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let Some(path) = self.current_or_next_file()? else {
            return Err(SourceError::FileNotFound(self.downloads_dir.clone()));
        };

        Self::read_payload(&path)
    }

    fn prepare_for_delivery(&self) -> Result<serde_json::Value, SourceError> {
        let path = if let Some(path) = self.claimed_path() {
            if path.exists() {
                path
            } else {
                self.clear_claimed_path()?;
                self.next_available_file()?
                    .ok_or_else(|| SourceError::FileNotFound(self.downloads_dir.clone()))?
            }
        } else {
            self.next_available_file()?
                .ok_or_else(|| SourceError::FileNotFound(self.downloads_dir.clone()))?
        };

        self.set_claimed_path(&path)?;
        Self::read_payload(&path)
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let files = self.matching_files()?;
        let next_file = self.current_or_next_file()?;
        let last_updated = next_file
            .as_ref()
            .and_then(|path| fs::metadata(path).ok())
            .and_then(|metadata| metadata.modified().ok())
            .map(DateTime::<Utc>::from);

        let summary = if files.is_empty() {
            "No pending claude-task outputs".to_string()
        } else if files.len() == 1 {
            "1 pending claude-task file".to_string()
        } else {
            format!("{} pending claude-task files", files.len())
        };

        let mut fields = vec![
            PreviewField {
                label: "Watch Path".to_string(),
                value: self.downloads_dir.display().to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Pending Files".to_string(),
                value: files.len().to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Post-Process".to_string(),
                value: self.post_process_label().to_string(),
                sensitive: false,
            },
        ];

        if let Some(path) = next_file {
            fields.push(PreviewField {
                label: "Next File".to_string(),
                value: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
                sensitive: false,
            });
        }

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary,
            fields,
            last_updated,
        })
    }

    fn fingerprint_payload(&self, payload: &serde_json::Value) -> serde_json::Value {
        let mut fingerprint = serde_json::json!({ "payload": payload });
        if let Some(path) = self.claimed_path() {
            fingerprint["claimed_path"] = serde_json::Value::String(path.display().to_string());
        }
        fingerprint
    }

    fn on_delivery_queued(
        &self,
        event_id: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), SourceError> {
        if let Some(path) = self.claimed_path() {
            self.remember_event_path(event_id, &path)?;
        }
        Ok(())
    }

    fn rewrite_delivery_headers(
        &self,
        event_id: &str,
        headers: &mut Vec<(String, String)>,
    ) -> Result<(), SourceError> {
        let Some(path) = self.path_for_event(event_id) else {
            return Ok(());
        };
        let Some(task_type) = Self::task_type_for_path(&path) else {
            return Ok(());
        };

        for (name, value) in headers.iter_mut() {
            if name.eq_ignore_ascii_case("X-Metrick-Source") {
                let base = value.trim_end_matches('.');
                if !base.is_empty() {
                    *value = format!("{base}.{task_type}");
                }
            }
        }

        Ok(())
    }

    fn on_delivery_success(
        &self,
        event_id: &str,
        _payload: &serde_json::Value,
    ) -> Result<PostDeliveryAction, SourceError> {
        let Some(path) = self.path_for_event(event_id) else {
            return Ok(PostDeliveryAction::None);
        };

        if path.exists() {
            match self.post_process {
                PostProcessMode::Archive => {
                    fs::create_dir_all(&self.archive_dir)?;
                    let destination = self.archive_destination(&path);
                    fs::rename(&path, destination)?;
                }
                PostProcessMode::Delete => {
                    fs::remove_file(&path)?;
                }
            }
        }

        self.clear_event_path(event_id)?;
        self.clear_all_event_paths_for(&path)?;

        if self.claimed_path().as_ref() == Some(&path) {
            self.clear_claimed_path()?;
        }

        Ok(PostDeliveryAction::FlushNext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_task_file(dir: &Path, name: &str, body: &serde_json::Value) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, serde_json::to_vec(body).unwrap()).unwrap();
        path
    }

    #[test]
    fn ignores_state_files_for_events_and_listing() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join(".archive");
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let source = CicTaskOutputSource::new_with_paths(temp.path(), &archive, config);

        let state_path = write_task_file(
            temp.path(),
            "claude-task-linkedin-harvest-_state.json",
            &serde_json::json!({"cursor": "abc"}),
        );
        let data_path = write_task_file(
            temp.path(),
            "claude-task-linkedin-harvest-2026-03-24T10-00-00Z.json",
            &serde_json::json!({"items": [1, 2]}),
        );

        assert!(!source.should_process_event(&state_path));
        assert!(source.should_process_event(&data_path));
        assert_eq!(source.matching_files().unwrap(), vec![data_path]);
    }

    #[test]
    fn prepare_for_delivery_claims_oldest_file_without_mutating_preview() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join(".archive");
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let source = CicTaskOutputSource::new_with_paths(temp.path(), &archive, config);

        let first = write_task_file(
            temp.path(),
            "claude-task-alpha-2026-03-24T08-00-00Z.json",
            &serde_json::json!({"slug": "alpha"}),
        );
        write_task_file(
            temp.path(),
            "claude-task-beta-2026-03-24T09-00-00Z.json",
            &serde_json::json!({"slug": "beta"}),
        );

        let preview_payload = source.parse().unwrap();
        assert_eq!(preview_payload["slug"], "alpha");
        assert!(
            source.claimed_path().is_none(),
            "preview parse should not claim"
        );

        let delivery_payload = source.prepare_for_delivery().unwrap();
        assert_eq!(delivery_payload["slug"], "alpha");
        assert_eq!(source.claimed_path(), Some(first));
    }

    #[test]
    fn rewrite_delivery_headers_appends_task_type_from_filename() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join(".archive");
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let source = CicTaskOutputSource::new_with_paths(temp.path(), &archive, config);

        let payload = serde_json::json!({"items": [1, 2]});
        write_task_file(
            temp.path(),
            "claude-task-linkedin-harvest-2026-03-24T10-00-00Z.json",
            &payload,
        );

        source.prepare_for_delivery().unwrap();
        source.on_delivery_queued("evt-linkedin", &payload).unwrap();

        let mut headers = vec![(
            "X-Metrick-Source".to_string(),
            "localpush.cic-task-output".to_string(),
        )];
        source
            .rewrite_delivery_headers("evt-linkedin", &mut headers)
            .unwrap();

        assert_eq!(
            headers[0],
            (
                "X-Metrick-Source".to_string(),
                "localpush.cic-task-output.linkedin".to_string(),
            )
        );
    }

    #[test]
    fn success_archives_claimed_file_and_clears_duplicate_event_mappings() {
        let temp = TempDir::new().unwrap();
        let archive = temp.path().join(".archive");
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let source = CicTaskOutputSource::new_with_paths(temp.path(), &archive, config);

        let first = write_task_file(
            temp.path(),
            "claude-task-alpha-2026-03-24T08-00-00Z.json",
            &serde_json::json!({"slug": "alpha"}),
        );
        let payload = source.prepare_for_delivery().unwrap();

        source.on_delivery_queued("evt-1", &payload).unwrap();
        source.on_delivery_queued("evt-2", &payload).unwrap();

        let action = source.on_delivery_success("evt-1", &payload).unwrap();
        assert_eq!(action, PostDeliveryAction::FlushNext);
        assert!(
            !first.exists(),
            "successful delivery should post-process the file"
        );
        assert!(archive.exists(), "archive directory should be created");
        assert!(source.path_for_event("evt-1").is_none());
        assert!(source.path_for_event("evt-2").is_none());

        let second = write_task_file(
            temp.path(),
            "claude-task-beta-2026-03-24T09-00-00Z.json",
            &serde_json::json!({"slug": "beta"}),
        );
        let second_payload = source.prepare_for_delivery().unwrap();
        source.on_delivery_queued("evt-3", &second_payload).unwrap();
        source.on_delivery_success("evt-2", &payload).unwrap();
        assert!(
            second.exists(),
            "stale success should not touch a newer file"
        );
    }
}
