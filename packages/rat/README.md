# RAT (Reusable Asset Toolkit)

A toolkit for indexing and searching code assets.

## Supported Repository Types

Only **Git repositories** are supported today.

- Git repos define indexable files clearly via `.gitignore`, so the tracked file list can be used as-is.
- `git2` (libgit2 bindings) extracts the tracked file list from a local Git repo.
- Plain directories (non-Git) are not supported yet.

See [docs/repository-abstraction.md](docs/repository-abstraction.md) for the extension design to support other repository types.

## SQS Message Format

Messages are sent on a per-file basis. The Consumer Lambda stores the raw file, generates a per-chunk LLM description, and embeds them.

```
FileMessage (1 SQS message = 1 file)
├── action: "upsert" | "delete"
├── repo_id: String                  // git remote URL
├── source_path: String              // repo-relative path
├── content: String?                 // full file content (upsert only)
└── chunks: Vec<ChunkEntry>          // chunks (upsert only)
    ├── source_type: "code" | "doc"
    ├── start_line / end_line
    └── content: String              // imports + chunk code
```

Whole-repo deletion (`rat purge`) is handled at the API level, not as an SQS message.

### Samples by action

**`upsert`** — on file creation/modification

```json
{
  "action": "upsert",
  "repo_id": "git@gitlab.example.com:team/my-service.git",
  "source_path": "src/handlers/user.ts",
  "content": "import { Request, Response } from 'express';\n\nexport class UserService { ... }",
  "chunks": [
    {
      "source_type": "code",
      "start_line": 3,
      "end_line": 20,
      "content": "import { Request, Response } from 'express';\n\nexport class UserService { ... }"
    }
  ]
}
```

**`delete`** — when an incremental ingest sees a tracked file removed

```json
{
  "action": "delete",
  "repo_id": "git@gitlab.example.com:team/my-service.git",
  "source_path": "src/handlers/legacy.ts"
}
```

### Consumer Lambda handling

| action | Handling |
|--------|----------|
| `upsert` | Store raw file in `files` → send chunks to the LLM for descriptions → write to `snippets` with the embedding |
| `delete` | Delete `files`/`snippets` rows by `(repo_id, source_path)` |

## Crates

| Crate | Description |
|-------|-------------|
| `rat-core` | Core logic (git integration, tree-sitter chunking, DB models, SQS messages) |
| `rat-cli` | User-facing CLI |
| `rat-api` | Axum-based API server (repo/file upsert, search endpoints) |
| `rat-lambda` | Consumer Lambda that processes SQS messages |
| `rat-migration` | Aurora PostgreSQL migration runner |

## Ingest Behavior

`rat ingest` classifies the current repo into one of four states and chooses a strategy from there (`rat-cli/src/cmd/ingest.rs`).

| State | Condition | Handling |
|-------|-----------|----------|
| `NotIndexed` | No repo row on the server | Full ingest |
| `Interrupted` | Row exists but `indexed_commit_id = NULL` | Full re-ingest (recovery from a previous aborted run) |
| `AlreadyIndexed` | Stored commit = current HEAD | No-op |
| `OutOfDate` | Stored commit ≠ current HEAD | Incremental processing from the diff between the two commits |

The `--force` flag forces a full re-ingest regardless of state.

### File selection

- **Full mode**: walks the HEAD tree with `git2::TreeWalkMode::PreOrder` to collect every tracked file.
- **Incremental mode**: `git::diff_between_commits()` compares the stored commit against HEAD and extracts `Added | Modified | Renamed | Copied | Deleted` entries.
- Both modes filter by `chunk::is_supported()` for supported languages and apply `.ratignore` rules at the repo root.

### Processing order

1. Send a `RepoUpsertRequest` to the API to create the repo row with `indexed_commit_id = NULL`. (If the CLI crashes in this state, the next run detects `Interrupted` and retries with a full ingest.)
2. For each file, tree-sitter chunk it and send a `FileMessage` (upsert/delete) to SQS.
3. Once every message has been dispatched, send a final `RepoUpsertRequest` with the README content to set `indexed_commit_id` to HEAD and generate the repo description.

## API Endpoints

`rat-api` is deployed as an AWS Lambda and dispatches on the `ApiRequest` enum from a single JSON event (`rat-api/src/main.rs`).

| Request | Description |
|---------|-------------|
| `SearchRequest` | Snippet hybrid search (FTS + vector + RRF) |
| `RepoSearchRequest` | Repo-level hybrid search |
| `ListRequest` | List indexed repos |
| `PurgeRequest` | Delete a repo and all its files/snippets |
| `RepoUpsertRequest` | Create/update a repo row and generate its description from the README |
| `RepoGetRequest` | Fetch single repo metadata |

Both the CLI and the MCP server call the same Lambda.

## Database Schema

Aurora PostgreSQL Serverless v2 + pgvector. Migrations live in `migrations/*.sql` and are applied through the `rat-migration` Lambda via `rat migration`.

| Table | Key columns | Indexes |
|-------|-------------|---------|
| `repos` | `repo_id` (PK), `branch`, `indexed_commit_id`, `description`, `embedding vector(1024)`, `search_vector tsvector` | HNSW(`embedding`, cosine), GIN(`search_vector`) |
| `files` | `id BIGINT IDENTITY` (PK), `repo_id`, `source_path`, `content`, `language` | UNIQUE(`repo_id`, `source_path`) |
| `snippets` | `id` (PK), `file_id` (FK→files), `repo_id`, `content`, `description`, `embedding vector(1024)`, `search_vector tsvector`, `source_type`, `symbol_name`, `start_line`, `end_line`, `tags TEXT[]`, `metadata JSONB` | HNSW(`embedding`, cosine), GIN(`search_vector`), GIN(`tags`), UNIQUE(`file_id`, `start_line`, `end_line`) |

`search_vector` is a stored generated column derived from `description`, and the embedding target is the **description**, not the raw code (description-based search aligns with intent better than literal code matching).

## Search Pipeline (Hybrid + RRF)

Flow in `rat-api/src/actions/search.rs`:

1. **Generate the query embedding** — Bedrock `amazon.nova-2-multimodal-embeddings-v1:0`, `purpose=GENERIC_RETRIEVAL`, 1024 dims.
2. **Run in parallel** (tokio::join!):
   - **FTS**: `websearch_to_tsquery('english', query)` against the `search_vector` GIN index
   - **Vector**: cosine distance against `embedding` via the HNSW index
3. **RRF fusion** — take the rank from each side and sum `score = Σ 1 / (K + rank + 1)` with `K = 60.0`.
4. Return the top `limit` entries by fused score.

Repo search (`repo_search`) applies the same RRF logic against the `repos` table.

At indexing time the same model is called with `purpose=GENERIC_INDEX` to produce the stored embeddings.

## Consumer Lambda Details

`rat-lambda/src/main.rs`:

- Accepts SQS batch events and processes each message. Parse/handling errors are logged and the loop moves on; failed messages rely on the SQS visibility timeout for redelivery (no in-Lambda retry).
- **Upsert flow** (inside a transaction):
  1. Upsert the raw file into `files`
  2. For each `ChunkEntry`, call the Bedrock `Converse` API to generate an **English description** (the model is injected via the `summary_model_id` env var). File path, language, and source_type are passed as context.
  3. Embed the generated description with `amazon.nova-2-multimodal-embeddings-v1:0` (`GENERIC_INDEX`).
  4. Store content + description + embedding in `snippets`.
- **Delete flow**: remove `files` and its linked `snippets` by `(repo_id, source_path)`.
- If description generation or embedding fails for a chunk, that snippet is skipped with a warning log.

## Config & Auth

Where `rat-cli` keeps its configuration and tokens:

| File | Mode | Contents |
|------|------|----------|
| `~/.config/rat/config.toml` | 0644 | Per-profile AWS region, Cognito (domain, app_client_id, identity_pool_id, user_pool_id), SQS queue URL, API/migration Lambda ARNs |
| `~/.config/rat/credentials.toml` | 0600 | Per-profile `TokenSet` (id/access/refresh token, `expires_at`) |

- `rat login` runs the Cognito OIDC **PKCE flow**, spinning up a local callback server on `http://localhost:9876` to receive the auth code.
- Tokens are auto-refreshed via the refresh grant when they are within 60 seconds of expiry.
- There is no keychain integration; tokens are stored in plaintext TOML with 0600 permissions.
- `rat configure` resolves some fields (Cognito domain, client ID, etc.) automatically from SSM Parameter Store.
