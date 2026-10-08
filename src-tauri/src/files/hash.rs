use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

/// SHA-256 of a file's contents as lowercase hex.
///
/// Used to tell whether a file changed since it was last indexed. Reads in
/// chunks so large files don't have to fit in memory.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_sha256_values() {
        let dir = tempfile::TempDir::new().unwrap();
        let empty = dir.path().join("empty.txt");
        let abc = dir.path().join("abc.txt");
        std::fs::write(&empty, "").unwrap();
        std::fs::write(&abc, "abc").unwrap();
        // Standard SHA-256 test vectors.
        assert_eq!(
            sha256_file(&empty).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_file(&abc).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_changes_when_content_changes() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("note.txt");
        std::fs::write(&path, "version one").unwrap();
        let first = sha256_file(&path).unwrap();
        std::fs::write(&path, "version two").unwrap();
        assert_ne!(first, sha256_file(&path).unwrap());
    }
}
