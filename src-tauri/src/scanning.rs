//! Running indexing in the background for the desktop app.
//!
//! The pipeline itself is in `indexing::pipeline` (no Tauri). This module
//! runs it on its own thread so the UI stays responsive, keeps the latest
//! progress per location, lets a newer scan or a removal cancel an older
//! one, and pushes progress to the UI as `scan-progress` events.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::database::Database;
use crate::embeddings::EmbeddingModel;
use crate::indexing::pipeline::{index_location, IndexStop, Models, ScanState};
use crate::ocr::OcrModel;

pub use crate::indexing::pipeline::ScanStatus;

/// Name of the event the UI listens to. Documented in docs/API.md.
pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";

/// How often progress events may be sent, so big folders don't flood the UI.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

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

    /// Store `status`, but only if `cancel` still belongs to the current scan
    /// (a newer scan or a removal may have replaced it). Returns whether it
    /// was stored.
    fn set(&self, location_id: i64, cancel: &Arc<AtomicBool>, status: &ScanStatus) -> bool {
        let mut scans = self.scans();
        match scans.get_mut(&location_id) {
            Some(scan) if Arc::ptr_eq(&scan.cancel, cancel) && !cancel.load(Ordering::Relaxed) => {
                scan.status = status.clone();
                true
            }
            _ => false,
        }
    }
}

/// Starts (or restarts) a background scan of `root` for `location_id`.
pub fn start_scan(app: &AppHandle, location_id: i64, root: PathBuf) {
    let cancel = Arc::new(AtomicBool::new(false));
    let status = ScanStatus::new(location_id);
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

fn run_scan(app: &AppHandle, location_id: i64, root: &std::path::Path, cancel: &Arc<AtomicBool>) {
    let store = app.state::<ScanStore>();
    let publish = |status: &ScanStatus| {
        if store.set(location_id, cancel, status) {
            let _ = app.emit(SCAN_PROGRESS_EVENT, status.clone());
        }
    };

    let ocr = app.state::<OcrModel>().engine.clone();
    let embedder = app.state::<EmbeddingModel>().embedder.clone();
    let models = Models {
        ocr: ocr.as_deref(),
        embedder: embedder.as_deref(),
    };

    let mut last_emit = Instant::now();
    let result = app
        .state::<Database>()
        .connect()
        .map_err(IndexStop::Database)
        .and_then(|mut conn| {
            index_location(
                &mut conn,
                location_id,
                root,
                models,
                cancel,
                |status, important| {
                    if important || last_emit.elapsed() >= PROGRESS_INTERVAL {
                        last_emit = Instant::now();
                        publish(status);
                    } else {
                        store.set(location_id, cancel, status);
                    }
                },
            )
        });

    let message = match result {
        Ok(_) | Err(IndexStop::Cancelled) => return,
        Err(IndexStop::FolderMissing) => {
            "This folder no longer exists or can't be opened. It may have been moved, renamed or disconnected."
        }
        Err(IndexStop::Model(err)) => {
            eprintln!("recall: embedding model failed for location {location_id}: {err}");
            "Recall couldn't run its local AI model on this folder. Keyword search still works."
        }
        Err(IndexStop::Database(err)) => {
            // A removed location makes in-flight writes fail; that's expected.
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            eprintln!("recall: scan of location {location_id} failed: {err}");
            "Recall couldn't save what it found. Try Rescan; if this keeps happening, restart Recall."
        }
    };
    let mut status = store
        .status(location_id)
        .unwrap_or_else(|| ScanStatus::new(location_id));
    status.state = ScanState::Failed;
    status.current_file = None;
    status.error = Some(message.into());
    publish(&status);
}
