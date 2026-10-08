//! Finding the passages that answer a search.
//!
//! Search returns one result per file: its best-matching passage, where it
//! is (page), a highlighted snippet and why it matched. Keyword search
//! (SQLite FTS5) is the first strategy; meaning-based search joins it in a
//! later milestone.

pub mod keyword;

use rusqlite::Connection;

pub use keyword::{KeywordHit, SnippetPart};

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub hit: KeywordHit,
    /// 0–1: the share of the search's words found in the passage.
    pub relevance: f64,
    /// Plain-language reasons this file matched, e.g. `Mentions “budget”`.
    pub match_reasons: Vec<String>,
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> rusqlite::Result<Vec<SearchResult>> {
    let hits = keyword::search(conn, query, limit)?;
    Ok(hits
        .into_iter()
        .map(|hit| SearchResult {
            relevance: hit.matched_terms.len() as f64 / hit.query_term_count.max(1) as f64,
            match_reasons: keyword_reasons(&hit.matched_terms),
            hit,
        })
        .collect())
}

fn keyword_reasons(terms: &[String]) -> Vec<String> {
    if terms.is_empty() {
        return Vec::new();
    }
    let quoted: Vec<String> = terms.iter().map(|t| format!("“{t}”")).collect();
    vec![format!("Mentions {}", quoted.join(", "))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_quote_the_matched_words() {
        assert_eq!(
            keyword_reasons(&["budget".into(), "garden".into()]),
            vec!["Mentions “budget”, “garden”"]
        );
        assert!(keyword_reasons(&[]).is_empty());
    }
}
