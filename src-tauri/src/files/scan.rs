use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

use super::FileKind;

/// Folder names never worth indexing: app/system internals, not personal files.
const SKIPPED_FOLDERS: &[&str] = &["node_modules", "$RECYCLE.BIN", "System Volume Information"];

/// A supported file found on disk, before any content is read.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub kind: FileKind,
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub created_at: Option<SystemTime>,
}

#[derive(Debug, Default)]
pub struct Discovery {
    pub files: Vec<DiscoveredFile>,
    /// Folders or files we weren't allowed to read (e.g. permission denied).
    pub unreadable: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Cancelled;

/// Hidden files/folders (starting with "."), Office lock files ("~$report.docx")
/// and known system folders are skipped.
fn is_skipped(entry: &DirEntry) -> bool {
    // Never skip the folder the user explicitly chose, even if it's hidden.
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.')
        || name.starts_with("~$")
        || (entry.file_type().is_dir() && SKIPPED_FOLDERS.contains(&name.as_ref()))
}

/// Walk `root` and collect every supported file.
///
/// Symbolic links are not followed, which avoids loops and leaving the chosen
/// folder. `on_progress` receives the running count of files found.
/// Stops early with `Cancelled` when `cancel` is set.
pub fn discover(
    root: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(usize),
) -> Result<Discovery, Cancelled> {
    let mut discovery = Discovery::default();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_skipped(entry));

    for entry in walker {
        if cancel.load(Ordering::Relaxed) {
            return Err(Cancelled);
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                discovery.unreadable += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(kind) = FileKind::from_path(entry.path()) else {
            continue;
        };
        let Ok(metadata) = entry.metadata() else {
            discovery.unreadable += 1;
            continue;
        };
        discovery.files.push(DiscoveredFile {
            path: entry.into_path(),
            kind,
            size_bytes: metadata.len(),
            modified_at: metadata.modified().ok(),
            // Not available on every platform/filesystem.
            created_at: metadata.created().ok(),
        });
        on_progress(discovery.files.len());
    }

    Ok(discovery)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn names(discovery: &Discovery) -> Vec<String> {
        let mut names: Vec<_> = discovery
            .files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn finds_supported_files_recursively_and_skips_the_rest() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("work/2024")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("budget.pdf"), "x").unwrap();
        fs::write(root.join("work/notes.md"), "x").unwrap();
        fs::write(root.join("work/2024/receipt.JPG"), "x").unwrap();
        fs::write(root.join("work/song.mp3"), "x").unwrap();
        fs::write(root.join("~$draft.docx"), "x").unwrap();
        fs::write(root.join(".secret.txt"), "x").unwrap();
        fs::write(root.join(".git/HEAD.txt"), "x").unwrap();
        fs::write(root.join("node_modules/pkg/readme.md"), "x").unwrap();

        let discovery = discover(root, &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(
            names(&discovery),
            vec!["budget.pdf", "notes.md", "receipt.JPG"]
        );
    }

    #[test]
    fn records_size_and_kind() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        let discovery = discover(dir.path(), &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(discovery.files[0].size_bytes, 5);
        assert_eq!(discovery.files[0].kind, FileKind::Text);
        assert!(discovery.files[0].modified_at.is_some());
    }

    #[test]
    fn indexes_a_hidden_folder_when_the_user_chose_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let hidden = dir.path().join(".notes");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("idea.md"), "x").unwrap();
        let discovery = discover(&hidden, &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(names(&discovery), vec!["idea.md"]);
    }

    #[test]
    fn stops_when_cancelled() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        assert_eq!(
            discover(dir.path(), &AtomicBool::new(true), |_| {}).unwrap_err(),
            Cancelled
        );
    }
}
