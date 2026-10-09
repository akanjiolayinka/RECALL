//! Watching library folders for changes, so new and edited files are indexed
//! without pressing Rescan.
//!
//! Uses the operating system's change notifications (via `notify`). Only
//! changes count: reading a file (which Recall itself does while indexing)
//! is ignored, otherwise every scan would trigger another. Changes are
//! collected until the folder has been quiet for `DEBOUNCE`, then each
//! affected folder gets a normal incremental scan, which only re-processes
//! what actually changed.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::Mutex;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use crate::database::{self, Database};
use crate::locations::Location;
use crate::scanning::start_scan;

/// Wait for this long without new changes before rescanning.
const DEBOUNCE: Duration = Duration::from_secs(2);

/// Watches every library folder. `None` if the OS watcher couldn't start;
/// Recall then still works, but changes are only picked up on Rescan or restart.
pub struct FolderWatcher(Mutex<Option<RecommendedWatcher>>);

impl FolderWatcher {
    pub fn start(app: &AppHandle, folders: &[PathBuf]) -> Self {
        let (sender, receiver) = channel::<PathBuf>();
        let watcher =
            notify::recommended_watcher(move |result: notify::Result<Event>| match result {
                Ok(event) if is_change(&event.kind) => {
                    for path in event.paths {
                        let _ = sender.send(path);
                    }
                }
                Ok(_) => {}
                Err(err) => eprintln!("recall: file watcher error: {err}"),
            });
        let watcher = match watcher {
            Ok(watcher) => {
                let app = app.clone();
                let spawned = std::thread::Builder::new()
                    .name("folder-watcher".into())
                    .spawn(move || rescan_when_quiet(&app, &receiver));
                if spawned.is_err() {
                    eprintln!("recall: could not start the file watcher thread");
                }
                Some(watcher)
            }
            Err(err) => {
                eprintln!("recall: could not start the file watcher: {err}");
                None
            }
        };
        let watcher = Self(Mutex::new(watcher));
        for folder in folders {
            watcher.watch(folder);
        }
        watcher
    }

    pub fn watch(&self, folder: &Path) {
        if let Some(watcher) = self.0.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
            if let Err(err) = watcher.watch(folder, RecursiveMode::Recursive) {
                eprintln!("recall: could not watch {}: {err}", folder.display());
            }
        }
    }

    pub fn unwatch(&self, folder: &Path) {
        if let Some(watcher) = self.0.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
            // Fails harmlessly if the folder wasn't being watched.
            let _ = watcher.unwatch(folder);
        }
    }
}

/// Creations, edits, renames and deletions count; reads (opening or closing a
/// file without writing) don't.
fn is_change(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

/// Collect changed paths until none arrive for `DEBOUNCE`, then rescan.
/// Ends when the watcher (and with it the sender) is dropped.
fn rescan_when_quiet(app: &AppHandle, changes: &Receiver<PathBuf>) {
    let mut pending: HashSet<PathBuf> = HashSet::new();
    loop {
        match changes.recv_timeout(DEBOUNCE) {
            Ok(path) => {
                pending.insert(path);
            }
            Err(RecvTimeoutError::Timeout) => {
                if !pending.is_empty() {
                    rescan_changed(app, pending.drain());
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn rescan_changed(app: &AppHandle, changed: impl Iterator<Item = PathBuf>) {
    let locations = match app
        .state::<Database>()
        .connect()
        .and_then(|conn| database::locations::list(&conn))
    {
        Ok(locations) => locations,
        Err(err) => {
            eprintln!("recall: file watcher could not read library folders: {err}");
            return;
        }
    };
    for location in affected_locations(changed, &locations) {
        start_scan(app, location.id, location.path.clone());
    }
}

/// The locations containing any of `changed` paths, each once.
fn affected_locations(
    changed: impl Iterator<Item = PathBuf>,
    locations: &[Location],
) -> Vec<&Location> {
    let ids: BTreeSet<i64> = changed
        .filter_map(|path| {
            locations
                .iter()
                .find(|l| path.starts_with(&l.path))
                .map(|l| l.id)
        })
        .collect();
    locations.iter().filter(|l| ids.contains(&l.id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_a_file_is_not_a_change() {
        use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind};
        assert!(!is_change(&EventKind::Access(AccessKind::Open(
            AccessMode::Read
        ))));
        assert!(!is_change(&EventKind::Access(AccessKind::Close(
            AccessMode::Read
        ))));
        assert!(is_change(&EventKind::Create(CreateKind::File)));
        assert!(is_change(&EventKind::Modify(ModifyKind::Any)));
        assert!(is_change(&EventKind::Remove(RemoveKind::File)));
    }

    #[test]
    fn maps_changed_paths_to_their_locations_once() {
        let locations = vec![
            Location {
                id: 1,
                path: PathBuf::from("/home/me/Documents"),
            },
            Location {
                id: 2,
                path: PathBuf::from("/home/me/Pictures"),
            },
            Location {
                id: 3,
                path: PathBuf::from("/home/me/Documents-old"),
            },
        ];
        let changed = [
            "/home/me/Documents/a.txt",
            "/home/me/Documents/sub/b.pdf",
            "/home/me/Pictures/c.png",
            "/tmp/elsewhere.txt",
        ]
        .map(PathBuf::from);
        let ids: Vec<i64> = affected_locations(changed.into_iter(), &locations)
            .iter()
            .map(|l| l.id)
            .collect();
        assert_eq!(ids, vec![1, 2], "Documents-old is not inside Documents");
    }
}
