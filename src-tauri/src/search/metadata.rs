//! Matching search words against file names and document titles.
//!
//! This is the only way to find files whose text hasn't been read, such as
//! images before OCR ("headphones receipt.jpg").

use std::path::PathBuf;

use rusqlite::{params_from_iter, Connection};

use crate::files::FileKind;

const CANDIDATES: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataHit {
    pub file_id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
    /// Query terms found in the file name or title, in query order.
    pub matched_terms: Vec<String>,
}

/// Files whose name or title contains any of `terms` (from `keyword::query_terms`).
pub fn search(conn: &Connection, terms: &[String]) -> rusqlite::Result<Vec<MetadataHit>> {
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    // A cheap substring pre-filter in SQL; `matched_terms` then checks words.
    let conditions = (1..=terms.len())
        .map(|i| {
            format!(
                "instr(lower(files.filename || ' ' || coalesce(documents.title, '')), ?{i}) > 0"
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    let mut stmt = conn.prepare(&format!(
        "SELECT files.id, files.path, files.kind, files.filename, documents.title
         FROM files LEFT JOIN documents ON documents.file_id = files.id
         WHERE {conditions}
         LIMIT {CANDIDATES}"
    ))?;
    let rows = stmt.query_map(params_from_iter(terms.iter().map(|t| stem(t))), |row| {
        let name: String = row.get(3)?;
        let title: Option<String> = row.get(4)?;
        Ok(MetadataHit {
            file_id: row.get(0)?,
            path: PathBuf::from(row.get::<_, String>(1)?),
            kind: FileKind::parse(&row.get::<_, String>(2)?).unwrap_or(FileKind::Text),
            matched_terms: matched_terms(terms, &format!("{name} {}", title.unwrap_or_default())),
        })
    })?;
    let hits: Vec<MetadataHit> = rows.collect::<rusqlite::Result<_>>()?;
    Ok(hits
        .into_iter()
        .filter(|hit| !hit.matched_terms.is_empty())
        .collect())
}

/// Drop a plural "s" so "receipts" also matches "receipt".
fn stem(term: &str) -> &str {
    match term.strip_suffix('s') {
        Some(stem) if stem.len() >= 3 => stem,
        _ => term,
    }
}

/// Terms that start a word of `text` (case-insensitive). Multi-word terms
/// such as "2 500 000" must appear as consecutive words.
fn matched_terms(terms: &[String], text: &str) -> Vec<String> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let joined = format!(" {} ", words.join(" "));
    terms
        .iter()
        .filter(|term| {
            if term.contains(' ') {
                joined.contains(&format!(" {term} "))
            } else {
                let stem = stem(term);
                words.iter().any(|word| word.starts_with(stem))
            }
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;
    use crate::test_support::{add_document, add_file, add_location};

    fn terms(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn matches_whole_word_starts_and_plurals() {
        let name = "Headphones receipt 2024-05.jpg";
        assert_eq!(
            matched_terms(&terms(&["receipts", "headphone"]), name),
            terms(&["receipts", "headphone"])
        );
        assert!(
            matched_terms(&terms(&["phones"]), name).is_empty(),
            "not a word start"
        );
        assert_eq!(
            matched_terms(&terms(&["2024 05"]), name),
            terms(&["2024 05"])
        );
    }

    #[test]
    fn finds_unread_images_by_name_and_documents_by_title() {
        let conn = test_connection();
        let loc = add_location(&conn);
        let image = add_file(&conn, loc, "/pics/Headphones receipt.jpg");
        add_file(&conn, loc, "/pics/holiday.png");
        let doc = add_document(
            &conn,
            loc,
            "/docs/q3.md",
            &[(None, "# Marketing plan\nSpend.")],
        );
        conn.execute(
            "UPDATE documents SET title = 'Marketing plan' WHERE file_id = ?1",
            [doc],
        )
        .unwrap();

        let hits = search(&conn, &terms(&["headphone", "receipts"])).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_id, image);
        assert_eq!(hits[0].matched_terms, terms(&["headphone", "receipts"]));

        let by_title = search(&conn, &terms(&["marketing"])).unwrap();
        assert_eq!(
            by_title.iter().map(|h| h.file_id).collect::<Vec<_>>(),
            vec![doc]
        );
    }
}
