use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc;
use std::time::Duration;
use std::path::PathBuf;

pub struct FsWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl FsWatcher {
    pub fn new<F>(paths: Vec<PathBuf>, callback: F) -> Result<Self, String>
    where
        F: Fn(Vec<PathBuf>) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = tx.send(event.paths);
                    }
                    _ => {}
                }
            }
        }).map_err(|e| e.to_string())?;

        for path in &paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)
                    .map_err(|e| e.to_string())?;
            }
        }

        // Debounce thread: collect events over 500ms then fire callback
        std::thread::spawn(move || {
            loop {
                let mut collected = Vec::new();
                match rx.recv() {
                    Ok(paths) => collected.extend(paths),
                    Err(_) => break,
                }
                while let Ok(paths) = rx.recv_timeout(Duration::from_millis(500)) {
                    collected.extend(paths);
                }
                collected.sort();
                collected.dedup();
                if !collected.is_empty() {
                    callback(collected);
                }
            }
        });

        Ok(Self { _watcher: watcher })
    }
}
