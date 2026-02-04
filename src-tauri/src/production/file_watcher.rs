//! FSEvents file watcher implementation

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, Debouncer, FileIdMap};
use std::time::Duration;

use crate::traits::{FileWatcher, FileWatcherError, FileEvent, FileEventKind};

pub struct FsEventsWatcher {
    debouncer: Arc<Mutex<Debouncer<RecommendedWatcher, FileIdMap>>>,
    watched_paths: Arc<Mutex<Vec<PathBuf>>>,
    event_handler: Arc<Mutex<Option<Arc<dyn Fn(FileEvent) + Send + Sync>>>>,
}

impl FsEventsWatcher {
    pub fn new() -> Result<Self, FileWatcherError> {
        let (tx, rx) = std::sync::mpsc::channel();
        let event_handler = Arc::new(Mutex::new(None));
        let event_handler_clone = Arc::clone(&event_handler);

        // Spawn event handler thread
        std::thread::spawn(move || {
            for result in rx {
                match result {
                    Ok(events) => {
                        for event in events {
                            tracing::debug!("File event: {:?}", event);
                            // Forward to handler if set
                            if let Some(handler) = event_handler_clone.lock().unwrap().as_ref() {
                                // Convert notify event paths to FileEvent
                                for path in &event.paths {
                                    let file_event = FileEvent {
                                        path: path.clone(),
                                        kind: FileEventKind::Modified, // Simplified for MVP
                                        timestamp: chrono::Utc::now(),
                                    };
                                    handler(file_event);
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            tracing::error!("Watch error: {:?}", error);
                        }
                    }
                }
            }
        });

        let debouncer = new_debouncer(
            Duration::from_millis(300),
            None,
            tx,
        ).map_err(|e| FileWatcherError::WatchError(e.to_string()))?;

        Ok(Self {
            debouncer: Arc::new(Mutex::new(debouncer)),
            watched_paths: Arc::new(Mutex::new(Vec::new())),
            event_handler,
        })
    }
}

impl FileWatcher for FsEventsWatcher {
    fn watch(&self, path: PathBuf) -> Result<(), FileWatcherError> {
        if !path.exists() {
            return Err(FileWatcherError::PathNotFound(path));
        }

        let mut debouncer = self.debouncer.lock().unwrap();
        debouncer.watcher()
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| FileWatcherError::WatchError(e.to_string()))?;

        self.watched_paths.lock().unwrap().push(path.clone());
        tracing::info!("Watching path: {:?}", path);

        Ok(())
    }

    fn unwatch(&self, path: PathBuf) -> Result<(), FileWatcherError> {
        let mut debouncer = self.debouncer.lock().unwrap();
        debouncer.watcher()
            .unwatch(&path)
            .map_err(|e| FileWatcherError::WatchError(e.to_string()))?;

        self.watched_paths.lock().unwrap().retain(|p| p != &path);
        tracing::info!("Unwatched path: {:?}", path);

        Ok(())
    }

    fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().unwrap().clone()
    }

    fn set_event_handler(&self, handler: Arc<dyn Fn(FileEvent) + Send + Sync>) {
        *self.event_handler.lock().unwrap() = Some(handler);
        tracing::debug!("File event handler set");
    }
}
