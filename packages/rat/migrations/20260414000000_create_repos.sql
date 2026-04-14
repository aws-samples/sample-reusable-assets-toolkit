CREATE TABLE repos (
    repo_id TEXT PRIMARY KEY,
    branch TEXT NOT NULL,
    indexed_commit_id TEXT,
    description TEXT,
    embedding vector(1024),
    search_vector tsvector
        GENERATED ALWAYS AS (to_tsvector('english', COALESCE(description, ''))) STORED,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX ON repos USING hnsw (embedding vector_cosine_ops);
CREATE INDEX ON repos USING gin (search_vector);

ALTER TABLE files
    ADD CONSTRAINT files_repo_id_fkey
    FOREIGN KEY (repo_id) REFERENCES repos(repo_id) ON DELETE CASCADE;
