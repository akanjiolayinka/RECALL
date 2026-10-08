-- Full-text (keyword) search over chunk text, using SQLite FTS5.
--
-- An "external content" table: the index refers to rows in `chunks` instead
-- of storing the text twice. Triggers keep it in sync, following the pattern
-- in the FTS5 documentation (https://www.sqlite.org/fts5.html#external_content_tables).
--
-- Tokenizer: `unicode61 remove_diacritics 2` splits on Unicode word
-- boundaries and ignores accents; `porter` adds English stemming so that
-- "budgets" also matches "budget".

CREATE VIRTUAL TABLE chunks_fts USING fts5(
    text,
    content = 'chunks',
    content_rowid = 'id',
    tokenize = 'porter unicode61 remove_diacritics 2'
);

CREATE TRIGGER chunks_fts_insert AFTER INSERT ON chunks BEGIN
    INSERT INTO chunks_fts (rowid, text) VALUES (new.id, new.text);
END;

CREATE TRIGGER chunks_fts_delete AFTER DELETE ON chunks BEGIN
    INSERT INTO chunks_fts (chunks_fts, rowid, text) VALUES ('delete', old.id, old.text);
END;

CREATE TRIGGER chunks_fts_update AFTER UPDATE ON chunks BEGIN
    INSERT INTO chunks_fts (chunks_fts, rowid, text) VALUES ('delete', old.id, old.text);
    INSERT INTO chunks_fts (rowid, text) VALUES (new.id, new.text);
END;

-- Index chunks that already exist in databases created before this migration.
INSERT INTO chunks_fts (chunks_fts) VALUES ('rebuild');
