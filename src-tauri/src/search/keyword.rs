//! Keyword search using SQLite FTS5 (see migration 0002).

use std::path::PathBuf;

use rusqlite::{params, Connection};

use crate::files::FileKind;

/// Common English words that carry no meaning for finding a document.
/// Dropped from queries unless the query has nothing else.
const STOPWORDS: &[&str] = &[
    "a", "about", "an", "and", "are", "as", "at", "be", "can", "did", "do", "find", "for", "from",
    "had", "has", "have", "how", "i", "in", "is", "it", "me", "my", "of", "on", "or", "our",
    "save", "saved", "show", "that", "the", "this", "to", "was", "we", "what", "when", "where",
    "which", "who", "why", "with", "you", "your",
];

/// Longer queries are cut to this many words to keep matching fast.
const MAX_TERMS: usize = 16;

/// Words of context FTS5 includes around matches in a snippet.
const SNIPPET_WORDS: i64 = 24;

// Private-use characters that never appear in normal text, used to mark where
// FTS5 highlights matches inside a snippet.
const MARK_START: char = '\u{E000}';
const MARK_END: char = '\u{E001}';

/// Part of a snippet; `highlight` is true for words that matched the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetPart {
    pub text: String,
    pub highlight: bool,
}

/// A file's best keyword match.
#[derive(Debug, Clone, PartialEq)]
pub struct KeywordHit {
    pub chunk_id: i64,
    pub file_id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
    pub page_number: Option<u32>,
    pub snippet: Vec<SnippetPart>,
    /// The query's terms found in this passage, in query order.
    pub matched_terms: Vec<String>,
    /// Number of terms in the query, so callers can tell how complete a match is.
    pub query_term_count: usize,
    /// FTS5 BM25 score: more negative means a better match.
    pub bm25: f64,
}

/// The words of a search, lowercased and without filler words.
///
/// Pieces of one typed word, such as "2,500,000" or "e-mail", stay together
/// as a phrase ("2 500 000") so they match together.
pub fn query_terms(input: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for word in input.split_whitespace() {
        let pieces: Vec<String> = word
            .split(|c: char| !c.is_alphanumeric())
            .filter(|piece| !piece.is_empty())
            .map(str::to_lowercase)
            .collect();
        let term = pieces.join(" ");
        if !term.is_empty() && !terms.contains(&term) {
            terms.push(term);
        }
    }
    let meaningful: Vec<String> = terms
        .iter()
        .filter(|t| !STOPWORDS.contains(&t.as_str()))
        .cloned()
        .collect();
    // A query of only filler words is still searched for, rather than nothing.
    let mut chosen = if meaningful.is_empty() {
        terms
    } else {
        meaningful
    };
    chosen.truncate(MAX_TERMS);
    chosen
}

/// A safe FTS5 expression for one term: quoted, so punctuation and FTS5
/// operators typed by the user can't change the query's meaning.
fn quoted(term: &str) -> String {
    format!("\"{term}\"")
}

/// Matches passages containing any of the terms; BM25 ranks them.
fn any_of(terms: &[String]) -> String {
    terms
        .iter()
        .map(|t| quoted(t))
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// How many of the best BM25 matches are considered before re-ranking.
const CANDIDATES: usize = 200;

/// The best-matching passage of each file, best files first.
///
/// Passages containing more of the query's words rank first; BM25 orders
/// passages with the same number of words. Plain BM25 alone strongly favours
/// very short texts, so a two-word note mentioning one search word would
/// outrank a document mentioning all of them.
pub fn search(conn: &Connection, query: &str, limit: usize) -> rusqlite::Result<Vec<KeywordHit>> {
    let terms = query_terms(query);
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let mut hits = best_passage_per_file(conn, &terms)?;
    for term in &terms {
        for chunk_id in passages_containing(conn, term, &hits)? {
            if let Some(hit) = hits.iter_mut().find(|h| h.chunk_id == chunk_id) {
                hit.matched_terms.push(term.clone());
            }
        }
    }
    hits.sort_by(|a, b| {
        b.matched_terms
            .len()
            .cmp(&a.matched_terms.len())
            .then(a.bm25.total_cmp(&b.bm25))
    });
    hits.truncate(limit);
    Ok(hits)
}

/// Which of the `hits` passages contain `term` (as FTS5 matches it, so
/// "budget" also matches "budgets").
fn passages_containing(
    conn: &Connection,
    term: &str,
    hits: &[KeywordHit],
) -> rusqlite::Result<Vec<i64>> {
    let ids: Vec<String> = hits.iter().map(|h| h.chunk_id.to_string()).collect();
    // The ids are integers we produced ourselves, so joining them is safe.
    let mut stmt = conn.prepare(&format!(
        "SELECT rowid FROM chunks_fts WHERE chunks_fts MATCH ?1 AND rowid IN ({})",
        ids.join(",")
    ))?;
    let rows = stmt.query_map([quoted(term)], |row| row.get(0))?;
    rows.collect()
}

/// For each file, its single best passage by BM25 among the top candidates.
fn best_passage_per_file(conn: &Connection, terms: &[String]) -> rusqlite::Result<Vec<KeywordHit>> {
    let mut stmt = conn.prepare(
        "WITH hits AS (
             SELECT chunks.id AS chunk_id, chunks.document_id, chunks.page_number,
                    bm25(chunks_fts) AS score,
                    snippet(chunks_fts, 0, ?2, ?3, '…', ?4) AS snippet
             FROM chunks_fts JOIN chunks ON chunks.id = chunks_fts.rowid
             WHERE chunks_fts MATCH ?1
         ),
         ranked AS (
             SELECT *, ROW_NUMBER() OVER (PARTITION BY document_id ORDER BY score) AS position
             FROM hits
         )
         SELECT ranked.chunk_id, files.id, files.path, files.kind, ranked.page_number,
                ranked.snippet, ranked.score
         FROM ranked
         JOIN documents ON documents.id = ranked.document_id
         JOIN files ON files.id = documents.file_id
         WHERE ranked.position = 1
         ORDER BY ranked.score
         LIMIT ?5",
    )?;
    let rows = stmt.query_map(
        params![
            any_of(terms),
            MARK_START.to_string(),
            MARK_END.to_string(),
            SNIPPET_WORDS,
            CANDIDATES as i64
        ],
        |row| {
            let path: String = row.get(2)?;
            let kind: String = row.get(3)?;
            Ok(KeywordHit {
                chunk_id: row.get(0)?,
                file_id: row.get(1)?,
                path: PathBuf::from(path),
                kind: FileKind::parse(&kind).unwrap_or(FileKind::Text),
                page_number: row.get(4)?,
                snippet: parse_snippet(&row.get::<_, String>(5)?),
                matched_terms: Vec::new(),
                query_term_count: terms.len(),
                bm25: row.get(6)?,
            })
        },
    )?;
    rows.collect()
}

/// Split FTS5's marked-up snippet into plain and highlighted parts.
fn parse_snippet(marked: &str) -> Vec<SnippetPart> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut highlight = false;
    for c in marked.chars() {
        if c == MARK_START || c == MARK_END {
            if !current.is_empty() {
                parts.push(SnippetPart {
                    text: std::mem::take(&mut current),
                    highlight,
                });
            }
            highlight = c == MARK_START;
        } else {
            // Show line breaks inside a snippet as spaces.
            current.push(if c == '\n' { ' ' } else { c });
        }
    }
    if !current.is_empty() {
        parts.push(SnippetPart {
            text: current,
            highlight,
        });
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;
    use crate::test_support::{add_document, add_location};

    #[test]
    fn extracts_terms_without_filler_words_or_operators() {
        assert_eq!(
            query_terms("Where did I save my apartment notes?"),
            vec!["apartment", "notes"]
        );
        assert_eq!(
            query_terms("budget NOT \"draft\" OR (x*"),
            vec!["budget", "not", "draft", "x"]
        );
        assert_eq!(query_terms("NGN 2,500,000"), vec!["ngn", "2 500 000"]);
        assert_eq!(query_terms("budget Budget BUDGET"), vec!["budget"]);
        // Only filler words: search for them anyway rather than nothing.
        assert_eq!(query_terms("the"), vec!["the"]);
        assert!(query_terms("  ?!  ").is_empty());
        assert_eq!(
            any_of(&query_terms("garden (budget")),
            "\"garden\" OR \"budget\""
        );
    }

    #[test]
    fn parses_marked_snippets() {
        let marked = format!("The {MARK_START}budget{MARK_END} is\nset…");
        assert_eq!(
            parse_snippet(&marked),
            vec![
                SnippetPart {
                    text: "The ".into(),
                    highlight: false
                },
                SnippetPart {
                    text: "budget".into(),
                    highlight: true
                },
                SnippetPart {
                    text: " is set…".into(),
                    highlight: false
                },
            ]
        );
    }

    fn library() -> (Connection, i64, i64, i64) {
        let conn = test_connection();
        let loc = add_location(&conn);
        let proposal = add_document(
            &conn,
            loc,
            "/docs/proposal.pdf",
            &[
                (Some(1), "Riverside community garden proposal."),
                (
                    Some(3),
                    "The estimated project budget is NGN 2,500,000 for the garden.",
                ),
            ],
        );
        let lease = add_document(
            &conn,
            loc,
            "/docs/lease.pdf",
            &[(
                Some(2),
                "Give sixty days notice before moving out of the flat.",
            )],
        );
        let notes = add_document(
            &conn,
            loc,
            "/docs/notes.md",
            &[(
                None,
                "Garden committee: order the water tank. Budgets are tight.",
            )],
        );
        (conn, proposal, lease, notes)
    }

    #[test]
    fn finds_the_right_file_page_and_highlights() {
        let (conn, proposal, _, _) = library();
        let hits = search(&conn, "What was the project budget?", 10).unwrap();
        let top = &hits[0];
        assert_eq!(top.file_id, proposal);
        assert_eq!(top.page_number, Some(3));
        assert_eq!(top.matched_terms, vec!["project", "budget"]);
        assert_eq!(top.query_term_count, 2);
        assert!(top
            .snippet
            .iter()
            .any(|p| p.highlight && p.text == "budget"));
    }

    #[test]
    fn passages_with_more_query_words_beat_short_texts_with_fewer() {
        let (conn, proposal, _, _) = library();
        let loc = crate::database::locations::list(&conn).unwrap()[0].id;
        let tiny = add_document(&conn, loc, "/docs/budget.txt", &[(None, "budget notes")]);
        let hits = search(&conn, "project budget", 10).unwrap();
        let ids: Vec<i64> = hits.iter().map(|h| h.file_id).collect();
        assert_eq!(ids[0], proposal, "contains both words");
        assert!(ids.contains(&tiny));
    }

    #[test]
    fn stemming_matches_word_forms() {
        let (conn, proposal, _, notes) = library();
        let ids: Vec<i64> = search(&conn, "budgets", 10)
            .unwrap()
            .iter()
            .map(|h| h.file_id)
            .collect();
        assert!(ids.contains(&proposal) && ids.contains(&notes));
    }

    #[test]
    fn returns_one_result_per_file_best_first() {
        let (conn, proposal, _, notes) = library();
        // "garden" appears on two pages of the proposal and once in the notes.
        let hits = search(&conn, "garden", 10).unwrap();
        let ids: Vec<i64> = hits.iter().map(|h| h.file_id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&proposal) && ids.contains(&notes));
        assert!(hits[0].bm25 <= hits[1].bm25);
    }

    #[test]
    fn exact_numbers_match_as_a_phrase() {
        let (conn, proposal, _, _) = library();
        let hits = search(&conn, "2,500,000", 10).unwrap();
        assert_eq!(
            hits.iter().map(|h| h.file_id).collect::<Vec<_>>(),
            vec![proposal]
        );
        assert!(search(&conn, "2,600,000", 10).unwrap().is_empty());
    }

    #[test]
    fn nothing_matches_or_empty_queries_return_nothing() {
        let (conn, _, _, _) = library();
        assert!(search(&conn, "headphones", 10).unwrap().is_empty());
        assert!(search(&conn, "???", 10).unwrap().is_empty());
    }

    #[test]
    fn removed_files_disappear_from_results_and_the_index_stays_consistent() {
        let (conn, _, lease, _) = library();
        assert_eq!(search(&conn, "notice", 10).unwrap()[0].file_id, lease);
        crate::database::files::delete(&conn, lease).unwrap();
        assert!(search(&conn, "notice", 10).unwrap().is_empty());
        // FTS5's own check that the index matches the chunks table.
        conn.execute(
            "INSERT INTO chunks_fts (chunks_fts) VALUES ('integrity-check')",
            [],
        )
        .unwrap();
    }
}
