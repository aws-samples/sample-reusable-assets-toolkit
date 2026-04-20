# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.2] - 2026-04-20

### Added

- `rat-api`: `file_list` and `snippet_list` actions for per-repo file and per-file snippet enumeration.
- `rat-mcp`: new Lambda crate exposing the asset store as MCP tools (`search`, `search_repos`, `list_repos`, `file_get`) via AgentCore Gateway.
- `rat-cli`: `file_get` tool on the stdio MCP server.
- Set workspace license to Amazon Software License.
- Add `CHANGELOG.md`.

### Changed

- Extract API-calling logic from `rat-cli` into a new `rat-client` crate so it can be shared by non-CLI clients; `rat-cli` commands now wrap `rat_client::ops`.

## [0.0.1] - 2026-04-15

- Initial version.
