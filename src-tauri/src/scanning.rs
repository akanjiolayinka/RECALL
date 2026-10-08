//! Background folder scans.
//!
//! A scan runs on its own thread so the UI stays responsive. It finds the
//! supported files in a location, then fingerprints (hashes) each one.
//! Progress is stored here and pushed to the UI as `scan-progress` events.
//!
//! Results are currently held in memory. They move into SQLite in Milestone 4.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::files::scan::{discover, Cancelled};
use crate::files::{hash::sha256_file, FileRecord};

/// Name of the event the UI listens to. Documented in docs/API.md.
pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";

/// How often progress events may be sent, so big folders don't flood the UI.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Files at least this big take noticeable time to read, so we always tell
/// the UI when we start one rather than waiting for the next throttled update.
const LARGE_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanState {
    /// Looking through folders for supported files.
    Discovering,
    /// Reading each file to compute its fingerprint.
    Hashing,
    Done,
    Failed,
}

/// Progress of one location's scan. Sent to the UI as-is, so it doubles as
/// the API DTO (mirrors `ScanStatus` in src/lib/api/types.ts).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub location_id: String,
    pub state: ScanState,
    pub files_found: usize,
    /// Files fingerprinted so far, including ones that failed.
    pub files_processed: usize,
    /// Files that were found but couldn't be read.
    pub files_failed: usize,
    /// Folders or files skipped because we weren't allowed to read them.
    pub unreadable: usize,
    /// Name (not full path) of the file being processed.
    pub current_file: Option<String>,
    pub error: Option<String>,
}

struct LocationScan {
    status: ScanStatus,
    files: Vec<FileRecord>,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct ScanStore {
    scans: Mutex<HashMap<String, LocationScan>>,
    next_file_id: AtomicU64,
}

impl ScanStore {
    fn scans(&self) -> std::sync::MutexGuard<'_, HashMap<String, LocationScan>> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.scans
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn status(&self, location_id: &str) -> Option<ScanStatus> {
        self.scans()
            .get(location_id)
            .map(|scan| scan.status.clone())
    }

    /// Calls `f` with every file found so far, across all locations.
    pub fn with_files<T>(&self, f: impl FnOnce(&mut dyn Iterator<Item = &FileRecord>) -> T) -> T {
        let scans = self.scans();
        let mut iter = scans.values().flat_map(|scan| scan.files.iter());
        f(&mut iter)
    }

    /// Stops any running scan for the location and forgets its results.
    pub fn forget(&self, location_id: &str) {
        if let Some(scan) = self.scans().remove(location_id) {
            scan.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Update the status, but only if `cancel` still belongs to the current
    /// scan (a newer scan or a removal may have replaced it).
    fn update(
        &self,
        location_id: &str,
        cancel: &Arc<AtomicBool>,
        f: impl FnOnce(&mut LocationScan),
    ) -> Option<ScanStatus> {
        let mut scans = self.scans();
        let scan = scans.get_mut(location_id)?;
        if !Arc::ptr_eq(&scan.cancel, cancel) || cancel.load(Ordering::Relaxed) {
            return None;
        }
        f(scan);
        Some(scan.status.clone())
    }
}

/// Starts (or restarts) a background scan of `root` for `location_id`.
pub fn start_scan(app: &AppHandle, location_id: String, root: PathBuf) {
    let cancel = Arc::new(AtomicBool::new(false));
    let status = ScanStatus {
        location_id: location_id.clone(),
        state: ScanState::Discovering,
        files_found: 0,
        files_processed: 0,
        files_failed: 0,
        unreadable: 0,
        current_file: None,
        error: None,
    };
    let previous = app.state::<ScanStore>().scans().insert(
        location_id.clone(),
        LocationScan {
            status: status.clone(),
            files: Vec::new(),
            cancel: cancel.clone(),
        },
    );
    if let Some(previous) = previous {
        previous.cancel.store(true, Ordering::Relaxed);
    }
    let _ = app.emit(SCAN_PROGRESS_EVENT, status);

    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name(format!("scan-{location_id}"))
        .spawn(move || run_scan(&app, &location_id, &root, &cancel));
    if spawned.is_err() {
        eprintln!("recall: could not start scan thread");
    }
}

fn run_scan(app: &AppHandle, location_id: &str, root: &Path, cancel: &Arc<AtomicBool>) {
    let store = app.state::<ScanStore>();
    let emit = |status: Option<ScanStatus>| {
        if let Some(status) = status {
            let _ = app.emit(SCAN_PROGRESS_EVENT, status);
        }
    };
    let mut last_emit = Instant::now();
    let mut emit_throttled = |status: Option<ScanStatus>, force: bool| {
        if force || last_emit.elapsed() >= PROGRESS_INTERVAL {
            last_emit = Instant::now();
            emit(status);
        }
    };

    if !root.is_dir() {
        emit(store.update(location_id, cancel, |scan| {
            scan.status.state = ScanState::Failed;
            scan.status.error = Some(
                "This folder no longer exists or can't be opened. It may have been moved, renamed or disconnected.".into(),
            );
        }));
        return;
    }

    // Step 1: find supported files.
    let discovery = discover(root, cancel, |found| {
        let status = store.update(location_id, cancel, |scan| scan.status.files_found = found);
        emit_throttled(status, false);
    });
    let discovery = match discovery {
        Ok(discovery) => discovery,
        Err(Cancelled) => return,
    };
    emit(store.update(location_id, cancel, |scan| {
        scan.status.files_found = discovery.files.len();
        scan.status.unreadable = discovery.unreadable;
        scan.status.state = ScanState::Hashing;
    }));

    // Step 2: fingerprint each file.
    let mut records = Vec::with_capacity(discovery.files.len());
    let mut failed = 0;
    for (index, file) in discovery.files.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        let status = store.update(location_id, cancel, |scan| {
            scan.status.files_processed = index;
            scan.status.files_failed = failed;
            scan.status.current_file = name;
        });
        emit_throttled(status, file.size_bytes >= LARGE_FILE_BYTES);
        let (content_hash, error) = match sha256_file(&file.path) {
            Ok(hash) => (Some(hash), None),
            Err(err) => {
                failed += 1;
                (None, Some(read_error_message(&err)))
            }
        };
        records.push(FileRecord {
            id: format!(
                "file-{}",
                store.next_file_id.fetch_add(1, Ordering::Relaxed) + 1
            ),
            location_id: location_id.to_string(),
            path: file.path,
            kind: file.kind,
            size_bytes: file.size_bytes,
            modified_at: file.modified_at,
            created_at: file.created_at,
            content_hash,
            error,
        });
    }

    emit(store.update(location_id, cancel, move |scan| {
        scan.status.files_processed = records.len();
        scan.files = records;
        scan.status.files_failed = failed;
        scan.status.current_file = None;
        scan.status.state = ScanState::Done;
    }));
}

/// Short, user-facing reason a file couldn't be read.
fn read_error_message(err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => "Recall isn't allowed to read this file.".into(),
        std::io::ErrorKind::NotFound => "The file was moved or deleted during the scan.".into(),
        _ => "This file couldn't be read.".into(),
    }
}
