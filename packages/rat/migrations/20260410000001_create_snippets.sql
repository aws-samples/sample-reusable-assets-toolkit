CREATE TABLE files (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    repo_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    content TEXT NOT NULL,
    language TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX ON files (repo_id);
CREATE UNIQUE INDEX ON files (repo_id, source_path);

CREATE TABLE snippets (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    file_id BIGINT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    repo_id TEXT NOT NULL,
    content TEXT NOT NULL,
    description TEXT NOT NULL,
    embedding vector(1024),
    search_vector tsvector
        GENERATED ALWAYS AS (to_tsvector('english', description)) STORED,
    source_type TEXT NOT NULL,
    symbol_name TEXT,
    start_line INT,
    end_line INT,
    indexing_value TEXT,
    tags TEXT[],
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX ON snippets (file_id);
CREATE INDEX ON snippets (repo_id);
CREATE UNIQUE INDEX ON snippets (file_id, start_line, end_line);
CREATE INDEX ON snippets USING hnsw (embedding vector_cosine_ops);
CREATE INDEX ON snippets USING gin (search_vector);
CREATE INDEX ON snippets USING gin (tags);
