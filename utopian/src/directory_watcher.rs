use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

pub struct DirectoryWatcher {
    _directory_watcher: notify::ReadDirectoryChangesWatcher,
    watcher_rx: mpsc::Receiver<notify::DebouncedEvent>,
}

impl DirectoryWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let (watcher_tx, watcher_rx) = mpsc::channel();
        let mut directory_watcher: RecommendedWatcher =
            Watcher::new(watcher_tx, Duration::from_millis(100)).unwrap();
        directory_watcher
            .watch(path, RecursiveMode::Recursive)
            .unwrap();

        DirectoryWatcher {
            _directory_watcher: directory_watcher,
            watcher_rx,
        }
    }

    pub fn check_if_modification(&self) -> bool {
        if let Ok(_event) = self.watcher_rx.try_recv() {
            match self.watcher_rx.recv() {
                Ok(event) => {
                    if let notify::DebouncedEvent::Write(..) = event {
                        return true;
                    }
                }
                Err(e) => println!("recv Err {:?}", e),
            }
        }

        false
    }
}
