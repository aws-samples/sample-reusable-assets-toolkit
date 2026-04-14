CREATE TABLE repos (
    repo_id TEXT PRIMARY KEY,
    branch TEXT NOT NULL,
    indexed_commit_id TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

ALTER TABLE files
    ADD CONSTRAINT files_repo_id_fkey
    FOREIGN KEY (repo_id) REFERENCES repos(repo_id) ON DELETE CASCADE;
