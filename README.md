# Reusable Asset Toolkit

A toolkit for searching and applying reusable code assets, built on MCP Server + Skills.
Developers can discover curated code assets inside AI coding assistant workflows (Kiro, Claude Code, etc.) and apply them to the current project context.

## Problems We Solve

- Good code patterns, utilities, and templates exist inside the organization but are hard to discover and rarely reused
- Wiki documentation is inconvenient to reference at coding time
- Copy-pasted code doesn't fit the current context, incurring rework cost

## Workspace Layout

An Nx + pnpm workspace. The Rust-based `rat` toolkit and the TypeScript AWS CDK infrastructure live side by side.

```
packages/
├── rat/        # Rust workspace (CLI, API, Lambda, core logic)
├── infra/      # AWS CDK stacks (network, auth, storage, application)
└── common/     # Shared CDK constructs
```

### `rat` crates

| Crate | Description |
|-------|-------------|
| `rat-core` | Core logic: Git integration, tree-sitter chunking, DB models |
| `rat-cli` | User-facing CLI (ingest, search, chunk, mcp, etc.) |
| `rat-api` | Axum-based API server |
| `rat-lambda` | Consumer Lambda that processes SQS messages |
| `rat-migration` | Aurora PostgreSQL migration runner |

### `rat-cli` commands

- `configure`, `login` — profile setup and Cognito authentication
- `ingest` — parse and chunk a local Git repo, then ship it to the API (incremental / `--force`)
- `chunk` — preview tree-sitter chunking output locally
- `list`, `search`, `status` — browse indexed repos/snippets and run hybrid search
- `purge` — delete all data for a repo
- `mcp` — run as an MCP server

### `infra` stacks

| Stack | Description |
|-------|-------------|
| `network-stack` | VPC, subnets, endpoints |
| `auth-stack` | Authentication resources |
| `storage-stack` | Aurora PostgreSQL (pgvector), SQS |
| `application-stack` | API Gateway, Lambda, MCP server deployment |

## Ingestion Pipeline

```
Local Git repo → rat-cli ingest (tree-sitter parse + chunk)
              → API Gateway → SQS
              → rat-lambda (LLM description + embedding)
              → Aurora PostgreSQL (pgvector)
```

For design details see [packages/rat/README.md](./packages/rat/README.md).

### tree-sitter based code chunking

tree-sitter parses source files into an AST and splits them by language into meaningful units (functions, classes, structs, etc.).

| Language | Extracted nodes |
|----------|-----------------|
| Rust | `function_item`, `impl_item`, `struct_item`, `enum_item`, `trait_item`, `macro_definition`, `type_item` |
| TypeScript/TSX | `function_declaration`, `class_declaration`, `export_statement`, `lexical_declaration` |
| JavaScript | Same as TS (plus `require()`-style import detection) |
| Python | `function_definition`, `class_definition`, `decorated_definition` |
| Go | `function_declaration`, `method_declaration`, `type_declaration` |
| Java | `method_declaration`, `class_declaration`, `interface_declaration`, `enum_declaration` |

Chunking behavior:

1. **Top-level declaration extraction**: each target node becomes its own chunk.
2. **Attribute/decorator merging**: attributes sitting above a declaration (`#[derive]`, `@Injectable()`, `@dataclass`, …) are folded into the chunk.
3. **Doc comment merging**: doc comments directly above a declaration (`///`, `/** */`, `#`) are folded into the chunk.
4. **Import filtering**: only the imports each chunk actually uses are kept on it.
5. **Coverage fill**: code not covered by any extracted chunk is collected into separate chunks (split on blank lines when it exceeds 200 lines).

```bash
rat chunk <path/to/file>
```

### Supported repository types

Only **Git repositories** are supported today. `git2` (libgit2 bindings) pulls the tracked file list so that `.gitignore` rules naturally decide what gets indexed. Plain directories and other VCS backends are not supported yet.

## Search & MCP Surface

- Hybrid search combining vector (embedding) and keyword ranking
- The same search is exposed through both the `rat search` CLI and the `rat mcp` MCP server
- MCP tools: `search`, `search_repos`, `list_repos`

AI assistants call these MCP tools to inject relevant snippets into the conversation context, grounding code generation in validated patterns.

## Access Control

Both the MCP server and the API enforce Cognito authentication.

## Running

The `rat` CLI is built from `packages/rat`. The release binary lands at `packages/rat/target/release/rat`.

```sh
# Build
cargo build --release --manifest-path packages/rat/Cargo.toml
```

### Initial setup

```sh
# 1. Configure server endpoints and profile
rat configure
```

![rat configure](./docs/configure.gif)

To keep multiple profiles, pass `--profile <name>` to any command (default: `default`).

```sh
# 2. Log in via Cognito (browser-based OIDC PKCE)
rat login
```
![rat login](./docs/login.gif)

### Indexing and searching

```sh
# Ingest the current directory (must be a Git repo)
rat ingest .

# Force full re-index regardless of state
rat ingest . --force
```
![rat ingest](./docs/ingest.gif)
```sh
# List indexed repos
rat list
```
![rat list](./docs/list.gif)

```sh
# Check SQS queue status
rat status
```
![rat status](./docs/status.gif)

```sh
# Search code snippets (default scope=code)
rat search "sqs cdk"

# Repo-level search
rat search "unsturctured docuemnt analytics service" --scope repo

# Restrict to a single repo
rat search "retry logic" --repo-id git@gitlab.example.com:team/my-service.git

# Purge all data for a repo
rat purge <repo_id>
```
![rat search](./docs/search.gif) 


### Preview tree-sitter chunking

```sh
rat chunk path/to/file.ts
```

## MCP Server Configuration

`rat mcp` runs as a stdio MCP server and exposes the following tools:

- `search` — hybrid search over code snippets and docs
- `search_repos` — repository search
- `list_repos` — list indexed repositories

### Claude Code

Add to `~/.claude.json` or the project-level `.mcp.json`.

```json
{
  "mcpServers": {
    "rat": {
      "command": "/absolute/path/to/rat",
      "args": ["mcp"]
    }
  }
}
```

To pin a profile, set `args` to `["mcp", "--profile", "<name>"]`.

### Kiro

Register at the workspace level in `.kiro/settings/mcp.json`, or at the user level in `~/.kiro/settings/mcp.json`. If both exist, the workspace config takes precedence and the two are merged.

```json
{
  "mcpServers": {
    "rat": {
      "command": "/absolute/path/to/rat",
      "args": ["mcp"]
    }
  }
}
```

Tool names listed in `autoApprove` are executed without per-call confirmation. To pin a profile, change `args` to `["mcp", "--profile", "<name>"]`.

![rat mcp](./docs/mcp.svg) 

### Note

Run `rat login` to obtain a Cognito token **before** invoking MCP tools. If the token expires, searches will fail — log in again.
