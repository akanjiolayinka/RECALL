//! The indexing pipeline for one location, independent of Tauri.
//!
//! 1. Find supported files.
//! 2. Fingerprint new or changed files (unchanged size + modification time
//!    means unchanged) and save them; changed contents go back to "pending".
//! 3. Forget files that no longer exist.
//! 4. Read (extract + chunk) pending documents; images only with OCR.
//! 5. Embed passages without an embedding from the current model.
//!
//! Each step only touches what changed, so re-running on an unchanged folder
//! is cheap. Progress is reported through `on_progress`; `scanning` turns it
//! into UI events.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use serde::Serialize;

use crate::database::{self, files::ScannedFile};
use crate::embeddings::{EmbedError, Embedder};
use crate::extract;
use crate::files::scan::{discover, Cancelled, DiscoveredFile};
use crate::files::{hash::sha256_file, now_millis, unix_millis};
use crate::indexing::chunk::chunk_pages;
use crate::indexing::embed::{embed_location, EmbedStop};
use crate::ocr::Ocr;

/// Files are saved in batches: one transaction per batch keeps writes fast
/// while letting the Library show results as the scan goes.
const SAVE_BATCH_SIZE: usize = 200;

/// Files at least this big take noticeable time to read, so progress is
/// reported when one starts rather than at the next throttled update.
const LARGE_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanState {
    /// Looking through folders for supported files.
    Discovering,
    /// Fingerprinting new or changed files and saving them.
    Hashing,
    /// Extracting and chunking the text of new or changed documents.
    Reading,
    /// Computing embeddings for passages that don't have one (only when a
    /// local embedding model is installed).
    Embedding,
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
    /// Files that were new or changed since the last scan.
    pub files_changed: usize,
    /// Files that were found but couldn't be read.
    pub files_failed: usize,
    /// Folders or files skipped because we weren't allowed to read them.
    pub unreadable: usize,
    /// Documents whose text needs reading in this scan.
    pub files_to_read: usize,
    /// Documents read so far, including ones whose text couldn't be read.
    pub files_read: usize,
    /// Documents whose text couldn't be read (damaged, scanned, too large...).
    pub read_failed: usize,
    /// Passages needing an embedding in this scan (0 without a model).
    pub passages_to_embed: usize,
    pub passages_embedded: usize,
    /// Name (not full path) of the file being processed.
    pub current_file: Option<String>,
    /// User-facing reason, when `state` is `Failed`.
    pub error: Option<String>,
}

impl ScanStatus {
    pub fn new(location_id: i64) -> Self {
        Self {
            location_id: location_id.to_string(),
            state: ScanState::Discovering,
            files_found: 0,
            files_processed: 0,
            files_changed: 0,
            files_failed: 0,
            unreadable: 0,
            files_to_read: 0,
            files_read: 0,
            read_failed: 0,
            passages_to_embed: 0,
            passages_embedded: 0,
            current_file: None,
            error: None,
        }
    }
}

/// The local AI models available to the pipeline.
#[derive(Clone, Copy, Default)]
pub struct Models<'a> {
    pub ocr: Option<&'a Ocr>,
    pub embedder: Option<&'a dyn Embedder>,
}

/// Why indexing stopped before finishing.
#[derive(Debug)]
pub enum IndexStop {
    Cancelled,
    /// The location's folder doesn't exist or isn't a folder any more.
    FolderMissing,
    Database(rusqlite::Error),
    Model(EmbedError),
}

impl From<Cancelled> for IndexStop {
    fn from(_: Cancelled) -> Self {
        Self::Cancelled
    }
}

impl From<rusqlite::Error> for IndexStop {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err)
    }
}

impl From<EmbedStop> for IndexStop {
    fn from(stop: EmbedStop) -> Self {
        match stop {
            EmbedStop::Cancelled => Self::Cancelled,
            EmbedStop::Database(err) => Self::Database(err),
            EmbedStop::Model(err) => Self::Model(err),
        }
    }
}

/// Index (or re-index) the folder `root` of location `location_id`.
///
/// `on_progress(status, important)` is called after every change; `important`
/// marks updates worth showing immediately (step changes, large files).
pub fn index_location(
    conn: &mut Connection,
    location_id: i64,
    root: &Path,
    models: Models<'_>,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(&ScanStatus, bool),
) -> Result<ScanStatus, IndexStop> {
    let mut status = ScanStatus::new(location_id);
    if !root.is_dir() {
        return Err(IndexStop::FolderMissing);
    }
    let check_cancel = || {
        if cancel.load(Ordering::Relaxed) {
            Err(IndexStop::Cancelled)
        } else {
            Ok(())
        }
    };

    // Step 1: find supported files.
    let discovery = discover(root, cancel, |found| {
        status.files_found = found;
        on_progress(&status, false);
    })?;
    status.files_found = discovery.files.len();
    status.unreadable = discovery.unreadable;
    status.state = ScanState::Hashing;
    on_progress(&status, true);

    // Step 2: fingerprint new or changed files and save them in batches.
    let known = database::files::known_files(conn, location_id)?;
    let mut seen = HashSet::with_capacity(discovery.files.len());
    let mut batch: Vec<CheckedFile> = Vec::with_capacity(SAVE_BATCH_SIZE);
    let scanned_at = now_millis();

    for (index, file) in discovery.files.into_iter().enumerate() {
        check_cancel()?;
        seen.insert(file.path.clone());
        let modified_at = file.modified_at.and_then(unix_millis);
        if known
            .get(&file.path)
            .is_some_and(|k| k.looks_unchanged(file.size_bytes, modified_at))
        {
            continue;
        }
        status.files_processed = index;
        status.current_file = file_name(&file.path);
        on_progress(&status, file.size_bytes >= LARGE_FILE_BYTES);

        let checked = CheckedFile::read(file);
        status.files_changed += 1;
        if checked.error.is_some() {
            status.files_failed += 1;
        }
        batch.push(checked);
        if batch.len() >= SAVE_BATCH_SIZE {
            save_batch(conn, location_id, scanned_at, &mut batch)?;
        }
    }
    save_batch(conn, location_id, scanned_at, &mut batch)?;
    status.files_processed = status.files_found;
    status.current_file = None;

    // Step 3: forget files that were deleted or moved away since last time.
    let tx = conn.transaction()?;
    for (path, known_file) in &known {
        if !seen.contains(path) {
            database::files::delete(&tx, known_file.id)?;
        }
    }
    tx.commit()?;

    // Step 4: read the text of new or changed documents.
    let pending = database::documents::pending_files(conn, location_id, models.ocr.is_some())?;
    status.files_to_read = pending.len();
    status.state = ScanState::Reading;
    on_progress(&status, true);
    for file in pending {
        check_cancel()?;
        status.current_file = file_name(&file.path);
        on_progress(&status, false);
        if !read_document(conn, &file, models.ocr)? {
            status.read_failed += 1;
        }
        status.files_read += 1;
    }
    status.current_file = None;

    // Step 5: embed passages that don't have an embedding from this model.
    if let Some(embedder) = models.embedder {
        status.state = ScanState::Embedding;
        on_progress(&status, true);
        embed_location(conn, embedder, location_id, cancel, |done, total| {
            status.passages_embedded = done;
            status.passages_to_embed = total;
            on_progress(&status, false);
        })?;
    }

    status.state = ScanState::Done;
    on_progress(&status, true);
    Ok(status)
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned())
}

/// Extract, chunk and save one document. Returns `false` if its text
/// couldn't be read (the reason is saved on the file for the UI).
fn read_document(
    conn: &mut Connection,
    file: &database::documents::PendingFile,
    ocr: Option<&Ocr>,
) -> rusqlite::Result<bool> {
    match extract::extract(&file.path, file.kind, ocr) {
        Ok(extracted) => {
            let (full_text, chunks) = chunk_pages(&extracted.pages);
            let tx = conn.transaction()?;
            database::documents::save_document(
                &tx,
                file.id,
                &extracted,
                &full_text,
                &chunks,
                now_millis(),
            )?;
            tx.commit()?;
            Ok(true)
        }
        Err(err) => {
            // Log the technical reason (never the file's contents).
            eprintln!("recall: could not read file {}: {err}", file.id);
            database::documents::mark_unreadable(conn, file.id, &err.user_message(file.kind))?;
            Ok(false)
        }
    }
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
    conn: &mut Connection,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{locations, test_connection};
    use crate::embeddings::fake::FakeEmbedder;
    use crate::search;
    use std::fs;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    struct Library {
        dir: TempDir,
        conn: Connection,
        location: i64,
    }

    impl Library {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            fs::write(
                dir.path().join("budget.txt"),
                "The garden budget is NGN 2,500,000.",
            )
            .unwrap();
            fs::write(
                dir.path().join("lease.md"),
                "# Lease\nGive sixty days notice.",
            )
            .unwrap();
            fs::write(dir.path().join("photo.png"), b"not really an image").unwrap();
            let conn = test_connection();
            let location = locations::insert(&conn, dir.path(), 0).unwrap().id;
            Self {
                dir,
                conn,
                location,
            }
        }

        fn index(&mut self, models: Models<'_>) -> ScanStatus {
            let root = self.dir.path().to_path_buf();
            index_location(
                &mut self.conn,
                self.location,
                &root,
                models,
                &AtomicBool::new(false),
                |_, _| {},
            )
            .unwrap()
        }

        /// Rewrite a file and move its modification time forward, as an
        /// editor saving it later would.
        fn edit(&self, name: &str, contents: &str) {
            let path = self.dir.path().join(name);
            fs::write(&path, contents).unwrap();
            touch(&path);
        }

        fn found(&self, query: &str) -> Vec<String> {
            search::search(&self.conn, None, query, 10)
                .unwrap()
                .iter()
                .map(|r| r.path.file_name().unwrap().to_string_lossy().into_owned())
                .collect()
        }
    }

    fn touch(path: &Path) {
        let later = SystemTime::now() + Duration::from_secs(60);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(later)
            .unwrap();
    }

    #[test]
    fn first_run_reads_every_document() {
        let mut lib = Library::new();
        let status = lib.index(Models::default());
        assert_eq!(status.state, ScanState::Done);
        assert_eq!(status.files_found, 3);
        assert_eq!(status.files_changed, 3);
        // The image waits for OCR, so only two documents are read.
        assert_eq!(status.files_to_read, 2);
        assert_eq!(lib.found("sixty days notice"), vec!["lease.md"]);
    }

    #[test]
    fn unchanged_folder_does_no_work() {
        let mut lib = Library::new();
        lib.index(Models::default());
        let again = lib.index(Models::default());
        assert_eq!(again.files_found, 3);
        assert_eq!(again.files_changed, 0);
        assert_eq!(again.files_to_read, 0);
    }

    #[test]
    fn changed_files_are_re_read_and_old_text_disappears() {
        let mut lib = Library::new();
        lib.index(Models::default());
        lib.edit("budget.txt", "The garden budget was cut to NGN 1,000,000.");
        let status = lib.index(Models::default());
        assert_eq!((status.files_changed, status.files_to_read), (1, 1));
        assert_eq!(lib.found("1,000,000"), vec!["budget.txt"]);
        assert!(lib.found("2,500,000").is_empty());
    }

    #[test]
    fn a_touched_but_identical_file_is_not_re_read() {
        let mut lib = Library::new();
        lib.index(Models::default());
        touch(&lib.dir.path().join("lease.md"));
        let status = lib.index(Models::default());
        // Re-fingerprinted because its date changed, but the contents match.
        assert_eq!((status.files_changed, status.files_to_read), (1, 0));
        assert_eq!(lib.found("sixty"), vec!["lease.md"]);
    }

    #[test]
    fn added_and_deleted_files_are_picked_up() {
        let mut lib = Library::new();
        lib.index(Models::default());
        fs::remove_file(lib.dir.path().join("lease.md")).unwrap();
        fs::write(
            lib.dir.path().join("notes.txt"),
            "Order the rainwater tank.",
        )
        .unwrap();
        let status = lib.index(Models::default());
        assert_eq!(
            (
                status.files_found,
                status.files_changed,
                status.files_to_read
            ),
            (3, 1, 1)
        );
        assert!(lib.found("sixty").is_empty());
        assert_eq!(lib.found("rainwater"), vec!["notes.txt"]);
    }

    #[test]
    fn only_new_passages_are_embedded() {
        let mut lib = Library::new();
        let embedder = FakeEmbedder::new("fake");
        let models = Models {
            ocr: None,
            embedder: Some(&embedder),
        };
        let first = lib.index(models);
        assert_eq!((first.passages_to_embed, first.passages_embedded), (2, 2));
        assert_eq!(lib.index(models).passages_to_embed, 0);
        lib.edit("budget.txt", "A new budget.");
        assert_eq!(lib.index(models).passages_to_embed, 1);
    }

    #[test]
    fn missing_folders_and_cancellation_stop_cleanly() {
        let mut lib = Library::new();
        let missing = lib.dir.path().join("gone");
        let result = index_location(
            &mut lib.conn,
            lib.location,
            &missing,
            Models::default(),
            &AtomicBool::new(false),
            |_, _| {},
        );
        assert!(matches!(result, Err(IndexStop::FolderMissing)));

        let root = lib.dir.path().to_path_buf();
        let result = index_location(
            &mut lib.conn,
            lib.location,
            &root,
            Models::default(),
            &AtomicBool::new(true),
            |_, _| {},
        );
        assert!(matches!(result, Err(IndexStop::Cancelled)));
    }
}
