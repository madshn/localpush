//! FSEvents file watcher implementation

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, Debouncer, FileIdMap};
use std::time::Duration;

use crate::traits::{FileWatcher, FileWatcherError};

pub struct FsEventsWatcher {
    debouncer: Arc<Mutex<Debouncer<RecommendedWatcher, FileIdMap>>>,
    watched_paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl FsEventsWatcher {
    pub fn new() -> Result<Self, FileWatcherError> {
        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn event handler thread
        std::thread::spawn(move || {
            for result in rx {
                match result {
                    Ok(events) => {
                        for event in events {
                            tracing::debug!("File event: {:?}", event);
                            // TODO: Send to event processor
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
}
