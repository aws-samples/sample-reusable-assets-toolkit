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
├── action: "upsert" | "delete" | "purge"
├── repo_id: String                  // git remote URL
├── commit_id: String                // 처리 기준 커밋 SHA
├── source_path: String?             // 레포 내 상대 경로 (purge 시 생략)
├── content: String?                 // 파일 전체 원본 (upsert에만)
└── chunks: Vec<ChunkEntry>          // 청크 목록 (upsert에만)
    ├── source_type: "code" | "doc"
    ├── start_line / end_line
    └── content: String              // imports + 청크 코드
```

### 액션별 샘플

**`upsert`** — 파일 생성/변경 시

```json
{
  "action": "upsert",
  "repo_id": "git@gitlab.example.com:team/my-service.git",
  "commit_id": "a1b2c3d4e5f6...",
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

**`delete`** — 파일 삭제 시 (`--since`로 증분 ingest할 때)

```json
{
  "action": "delete",
  "repo_id": "git@gitlab.example.com:team/my-service.git",
  "commit_id": "a1b2c3d4e5f6...",
  "source_path": "src/handlers/legacy.ts"
}
```

**`purge`** — 레포 전체 재인덱싱 시 (`--force`)

```json
{
  "action": "purge",
  "repo_id": "git@gitlab.example.com:team/my-service.git",
  "commit_id": "a1b2c3d4e5f6..."
}
```

### Consumer Lambda 처리

| action | 처리 |
|--------|------|
| `upsert` | `files` 테이블에 원본 저장 → 청크를 LLM에 전달해 설명 생성 → `snippets` 테이블에 임베딩과 함께 저장 |
| `delete` | `repo_id` + `source_path`로 `files`/`snippets` 레코드 삭제 |
| `purge` | `repo_id`에 해당하는 모든 `files`/`snippets` 레코드 삭제 (force 재인덱싱 직전) |

## Crates

| Crate | 설명 |
|-------|------|
| `rat-core` | 핵심 로직 (git 연동, DB 모델) |
| `rat-cli` | CLI 인터페이스 |
| `rat-lambda` | AWS Lambda 핸들러 |
