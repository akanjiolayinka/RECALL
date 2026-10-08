-- Recall database, version 1.
-- Times are milliseconds since the Unix epoch (UTC). Paths are absolute.

-- Folders the user asked Recall to index.
CREATE TABLE locations (
    id        INTEGER PRIMARY KEY,
    path      TEXT    NOT NULL UNIQUE,
    added_at  INTEGER NOT NULL
);

-- Every supported file found inside a location.
CREATE TABLE files (
    id            INTEGER PRIMARY KEY,
    location_id   INTEGER NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    path          TEXT    NOT NULL UNIQUE,
    filename      TEXT    NOT NULL,
    extension     TEXT    NOT NULL,
    kind          TEXT    NOT NULL,   -- pdf | text | markdown | docx | image
    mime_type     TEXT    NOT NULL,
    size_bytes    INTEGER NOT NULL,
    created_at    INTEGER,            -- not available on every filesystem
    modified_at   INTEGER,
    scanned_at    INTEGER NOT NULL,   -- last time the scanner saw this file
    indexed_at    INTEGER,            -- last time its contents were indexed
    content_hash  TEXT,               -- SHA-256, hex; NULL if unreadable
    status        TEXT    NOT NULL,   -- pending | indexed | error
    error         TEXT                -- user-facing reason when status = error
);
CREATE INDEX files_location_id ON files(location_id);
CREATE INDEX files_modified_at ON files(modified_at);

-- Text extracted from a file (one row per file). Filled from Milestone 5.
CREATE TABLE documents (
    id              INTEGER PRIMARY KEY,
    file_id         INTEGER NOT NULL UNIQUE REFERENCES files(id) ON DELETE CASCADE,
    title           TEXT,
    author          TEXT,
    language        TEXT,
    page_count      INTEGER,
    word_count      INTEGER,
    extracted_text  TEXT    NOT NULL
);

-- Searchable pieces of a document. Filled from Milestone 5.
CREATE TABLE chunks (
    id           INTEGER PRIMARY KEY,
    document_id  INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index  INTEGER NOT NULL,
    text         TEXT    NOT NULL,
    page_number  INTEGER,             -- 1-based; NULL when the format has no pages
    char_start   INTEGER NOT NULL,    -- offsets into documents.extracted_text
    char_end     INTEGER NOT NULL,
    token_count  INTEGER,
    UNIQUE (document_id, chunk_index)
);

-- Vector embedding of each chunk. Filled from Milestone 7.
CREATE TABLE embeddings (
    chunk_id    INTEGER PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
    model       TEXT    NOT NULL,     -- which model produced it, to detect model changes
    dimensions  INTEGER NOT NULL,
    embedding   BLOB    NOT NULL      -- little-endian f32 values
);
