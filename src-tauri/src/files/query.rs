use super::{FileKind, FileRecord};

/// Filters for listing files. All fields are optional.
#[derive(Debug, Default, Clone)]
pub struct FileQuery {
    pub location_id: Option<String>,
    pub kind: Option<FileKind>,
    /// Case-insensitive match against the file name.
    pub name_contains: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

/// Default and maximum page size, so the UI never receives an unbounded list.
pub const DEFAULT_LIMIT: usize = 100;
pub const MAX_LIMIT: usize = 500;

/// Apply `query` to `records`. Returns one page of matches (most recently
/// modified first) and the total number of matches.
pub fn apply<'a>(
    records: impl Iterator<Item = &'a FileRecord>,
    query: &FileQuery,
) -> (Vec<&'a FileRecord>, usize) {
    let needle = query
        .name_contains
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_lowercase);

    let mut matches: Vec<&FileRecord> = records
        .filter(|r| {
            query
                .location_id
                .as_ref()
                .is_none_or(|id| &r.location_id == id)
        })
        .filter(|r| query.kind.is_none_or(|kind| r.kind == kind))
        .filter(|r| {
            needle
                .as_ref()
                .is_none_or(|needle| r.file_name().to_lowercase().contains(needle))
        })
        .collect();

    matches.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.path.cmp(&b.path))
    });

    let total = matches.len();
    let limit = query.limit.clamp(1, MAX_LIMIT);
    let page = matches.into_iter().skip(query.offset).take(limit).collect();
    (page, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn record(id: &str, location: &str, path: &str, kind: FileKind, age_secs: u64) -> FileRecord {
        FileRecord {
            id: id.into(),
            location_id: location.into(),
            path: PathBuf::from(path),
            kind,
            size_bytes: 1,
            modified_at: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000 - age_secs)),
            created_at: None,
            content_hash: None,
            error: None,
        }
    }

    fn sample() -> Vec<FileRecord> {
        vec![
            record("1", "loc-1", "/a/Budget 2024.pdf", FileKind::Pdf, 30),
            record("2", "loc-1", "/a/notes.md", FileKind::Markdown, 10),
            record("3", "loc-2", "/b/budget.png", FileKind::Image, 20),
        ]
    }

    fn ids(page: &[&FileRecord]) -> Vec<String> {
        page.iter().map(|r| r.id.clone()).collect()
    }

    #[test]
    fn returns_everything_newest_first_by_default() {
        let records = sample();
        let query = FileQuery {
            limit: DEFAULT_LIMIT,
            ..Default::default()
        };
        let (page, total) = apply(records.iter(), &query);
        assert_eq!(total, 3);
        assert_eq!(ids(&page), vec!["2", "3", "1"]);
    }

    #[test]
    fn filters_by_location_kind_and_name() {
        let records = sample();
        let by_name = FileQuery {
            name_contains: Some(" BUDGET ".into()),
            limit: DEFAULT_LIMIT,
            ..Default::default()
        };
        assert_eq!(ids(&apply(records.iter(), &by_name).0), vec!["3", "1"]);

        let by_kind_and_location = FileQuery {
            location_id: Some("loc-1".into()),
            kind: Some(FileKind::Pdf),
            limit: DEFAULT_LIMIT,
            ..Default::default()
        };
        assert_eq!(
            ids(&apply(records.iter(), &by_kind_and_location).0),
            vec!["1"]
        );
    }

    #[test]
    fn pages_results_but_reports_full_total() {
        let records = sample();
        let query = FileQuery {
            limit: 2,
            offset: 1,
            ..Default::default()
        };
        let (page, total) = apply(records.iter(), &query);
        assert_eq!(total, 3);
        assert_eq!(ids(&page), vec!["3", "1"]);
    }
}
