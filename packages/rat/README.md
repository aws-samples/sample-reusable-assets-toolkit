# RAT (Reusable Asset Toolkit)

코드 자산을 인덱싱하고 검색할 수 있도록 하는 툴킷입니다.

## Supported Repository Types

현재 **Git 저장소만** 지원합니다.

- Git 저장소는 `.gitignore`로 인덱싱 대상 파일이 명확하게 정의되어 있어, tracked 파일 목록을 그대로 활용할 수 있습니다.
- `git2` (libgit2 바인딩)를 사용하여 로컬 Git 저장소의 tracked 파일 목록을 추출합니다.
- 일반 디렉토리(non-Git)는 아직 지원하지 않습니다.

향후 다른 저장소 타입 지원을 위한 확장 설계는 [docs/repository-abstraction.md](docs/repository-abstraction.md)를 참고하세요.

## SQS 메시지 구조

파일 단위로 SQS 메시지를 전송합니다. Consumer Lambda가 파일 원본 저장 + 청크별 LLM 설명 생성 + 임베딩을 처리합니다.

```
FileMessage (SQS 메시지 1건 = 파일 1개)
├── action: "upsert" | "delete"
├── repo_id: String                  // git remote URL
├── source_path: String              // 레포 내 상대 경로
├── content: String?                 // 파일 전체 원본 (upsert에만)
└── chunks: Vec<ChunkEntry>          // 청크 목록 (upsert에만)
    ├── source_type: "code" | "doc"
    ├── start_line / end_line
    └── content: String              // imports + 청크 코드
```

레포 전체 삭제(`rat purge`)는 SQS 메시지가 아니라 API 레벨에서 처리됩니다.

### 액션별 샘플

**`upsert`** — 파일 생성/변경 시

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

**`delete`** — 파일 삭제 시 (증분 ingest에서 tracked 파일이 제거된 경우)

```json
{
  "action": "delete",
  "repo_id": "git@gitlab.example.com:team/my-service.git",
  "source_path": "src/handlers/legacy.ts"
}
```

### Consumer Lambda 처리

| action | 처리 |
|--------|------|
| `upsert` | `files` 테이블에 원본 저장 → 청크를 LLM에 전달해 설명 생성 → `snippets` 테이블에 임베딩과 함께 저장 |
| `delete` | `repo_id` + `source_path`로 `files`/`snippets` 레코드 삭제 |

## Crates

| Crate | 설명 |
|-------|------|
| `rat-core` | 핵심 로직 (git 연동, tree-sitter 청킹, DB 모델, SQS 메시지) |
| `rat-cli` | 사용자용 CLI 인터페이스 |
| `rat-api` | Axum 기반 API 서버 (레포/파일 upsert, 검색 엔드포인트) |
| `rat-lambda` | SQS 메시지를 처리하는 Consumer Lambda |
| `rat-migration` | Aurora PostgreSQL 마이그레이션 러너 |

## 인제스트 동작

`rat ingest`는 레포의 현재 상태를 4가지로 분류한 뒤 처리 방식을 결정합니다 (`rat-cli/src/cmd/ingest.rs`).

| 상태 | 조건 | 처리 |
|------|------|------|
| `NotIndexed` | 서버에 레포 레코드 없음 | Full ingest |
| `Interrupted` | 레코드 있으나 `indexed_commit_id = NULL` | Full 재인덱싱 (이전 실행 중단 복구) |
| `AlreadyIndexed` | 저장된 commit = 현재 HEAD | No-op |
| `OutOfDate` | 저장된 commit ≠ 현재 HEAD | 두 커밋 간 diff로 증분 처리 |

`--force` 플래그는 상태와 무관하게 Full 재인덱싱을 강제합니다.

### 파일 선별

- **Full 모드**: `git2::TreeWalkMode::PreOrder`로 HEAD 트리를 순회해 tracked 파일 전체를 가져옵니다.
- **증분 모드**: `git::diff_between_commits()`로 저장된 commit과 현재 HEAD를 비교해 `Added | Modified | Renamed | Copied | Deleted` 엔트리를 추출합니다.
- 양쪽 모두 `chunk::is_supported()`로 지원 언어를 필터링하고, 레포 루트의 `.ratignore` 규칙을 적용합니다.

### 처리 순서

1. `RepoUpsertRequest`를 API로 전송하여 레포 row를 먼저 만들고 `indexed_commit_id`를 `NULL`로 초기화합니다. (이 상태에서 크래시하면 다음 실행이 `Interrupted`로 인식하고 Full 재시도)
2. 파일별로 tree-sitter 청킹 후 `FileMessage`(upsert/delete)를 SQS에 전송합니다.
3. 모든 메시지 전송이 끝나면 README 원문과 함께 최종 `RepoUpsertRequest`를 다시 보내 `indexed_commit_id`를 HEAD로 갱신하고 레포 description을 생성합니다.

## API 엔드포인트

`rat-api`는 AWS Lambda로 배포되며 단일 JSON 이벤트에서 `ApiRequest` enum을 디스패치합니다 (`rat-api/src/main.rs`).

| Request | 설명 |
|---------|------|
| `SearchRequest` | 스니펫 하이브리드 검색 (FTS + 벡터 + RRF) |
| `RepoSearchRequest` | 레포 단위 하이브리드 검색 |
| `ListRequest` | 인덱싱된 레포 목록 |
| `PurgeRequest` | 레포와 관련 파일/스니펫 전체 삭제 |
| `RepoUpsertRequest` | 레포 row 생성/갱신, README 기반 description 생성 |
| `RepoGetRequest` | 단일 레포 메타데이터 조회 |

CLI, MCP 서버 모두 동일한 Lambda를 호출합니다.

## 데이터베이스 스키마

Aurora PostgreSQL Serverless v2 + pgvector. 마이그레이션은 `migrations/*.sql`에 있으며 `rat migration` 명령으로 `rat-migration` Lambda를 통해 적용됩니다.

| 테이블 | 주요 컬럼 | 인덱스 |
|--------|-----------|--------|
| `repos` | `repo_id` (PK), `branch`, `indexed_commit_id`, `description`, `embedding vector(1024)`, `search_vector tsvector` | HNSW(`embedding`, cosine), GIN(`search_vector`) |
| `files` | `id BIGINT IDENTITY` (PK), `repo_id`, `source_path`, `content`, `language` | UNIQUE(`repo_id`, `source_path`) |
| `snippets` | `id` (PK), `file_id` (FK→files), `repo_id`, `content`, `description`, `embedding vector(1024)`, `search_vector tsvector`, `source_type`, `symbol_name`, `start_line`, `end_line`, `tags TEXT[]`, `metadata JSONB` | HNSW(`embedding`, cosine), GIN(`search_vector`), GIN(`tags`), UNIQUE(`file_id`, `start_line`, `end_line`) |

`search_vector`는 `description`에서 생성된 stored generated column이며, 임베딩 대상도 코드 원문이 아닌 **description**입니다 (설명 기반 검색이 코드 리터럴 매칭보다 의도 일치도가 높음).

## 검색 파이프라인 (Hybrid + RRF)

`rat-api/src/actions/search.rs`의 검색 흐름:

1. **쿼리 임베딩 생성** — Bedrock `amazon.nova-2-multimodal-embeddings-v1:0` 호출, `purpose=GENERIC_RETRIEVAL`, 1024 차원
2. **병렬 실행** (tokio::join!):
   - **FTS**: `websearch_to_tsquery('english', query)` → `search_vector` GIN 인덱스
   - **Vector**: 쿼리 임베딩과 `embedding` HNSW 코사인 거리
3. **RRF 융합** — 각 결과의 rank를 받아 `score = Σ 1 / (K + rank + 1)` 로 합산. `K = 60.0`.
4. 통합 score 기준 상위 `limit` 건 반환.

레포 검색(`repo_search`)도 동일한 RRF 로직을 `repos` 테이블에 적용합니다.

인덱싱 시점에는 동일 모델을 `purpose=GENERIC_INDEX`로 호출하여 저장용 임베딩을 생성합니다.

## Consumer Lambda 처리 세부

`rat-lambda/src/main.rs`:

- SQS 배치 이벤트를 받아 메시지별로 처리합니다. 파싱/처리 에러는 로깅 후 다음 메시지로 진행하며, 실패한 개별 메시지는 SQS visibility timeout을 통해 재전달됩니다 (Lambda 내부 재시도 로직 없음).
- **Upsert 흐름** (트랜잭션 내):
  1. `files` 테이블에 원본 upsert
  2. 각 `ChunkEntry`에 대해 Bedrock `Converse` API로 **영어 설명(description)** 생성 (모델은 env `summary_model_id`로 주입). 컨텍스트로 파일 경로, 언어, source_type을 함께 전달.
  3. 생성된 description을 `amazon.nova-2-multimodal-embeddings-v1:0`(`GENERIC_INDEX`)으로 임베딩
  4. `snippets`에 content + description + embedding 저장
- **Delete 흐름**: `(repo_id, source_path)`로 `files` 및 연결된 `snippets` 삭제.
- 설명/임베딩 실패 시 해당 스니펫만 스킵하고 경고 로깅합니다.

## Config & 인증

`rat-cli`의 설정과 토큰 저장 위치:

| 파일 | 권한 | 내용 |
|------|------|------|
| `~/.config/rat/config.toml` | 0644 | 프로파일별 AWS region, Cognito(domain, app_client_id, identity_pool_id, user_pool_id), SQS queue URL, API/migration Lambda ARN |
| `~/.config/rat/credentials.toml` | 0600 | 프로파일별 `TokenSet` (id/access/refresh token, `expires_at`) |

- `rat login`은 Cognito OIDC **PKCE 플로우**로 로컬 `http://localhost:9876`에 콜백 서버를 띄워 인증 코드를 수신합니다.
- 토큰이 만료 60초 이내면 refresh grant로 자동 갱신합니다.
- 키체인 연동은 없으며, 토큰은 0600 권한 평문 TOML로 저장됩니다.
- `rat configure`는 Cognito 도메인·클라이언트 ID 등 일부 필드를 SSM Parameter Store에서 자동으로 해석해 채웁니다.
