//! Hybrid search: combine meaning, keyword and file-name matches into one
//! ranked list with one result per file.
//!
//! Each file gets up to three scores between 0 and 1:
//! - semantic: how close in meaning its best passage is (`semantic::score`;
//!   needs an embedding model)
//! - keyword: share of the query's words found in its best passage
//! - metadata: share of the query's words found in its file name or title
//!
//! They are combined as a weighted average. With an embedding model the
//! weights are `WEIGHTS` (meaning dominates); without one they are
//! `WEIGHTS_WITHOUT_SEMANTIC`, where words in the text and words in the file
//! name count equally — otherwise a file named exactly what you searched for
//! could never score well. Both are tunable starting points, not measured
//! optima. On equal scores, a file whose name matches comes first.

#[cfg(test)]
mod calibration;
pub mod keyword;
pub mod metadata;
pub mod semantic;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::embeddings::Embedder;
use crate::files::FileKind;

pub use keyword::SnippetPart;

pub struct Weights {
    pub semantic: f64,
    pub keyword: f64,
    pub metadata: f64,
}

pub const WEIGHTS: Weights = Weights {
    semantic: 0.65,
    keyword: 0.25,
    metadata: 0.10,
};

pub const WEIGHTS_WITHOUT_SEMANTIC: Weights = Weights {
    semantic: 0.0,
    keyword: 0.5,
    metadata: 0.5,
};

/// Longest snippet shown for a passage found by meaning alone.
const SEMANTIC_SNIPPET_CHARS: usize = 300;

/// Where in a file the evidence for a result is.
#[derive(Debug, Clone, PartialEq)]
pub struct Passage {
    pub chunk_id: i64,
    pub page_number: Option<u32>,
    pub snippet: Vec<SnippetPart>,
}

/// Per-signal scores, 0–1; `None` when the signal didn't match this file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Signals {
    pub semantic: Option<f64>,
    pub keyword: Option<f64>,
    pub metadata: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub file_id: i64,
    pub path: PathBuf,
    pub kind: FileKind,
    /// `None` when only the file name matched (e.g. an image not read yet).
    pub passage: Option<Passage>,
    /// Combined 0–1 score.
    pub relevance: f64,
    /// Plain-language reasons, e.g. `Mentions “budget”`.
    pub match_reasons: Vec<String>,
    pub signals: Signals,
}

/// Search the library. `embedder` is `None` when no embedding model is
/// available; search then uses keywords and file names only.
pub fn search(
    conn: &Connection,
    embedder: Option<&dyn Embedder>,
    query: &str,
    limit: usize,
) -> rusqlite::Result<Vec<SearchResult>> {
    let terms = keyword::query_terms(query);
    let typed = keyword::typed_forms(query);
    let quote_list = |terms: &[String]| {
        terms
            .iter()
            .map(|t| format!("“{}”", typed.get(t).unwrap_or(t)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let keyword_hits = keyword::search(conn, query, usize::MAX)?;
    let metadata_hits = metadata::search(conn, &terms)?;
    let semantic_hits = match embedder {
        Some(embedder) if !query.trim().is_empty() => semantic::search(conn, embedder, query)?,
        _ => Vec::new(),
    };

    let mut results: HashMap<i64, SearchResult> = HashMap::new();

    for hit in semantic_hits {
        let result = entry(&mut results, hit.file_id, &hit.path, hit.kind);
        result.signals.semantic = Some(semantic::score(hit.similarity));
        result.passage = Some(Passage {
            chunk_id: hit.chunk_id,
            page_number: hit.page_number,
            snippet: vec![SnippetPart {
                text: shorten(&hit.text),
                highlight: false,
            }],
        });
    }
    for hit in keyword_hits {
        let result = entry(&mut results, hit.file_id, &hit.path, hit.kind);
        result.signals.keyword =
            Some(hit.matched_terms.len() as f64 / hit.query_term_count.max(1) as f64);
        result
            .match_reasons
            .push(format!("Mentions {}", quote_list(&hit.matched_terms)));
        // A keyword passage has highlighted words, so it is better evidence.
        result.passage = Some(Passage {
            chunk_id: hit.chunk_id,
            page_number: hit.page_number,
            snippet: hit.snippet,
        });
    }
    for hit in metadata_hits {
        let result = entry(&mut results, hit.file_id, &hit.path, hit.kind);
        result.signals.metadata = Some(hit.matched_terms.len() as f64 / terms.len().max(1) as f64);
        result.match_reasons.push(format!(
            "File name or title matches {}",
            quote_list(&hit.matched_terms)
        ));
    }

    let semantic_available = embedder.is_some();
    let mut results: Vec<SearchResult> = results
        .into_values()
        .map(|mut result| {
            if result.signals.semantic.is_some() {
                result.match_reasons.insert(0, "Similar in meaning".into());
            }
            result.relevance = combine(&result.signals, semantic_available);
            result
        })
        .collect();
    results.sort_by(|a, b| {
        b.relevance
            .total_cmp(&a.relevance)
            .then(
                b.signals
                    .metadata
                    .is_some()
                    .cmp(&a.signals.metadata.is_some()),
            )
            .then(a.path.cmp(&b.path))
    });
    results.truncate(limit);
    Ok(results)
}

/// The result for `file_id`, created empty on first use.
fn entry<'a>(
    results: &'a mut HashMap<i64, SearchResult>,
    file_id: i64,
    path: &Path,
    kind: FileKind,
) -> &'a mut SearchResult {
    results.entry(file_id).or_insert_with(|| SearchResult {
        file_id,
        path: path.to_path_buf(),
        kind,
        passage: None,
        relevance: 0.0,
        match_reasons: Vec::new(),
        signals: Signals::default(),
    })
}

/// Weighted average of the signals. A signal that didn't match this file
/// counts as 0.
pub fn combine(signals: &Signals, semantic_available: bool) -> f64 {
    let w = if semantic_available {
        &WEIGHTS
    } else {
        &WEIGHTS_WITHOUT_SEMANTIC
    };
    let total = w.semantic * signals.semantic.unwrap_or(0.0)
        + w.keyword * signals.keyword.unwrap_or(0.0)
        + w.metadata * signals.metadata.unwrap_or(0.0);
    total / (w.semantic + w.keyword + w.metadata)
}

/// Collapse whitespace and cut at a word boundary.
fn shorten(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= SEMANTIC_SNIPPET_CHARS {
        return flat;
    }
    let cut: String = flat.chars().take(SEMANTIC_SNIPPET_CHARS).collect();
    match cut.rfind(' ') {
        Some(space) => format!("{}…", &cut[..space]),
        None => format!("{cut}…"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_connection;
    use crate::embeddings::{normalize, EmbedError};
    use crate::indexing::embed::embed_location;
    use crate::test_support::{add_document, add_file, add_location};
    use std::sync::atomic::AtomicBool;

    /// Test embedder with a hand-made notion of "meaning": texts about
    /// housing point one way, everything else another. Lets tests check that
    /// a meaning match is found even when no words overlap.
    struct HousingEmbedder;

    impl HousingEmbedder {
        fn vector(text: &str) -> Vec<f32> {
            let text = text.to_lowercase();
            let housing = ["apartment", "flat", "landlord", "tenancy", "moving"]
                .iter()
                .any(|w| text.contains(w));
            let mut v = if housing {
                vec![1.0, 0.1]
            } else {
                vec![0.1, 1.0]
            };
            normalize(&mut v);
            v
        }
    }

    impl Embedder for HousingEmbedder {
        fn model_id(&self) -> &str {
            "test-housing"
        }
        fn dimensions(&self) -> usize {
            2
        }
        fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
            Ok(texts.iter().map(|t| Self::vector(t)).collect())
        }
        fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
            Ok(Self::vector(text))
        }
    }

    struct Library {
        conn: Connection,
        proposal: i64,
        lease: i64,
        receipt: i64,
    }

    fn library() -> Library {
        let mut conn = test_connection();
        let loc = add_location(&conn);
        let proposal = add_document(
            &conn,
            loc,
            "/docs/Garden proposal.pdf",
            &[(Some(3), "The estimated project budget is NGN 2,500,000.")],
        );
        let lease = add_document(
            &conn,
            loc,
            "/docs/Tenancy agreement.pdf",
            &[(
                Some(2),
                "Give the landlord sixty days notice before you leave the flat.",
            )],
        );
        let receipt = add_file(&conn, loc, "/pics/Headphones receipt.jpg");
        embed_location(
            &mut conn,
            &HousingEmbedder,
            loc,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        Library {
            conn,
            proposal,
            lease,
            receipt,
        }
    }

    #[test]
    fn combine_uses_the_weights_for_the_available_signals() {
        let all_words = Signals {
            keyword: Some(1.0),
            ..Default::default()
        };
        assert!((combine(&all_words, false) - 0.5).abs() < 1e-9);
        assert!((combine(&all_words, true) - 0.25).abs() < 1e-9);
        let everything = Signals {
            semantic: Some(1.0),
            keyword: Some(1.0),
            metadata: Some(1.0),
        };
        assert!((combine(&everything, true) - 1.0).abs() < 1e-9);
        assert_eq!(combine(&Signals::default(), true), 0.0);
    }

    #[test]
    fn without_a_model_keywords_and_file_names_still_work() {
        let lib = library();
        let hits = search(&lib.conn, None, "project budget", 10).unwrap();
        assert_eq!(hits[0].file_id, lib.proposal);
        assert_eq!(hits[0].passage.as_ref().unwrap().page_number, Some(3));
        assert_eq!(hits[0].match_reasons, vec!["Mentions “project”, “budget”"]);
        assert_eq!(hits[0].signals.semantic, None);

        let receipt = search(&lib.conn, None, "headphone receipts", 10).unwrap();
        assert_eq!(receipt[0].file_id, lib.receipt);
        assert!((receipt[0].relevance - 0.5).abs() < 1e-9);
        assert_eq!(receipt[0].passage, None, "an unread image has no passage");
        assert_eq!(
            receipt[0].match_reasons,
            vec!["File name or title matches “headphone”, “receipts”"]
        );
    }

    #[test]
    fn meaning_matches_are_found_without_shared_words() {
        let lib = library();
        // No document contains "apartment", but the lease is about housing.
        assert!(search(&lib.conn, None, "apartment", 10).unwrap().is_empty());
        let hits = search(&lib.conn, Some(&HousingEmbedder), "apartment", 10).unwrap();
        assert_eq!(
            hits.len(),
            1,
            "unrelated passages fall below the similarity threshold"
        );
        assert_eq!(hits[0].file_id, lib.lease);
        assert_eq!(hits[0].match_reasons, vec!["Similar in meaning"]);
        let passage = hits[0].passage.as_ref().unwrap();
        assert_eq!(passage.page_number, Some(2));
        assert!(passage.snippet[0].text.starts_with("Give the landlord"));
    }

    #[test]
    fn signals_add_up_when_a_file_matches_several_ways() {
        let lib = library();
        let hits = search(&lib.conn, Some(&HousingEmbedder), "tenancy notice", 10).unwrap();
        let lease = &hits[0];
        assert_eq!(lease.file_id, lib.lease);
        assert!(
            lease.signals.semantic.is_some()
                && lease.signals.keyword.is_some()
                && lease.signals.metadata.is_some()
        );
        assert_eq!(lease.match_reasons[0], "Similar in meaning");
        // Meaning 1.0; keywords 0.5 (only "notice" is in the passage);
        // file name 0.5 (only "tenancy" is in the name).
        assert_eq!(lease.signals.keyword, Some(0.5));
        assert_eq!(lease.signals.metadata, Some(0.5));
        assert!((lease.relevance - (0.65 + 0.25 * 0.5 + 0.10 * 0.5)).abs() < 1e-3);
        // The keyword passage (with highlights) is the evidence shown.
        assert!(lease
            .passage
            .as_ref()
            .unwrap()
            .snippet
            .iter()
            .any(|p| p.highlight));
    }

    #[test]
    fn a_file_named_like_the_query_beats_a_text_that_only_mentions_it() {
        let lib = library();
        let loc = crate::database::locations::list(&lib.conn).unwrap()[0].id;
        add_document(
            &lib.conn,
            loc,
            "/docs/index.md",
            &[(None, "See Headphones receipt.jpg in pics.")],
        );
        let hits = search(&lib.conn, None, "headphone receipt", 10).unwrap();
        assert_eq!(hits[0].file_id, lib.receipt);
    }

    #[test]
    fn reasons_show_words_as_the_user_typed_them() {
        let lib = library();
        let hits = search(&lib.conn, None, "NGN 2,500,000", 10).unwrap();
        assert_eq!(hits[0].file_id, lib.proposal);
        assert_eq!(hits[0].match_reasons, vec!["Mentions “NGN”, “2,500,000”"]);
    }

    #[test]
    fn shorten_cuts_long_passages_at_a_word() {
        let long = "word ".repeat(100);
        let short = shorten(&long);
        assert!(short.ends_with("word…"));
        assert!(short.chars().count() <= SEMANTIC_SNIPPET_CHARS + 1);
        assert_eq!(shorten("  a \n b  "), "a b");
    }

    /// Calibration and end-to-end check with the real model on test-data/:
    /// searches that share no words with the right file must still find it.
    /// Unrelated queries must find nothing. Prints every file's best
    /// similarity, which is how MIN_SIMILARITY was chosen. Run with `cargo test --release -- --ignored --nocapture`.
    #[test]
    #[ignore = "needs the BGE model files"]
    fn real_model_finds_test_files_by_meaning() {
        use crate::database::locations;
        use crate::embeddings::bge::{BgeSmall, MODEL_SUBDIR};
        use crate::indexing::pipeline::{index_location, Models};
        use crate::ocr::Ocr;
        use crate::test_support::real_models_dir;

        let models = real_models_dir();
        let model = BgeSmall::load(&models.join(MODEL_SUBDIR)).expect("BGE model files installed");
        let ocr = Ocr::load(&models.join("ocrs")).ok();
        let mut conn = test_connection();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test-data");
        let location = locations::insert(&conn, &root, 0).unwrap().id;
        let started = std::time::Instant::now();
        let status = index_location(
            &mut conn,
            location,
            &root,
            Models {
                ocr: ocr.as_ref(),
                embedder: Some(&model),
            },
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        println!(
            "indexed test-data: {} passages embedded in {:?}",
            status.passages_embedded,
            started.elapsed()
        );

        let name = |path: &Path| path.file_name().unwrap().to_string_lossy().into_owned();
        // (query, acceptable top results). None of these queries shares a
        // searchable word with its expected file.
        let mut cases = vec![
            (
                "what is the price of the allotment",
                vec!["Project Proposal"],
            ),
            (
                "when do I get my money back after leaving the apartment",
                vec!["Tenancy Agreement", "Moving out checklist"],
            ),
            ("promotion expenses", vec!["Q3 Marketing Plan"]),
            ("groceries to buy", vec!["Shopping list"]),
        ];
        if ocr.is_some() {
            cases.push(("headset purchase", vec!["Headphones receipt"]));
        }
        let unrelated = ["quantum physics lecture", "recipe for chocolate cake"];

        let mut failures = Vec::new();
        for query in cases.iter().map(|c| c.0).chain(unrelated) {
            let vector = model.embed_query(query).unwrap();
            let mut best: Vec<(String, f32)> = Vec::new();
            for (chunk_id, sim) in
                crate::database::embeddings::nearest(&conn, model.model_id(), &vector, 1000)
                    .unwrap()
            {
                let path: String = conn
                    .query_row(
                        "SELECT files.path FROM chunks
                         JOIN documents ON documents.id = chunks.document_id
                         JOIN files ON files.id = documents.file_id
                         WHERE chunks.id = ?1",
                        [chunk_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                let file = name(Path::new(&path));
                if !best.iter().any(|(f, _)| *f == file) {
                    best.push((file, sim));
                }
            }
            println!("\n{query:?}");
            for (file, sim) in &best {
                println!("  {sim:.3}  {file}");
            }
            let results = search(&conn, Some(&model), query, 3).unwrap();
            for r in &results {
                println!(
                    "  -> {:.3} {} {:?}",
                    r.relevance,
                    name(&r.path),
                    r.match_reasons
                );
            }
            if let Some((_, expected)) = cases.iter().find(|c| c.0 == query) {
                let top = results.first().map(|r| name(&r.path)).unwrap_or_default();
                if !expected.iter().any(|e| top.starts_with(e)) {
                    failures.push(format!(
                        "{query:?}: top result {top:?}, expected {expected:?}"
                    ));
                }
            } else if !results.is_empty() {
                failures.push(format!("{query:?}: unrelated query matched {results:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
