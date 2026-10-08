//! Background folder scans.
//!
//! A scan runs on its own thread so the UI stays responsive. It finds the
//! supported files in a location, fingerprints (hashes) new or changed ones,
//! saves them to the database, and removes files that no longer exist.
//! Progress is kept in memory and pushed to the UI as `scan-progress` events.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::database::{self, files::ScannedFile, Database};
use crate::files::scan::{discover, Cancelled, DiscoveredFile};
use crate::files::{hash::sha256_file, now_millis, unix_millis};

/// Name of the event the UI listens to. Documented in docs/API.md.
pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";

/// How often progress events may be sent, so big folders don't flood the UI.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Files at least this big take noticeable time to read, so we always tell
/// the UI when we start one rather than waiting for the next throttled update.
const LARGE_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// Files are saved in batches: one transaction per batch keeps writes fast
/// while letting the Library show results as the scan goes.
const SAVE_BATCH_SIZE: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanState {
    /// Looking through folders for supported files.
    Discovering,
    /// Fingerprinting new or changed files and saving them.
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
    /// Files checked so far, including unchanged ones and ones that failed.
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
    cancel: Arc<AtomicBool>,
}

/// In-memory progress of running and finished scans, keyed by location id.
#[derive(Default)]
pub struct ScanStore {
    scans: Mutex<HashMap<i64, LocationScan>>,
}

impl ScanStore {
    fn scans(&self) -> std::sync::MutexGuard<'_, HashMap<i64, LocationScan>> {
        // A panic while holding the lock leaves plain data behind; keep going.
        self.scans
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn status(&self, location_id: i64) -> Option<ScanStatus> {
        self.scans()
            .get(&location_id)
            .map(|scan| scan.status.clone())
    }

    /// Stops any running scan for the location and forgets its progress.
    pub fn forget(&self, location_id: i64) {
        if let Some(scan) = self.scans().remove(&location_id) {
            scan.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Update the status, but only if `cancel` still belongs to the current
    /// scan (a newer scan or a removal may have replaced it).
    fn update(
        &self,
        location_id: i64,
        cancel: &Arc<AtomicBool>,
        f: impl FnOnce(&mut ScanStatus),
    ) -> Option<ScanStatus> {
        let mut scans = self.scans();
        let scan = scans.get_mut(&location_id)?;
        if !Arc::ptr_eq(&scan.cancel, cancel) || cancel.load(Ordering::Relaxed) {
            return None;
        }
        f(&mut scan.status);
        Some(scan.status.clone())
    }
}

/// Starts (or restarts) a background scan of `root` for `location_id`.
pub fn start_scan(app: &AppHandle, location_id: i64, root: PathBuf) {
    let cancel = Arc::new(AtomicBool::new(false));
    let status = ScanStatus {
        location_id: location_id.to_string(),
        state: ScanState::Discovering,
        files_found: 0,
        files_processed: 0,
        files_failed: 0,
        unreadable: 0,
        current_file: None,
        error: None,
    };
    let previous = app.state::<ScanStore>().scans().insert(
        location_id,
        LocationScan {
            status: status.clone(),
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
        .spawn(move || run_scan(&app, location_id, &root, &cancel));
    if spawned.is_err() {
        eprintln!("recall: could not start scan thread");
    }
}

/// Why a scan stopped before finishing.
enum ScanStop {
    Cancelled,
    Database(rusqlite::Error),
}

impl From<Cancelled> for ScanStop {
    fn from(_: Cancelled) -> Self {
        Self::Cancelled
    }
}

impl From<rusqlite::Error> for ScanStop {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err)
    }
}

fn run_scan(app: &AppHandle, location_id: i64, root: &Path, cancel: &Arc<AtomicBool>) {
    let store = app.state::<ScanStore>();
    let fail = |message: &str| {
        let status = store.update(location_id, cancel, |status| {
            status.state = ScanState::Failed;
            status.current_file = None;
            status.error = Some(message.to_string());
        });
        if let Some(status) = status {
            let _ = app.emit(SCAN_PROGRESS_EVENT, status);
        }
    };

    if !root.is_dir() {
        fail("This folder no longer exists or can't be opened. It may have been moved, renamed or disconnected.");
        return;
    }

    match scan_location(app, location_id, root, cancel) {
        Ok(()) | Err(ScanStop::Cancelled) => {}
        Err(ScanStop::Database(err)) => {
            // A removed location makes in-flight writes fail; that's expected.
            if !cancel.load(Ordering::Relaxed) {
                eprintln!("recall: scan of location {location_id} failed: {err}");
                fail("Recall couldn't save what it found. Try Rescan; if this keeps happening, restart Recall.");
            }
        }
    }
}

fn scan_location(
    app: &AppHandle,
    location_id: i64,
    root: &Path,
    cancel: &Arc<AtomicBool>,
) -> Result<(), ScanStop> {
    let store = app.state::<ScanStore>();
    let mut conn = app.state::<Database>().connect()?;

    let mut last_emit = Instant::now();
    let mut report = |force: bool, f: &mut dyn FnMut(&mut ScanStatus)| {
        let status = store.update(location_id, cancel, |status| f(status));
        if force || last_emit.elapsed() >= PROGRESS_INTERVAL {
            last_emit = Instant::now();
            if let Some(status) = status {
                let _ = app.emit(SCAN_PROGRESS_EVENT, status);
            }
        }
    };

    // Step 1: find supported files.
    let discovery = discover(root, cancel, |found| {
        report(false, &mut |status| status.files_found = found);
    })?;
    let total = discovery.files.len();
    report(true, &mut |status| {
        status.files_found = total;
        status.unreadable = discovery.unreadable;
        status.state = ScanState::Hashing;
    });

    // Step 2: fingerprint new or changed files and save them in batches.
    let known = database::files::known_files(&conn, location_id)?;
    let mut seen = HashSet::with_capacity(total);
    let mut batch: Vec<CheckedFile> = Vec::with_capacity(SAVE_BATCH_SIZE);
    let mut failed = 0;
    let scanned_at = now_millis();

    for (index, file) in discovery.files.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(ScanStop::Cancelled);
        }
        let modified_at = file.modified_at.and_then(unix_millis);
        seen.insert(file.path.clone());
        if known
            .get(&file.path)
            .is_some_and(|k| k.looks_unchanged(file.size_bytes, modified_at))
        {
            continue;
        }

        let name = file
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        report(file.size_bytes >= LARGE_FILE_BYTES, &mut |status| {
            status.files_processed = index;
            status.files_failed = failed;
            status.current_file = name.clone();
        });

        let checked = CheckedFile::read(file);
        if checked.error.is_some() {
            failed += 1;
        }
        batch.push(checked);
        if batch.len() >= SAVE_BATCH_SIZE {
            save_batch(&mut conn, location_id, scanned_at, &mut batch)?;
        }
    }
    save_batch(&mut conn, location_id, scanned_at, &mut batch)?;

    // Step 3: forget files that were deleted or moved away since last time.
    let tx = conn.transaction()?;
    for (path, known_file) in &known {
        if !seen.contains(path) {
            database::files::delete(&tx, known_file.id)?;
        }
    }
    tx.commit()?;

    report(true, &mut |status| {
        status.files_processed = total;
        status.files_failed = failed;
        status.current_file = None;
        status.state = ScanState::Done;
    });
    Ok(())
}

/// A discovered file after we tried to fingerprint it.
struct CheckedFile {
    file: DiscoveredFile,
    content_hash: Option<String>,
    error: Option<String>,
}

impl CheckedFile {
    fn read(file: DiscoveredFile) -> Self {
        match sha256_file(&file.path) {
            Ok(hash) => Self {
                file,
                content_hash: Some(hash),
                error: None,
            },
            Err(err) => Self {
                file,
                content_hash: None,
                error: Some(read_error_message(&err)),
            },
        }
    }
}

fn save_batch(
    conn: &mut rusqlite::Connection,
    location_id: i64,
    scanned_at: i64,
    batch: &mut Vec<CheckedFile>,
) -> rusqlite::Result<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for checked in batch.drain(..) {
        database::files::upsert(
            &tx,
            &ScannedFile {
                location_id,
                path: &checked.file.path,
                kind: checked.file.kind,
                size_bytes: checked.file.size_bytes,
                created_at: checked.file.created_at.and_then(unix_millis),
                modified_at: checked.file.modified_at.and_then(unix_millis),
                scanned_at,
                content_hash: checked.content_hash.as_deref(),
                error: checked.error.as_deref(),
            },
        )?;
    }
    tx.commit()
}

/// Short, user-facing reason a file couldn't be read.
fn read_error_message(err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => "Recall isn't allowed to read this file.".into(),
        std::io::ErrorKind::NotFound => "The file was moved or deleted during the scan.".into(),
        _ => "This file couldn't be read.".into(),
    }
}
