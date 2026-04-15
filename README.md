# Reusable Asset Toolkit

MCP Server + Skills 기반의 재사용 가능한 코드 자산(Reusable Assets) 검색 및 적용 툴킷.
AI 코딩 어시스턴트(Kiro, Claude Code 등) 워크플로우 안에서 큐레이션된 코드 자산을 검색하고 현재 프로젝트 컨텍스트에 맞게 적용할 수 있습니다.

## 해결하려는 문제

- 조직 내에 좋은 코드 패턴, 유틸리티, 템플릿이 존재하지만 발견하기 어렵고 재사용이 드묾
- 위키에 문서화되어 있어도 코딩 시점에서 참조하기 불편
- 복사-붙여넣기한 코드가 현재 컨텍스트에 맞지 않아 추가 수정 비용 발생

## 워크스페이스 구조

Nx + pnpm 워크스페이스로 구성되며, Rust 기반의 `rat` 툴킷과 TypeScript 기반의 AWS CDK 인프라로 나뉩니다.

```
packages/
├── rat/        # Rust 워크스페이스 (CLI, API, Lambda, 코어 로직)
├── infra/      # AWS CDK 스택 (network, auth, storage, application)
└── common/     # 공유 CDK 컨스트럭트
```

### `rat` 크레이트

| Crate | 설명 |
|-------|------|
| `rat-core` | Git 연동, tree-sitter 청킹, DB 모델 등 핵심 로직 |
| `rat-cli` | 사용자용 CLI (ingest, search, chunk, mcp 등) |
| `rat-api` | Axum 기반 API 서버 |
| `rat-lambda` | SQS 메시지를 처리하는 Consumer Lambda |
| `rat-migration` | Aurora PostgreSQL 마이그레이션 러너 |

### `rat-cli` 명령어

- `configure`, `login` — 설정 및 Midway 인증
- `ingest` — 로컬 Git 레포를 파싱·청킹하여 API로 전송 (증분/`--force` 지원)
- `chunk` — tree-sitter 기반 파일 청킹 결과를 로컬에서 미리보기
- `list`, `search`, `status` — 인덱싱된 레포/스니펫 조회 및 하이브리드 검색
- `purge` — 레포 전체 데이터 삭제
- `mcp` — MCP 서버 모드 실행

### `infra` 스택

| Stack | 설명 |
|-------|------|
| `network-stack` | VPC, 서브넷, 엔드포인트 |
| `auth-stack` | 인증 관련 리소스 |
| `storage-stack` | Aurora PostgreSQL(pgvector), SQS |
| `application-stack` | API Gateway, Lambda, MCP 서버 배포 |

## 데이터 수집 파이프라인

```
로컬 Git 레포 → rat-cli ingest (tree-sitter 파싱 + 청킹)
             → API Gateway → SQS
             → rat-lambda (LLM 설명 생성 + 임베딩)
             → Aurora PostgreSQL (pgvector)
```

상세 설계는 [packages/rat/README.md](./packages/rat/README.md)를 참고하세요.

### tree-sitter 기반 코드 청킹

tree-sitter로 소스 코드를 AST로 파싱하고 언어별로 의미 있는 단위(함수, 클래스, 구조체 등)로 분할합니다.

| 언어 | 추출 단위 |
|------|----------|
| Rust | `function_item`, `impl_item`, `struct_item`, `enum_item`, `trait_item`, `macro_definition`, `type_item` |
| TypeScript/TSX | `function_declaration`, `class_declaration`, `export_statement`, `lexical_declaration` |
| JavaScript | 동일 (+ `require()` 패턴 import 인식) |
| Python | `function_definition`, `class_definition`, `decorated_definition` |
| Go | `function_declaration`, `method_declaration`, `type_declaration` |
| Java | `method_declaration`, `class_declaration`, `interface_declaration`, `enum_declaration` |

청킹 동작:

1. **최상위 선언 추출**: 언어별 타겟 노드를 AST에서 추출하여 개별 청크로 생성
2. **어트리뷰트/데코레이터 병합**: `#[derive]`, `@Injectable()`, `@dataclass` 등 선언 위의 어트리뷰트를 해당 청크에 포함
3. **Doc 주석 병합**: 선언 바로 위의 doc 주석(`///`, `/** */`, `#`)을 해당 청크에 포함
4. **Import 필터링**: 각 청크에서 실제 사용하는 import만 선별
5. **커버리지 보완**: 남은 코드를 별도 청크로 수집 (200줄 초과 시 빈 줄 기준 분할)

```bash
rat chunk <파일경로>
```

### 지원 저장소 타입

현재는 **Git 저장소만** 지원합니다. `git2`(libgit2 바인딩)로 tracked 파일 목록을 추출하여 `.gitignore`에 맞게 대상 파일을 결정합니다. 일반 디렉토리 및 다른 VCS는 아직 지원하지 않습니다.

## 검색 및 MCP 노출

- 벡터(임베딩) + 키워드 기반 하이브리드 검색
- `rat search` CLI와 `rat mcp` MCP 서버를 통해 동일한 검색 기능 제공
- MCP 도구: `search`, `search_repos`, `list_repos`

AI 어시스턴트는 MCP 도구를 통해 대화 컨텍스트에 관련 스니펫을 주입하여 검증된 패턴 기반 코드 생성을 유도합니다.

## 접근 제어

MCP 서버 및 API는 Cognito 인증을 통해 접근을 제어합니다.

## 실행 방법

`rat` CLI는 `packages/rat`에서 빌드됩니다. 빌드된 바이너리는 `packages/rat/target/release/rat`에 생성됩니다.

```sh
# 빌드
cargo build --release --manifest-path packages/rat/Cargo.toml
```

### 초기 설정

```sh
# 1. 서버 엔드포인트/프로파일 설정
rat configure

# 2. Cognito 로그인 (브라우저 기반 OIDC PKCE)
rat login
```

프로파일을 여러 개 운영하려면 모든 명령에 `--profile <name>`을 붙이면 됩니다 (기본값: `default`).

### 레포 인덱싱 및 검색

```sh
# 현재 디렉토리(Git 레포)를 인덱싱
rat ingest .

# 변경 여부와 무관하게 전체 재인덱싱
rat ingest . --force

# 인덱싱된 레포 목록
rat list

# SQS 큐 상태 확인
rat status

# 코드 스니펫 검색 (기본 scope=code)
rat search "vector search hybrid"

# 레포 단위 검색
rat search "payments service" --scope repo

# 특정 레포로 범위 제한
rat search "retry logic" --repo-id git@gitlab.example.com:team/my-service.git

# 레포 전체 데이터 삭제
rat purge <repo_id>
```

### tree-sitter 청킹 미리보기

```sh
rat chunk path/to/file.ts
```

## MCP 서버 설정

`rat mcp` 는 stdio 기반 MCP 서버로 동작하며, 다음 도구를 노출합니다.

- `search` — 코드 스니펫/문서 하이브리드 검색
- `search_repos` — 레포지토리 검색
- `list_repos` — 인덱싱된 레포 목록

### Claude Code

`~/.claude.json` 또는 프로젝트 `.mcp.json`에 추가합니다.

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

프로파일을 지정하려면 `args`에 `["mcp", "--profile", "<name>"]`을 넣으면 됩니다.

### Kiro

워크스페이스 단위는 `.kiro/settings/mcp.json`, 사용자 단위는 `~/.kiro/settings/mcp.json`에 등록합니다. 두 파일이 모두 있으면 워크스페이스 설정이 우선하며 병합됩니다.

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

### 주의

MCP 호출 전에 `rat login`으로 Cognito 토큰을 먼저 발급받아야 합니다. 토큰이 만료되면 검색이 실패하므로 다시 로그인하세요.
