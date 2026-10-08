//! Watching library folders for changes, so new and edited files are indexed
//! without pressing Rescan.
//!
//! Uses the operating system's change notifications (via `notify`). Bursts of
//! events are grouped (`DEBOUNCE`), then each affected folder gets a normal
//! incremental scan, which only re-processes what actually changed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Manager};

use crate::database::{self, Database};
use crate::locations::Location;
use crate::scanning::start_scan;

/// Wait for this long without new changes before rescanning.
const DEBOUNCE: Duration = Duration::from_secs(2);

/// Watches every library folder. `None` if the OS watcher couldn't start;
/// Recall then still works, but changes are only picked up on Rescan or restart.
pub struct FolderWatcher(Mutex<Option<Debouncer<RecommendedWatcher>>>);

impl FolderWatcher {
    pub fn start(app: &AppHandle, folders: &[PathBuf]) -> Self {
        let handler_app = app.clone();
        let debouncer = new_debouncer(DEBOUNCE, move |result: DebounceEventResult| match result {
            Ok(events) => rescan_changed(&handler_app, events.into_iter().map(|e| e.path)),
            Err(err) => eprintln!("recall: file watcher error: {err}"),
        });
        let watcher = Self(Mutex::new(match debouncer {
            Ok(debouncer) => Some(debouncer),
            Err(err) => {
                eprintln!("recall: could not start the file watcher: {err}");
                None
            }
        }));
        for folder in folders {
            watcher.watch(folder);
        }
        watcher
    }

    pub fn watch(&self, folder: &Path) {
        if let Some(debouncer) = self.0.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
            if let Err(err) = debouncer.watcher().watch(folder, RecursiveMode::Recursive) {
                eprintln!("recall: could not watch {}: {err}", folder.display());
            }
        }
    }

    pub fn unwatch(&self, folder: &Path) {
        if let Some(debouncer) = self.0.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
            // Fails harmlessly if the folder wasn't being watched.
            let _ = debouncer.watcher().unwatch(folder);
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
