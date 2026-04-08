# Ingestion CLI

GitLab 레포를 분석하여 코드 스니펫을 추출하고, LLM 설명 생성 및 임베딩을 거쳐 벡터 DB에 저장하는 CLI 도구.

## 아키텍처

```
CLI (로컬)                                         AWS
┌──────────────┐                    ┌──────────────────────────────────┐
│ 파일 순회     │                    │                                  │
│ tree-sitter  │──청크 배치 전송────▶│ API Gateway                      │
│ 청킹         │                    │        │                         │
└──────────────┘                    │        ▼                         │
                                    │      SQS                         │
                                    │        │                         │
                                    │        ▼                         │
                                    │ Consumer Lambda (VPC 내부)       │
                                    │  ├─ LLM 설명 생성 (Bedrock)      │
                                    │  ├─ 임베딩 생성 (Bedrock)        │
                                    │  └─ DB 저장 (Aurora)             │
                                    └──────────────────────────────────┘
```

### CLI의 역할 (로컬)
1. 파일 순회 + tree-sitter AST 파싱 + 청킹
2. 추출된 청크를 API Gateway 경유로 SQS에 전송

### 서버 사이드의 역할 (AWS)
1. Consumer Lambda가 SQS에서 청크 수신
2. Bedrock으로 LLM 설명 생성 + 임베딩
3. Aurora PostgreSQL (pgvector)에 저장

### 인증
- API Gateway에서 인증 처리 (Cognito 연동)

## 파이프라인

```
[로컬 폴더 또는 git clone] → 파일 순회 → 언어 판별 → tree-sitter AST 파싱
→ 함수/클래스 추출 → API Gateway → SQS → Consumer Lambda
→ LLM 설명 생성 + 중요도 판단 → 임베딩 → Aurora pgvector 저장
```

입력은 두 가지 모드를 지원:
- **로컬 경로**: 이미 clone된 폴더를 직접 지정
- **원격 URL**: GitLab/GitHub URL을 받아 임시 디렉토리에 clone 후 처리

## 처리 대상

| 소스 타입 | 청킹 방식 | description 생성 |
|-----------|----------|-----------------|
| 소스코드 | tree-sitter AST (함수/클래스 단위) | LLM 생성 (영어) |
| README/문서 | 섹션(헤딩) 단위 | LLM 생성 (영어) |
| 설정 파일 (CDK 등) | 리소스 블록 단위 | LLM 생성 (영어) |

모든 소스 타입에 대해 동일하게 LLM 영어 설명을 생성한다.
원본 콘텐츠(한글 README 포함)는 content 필드에 보존.
description은 임베딩 + tsvector(전문 검색) 양쪽에 사용되므로 영어로 통일해야 일관된 검색 품질을 보장할 수 있다.

## AST 파싱

tree-sitter를 사용하여 단일 라이브러리로 다중 언어 지원.

### 청킹 전략

```
파일 파싱:
1차: 함수/클래스 추출 시도
  ├─ 있음 → 개별 청크
  └─ 없음 또는 추출된 코드가 파일의 일부만 커버
       → 파일 전체를 하나의 청크로 추가
       → 200줄 초과 시 논리적 블록(빈 줄 기준)으로 분할
```

모듈 레벨 코드(라우터 설정, CDK 스택, 에이전트 구성 등)가 TS/Python에서 빈번하므로,
함수/클래스가 없는 파일도 반드시 인덱싱 대상에 포함한다.

### 지원 언어 및 추출 대상 노드

| 언어 | 노드 타입 |
|------|----------|
| TypeScript | function_declaration, method_definition, class_declaration, arrow_function |
| Python | function_definition, class_definition |
| Java | method_declaration, class_declaration |
| Go | function_declaration, method_declaration |
| Rust | function_item, impl_item, struct_item |

새 언어 추가 = grammar 크레이트 추가 + 노드 타입 매핑.

## LLM 설명 생성

파일 단위 배치 처리. 한 파일의 모든 함수를 묶어서 LLM에 한 번에 전달.

LLM이 각 함수에 대해:
1. 영어 설명 생성
2. 인덱싱 가치 판단 (HIGH / LOW)

HIGH만 임베딩. LOW는 저장하되 임베딩 스킵.

모델: Nova Lite 또는 Haiku (저비용, 설명 생성에 충분한 품질).

## 임베딩

모델: Amazon Nova Embed (기본)
- 의도 이해, 동의어 인식, 크로스링구얼 검색에 강점
- 차원: 1024

임베딩 대상은 description 필드 (코드 원본이 아님).

## 저장소

Aurora PostgreSQL Serverless v2 + pgvector.

하이브리드 서치 지원 (벡터 유사도 + 전문 검색).

```sql
CREATE TABLE snippets (
    id UUID PRIMARY KEY,
    repo_id TEXT NOT NULL,
    content TEXT NOT NULL,           -- 원본 코드/문서
    description TEXT NOT NULL,       -- LLM 생성 설명 또는 원본 (문서)
    embedding vector(1024),          -- description의 임베딩
    search_vector tsvector           -- 전문 검색용 (description 기반)
        GENERATED ALWAYS AS (to_tsvector('english', description)) STORED,
    source_type TEXT NOT NULL,       -- 'code' | 'doc' | 'readme'
    source_path TEXT NOT NULL,       -- 파일 경로
    language TEXT,                   -- 프로그래밍 언어
    symbol_name TEXT,                -- 함수/클래스 이름
    start_line INT,
    end_line INT,
    indexing_value TEXT,             -- 'HIGH' | 'LOW'
    tags TEXT[],
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX ON snippets USING ivfflat (embedding vector_cosine_ops);
CREATE INDEX ON snippets USING gin (search_vector);
```

## 기술 스택

- CLI: Rust (tree-sitter, reqwest)
- 메시지 큐: Amazon SQS
- API: Amazon API Gateway (인증: Cognito)
- Consumer: AWS Lambda (Rust, VPC 내부)
- LLM: Amazon Bedrock (Nova Lite)
- 임베딩: Amazon Bedrock (Nova Embed)
- DB: Aurora PostgreSQL Serverless v2 + pgvector
- Git: git2 크레이트 또는 CLI

## CLI 사용법 (예상)

```bash
# 로컬 폴더 인덱싱
rat ingest ./my-service

# 원격 레포 인덱싱
rat ingest --repo https://gitlab.example.com/team/my-service.git

# 특정 브랜치 (원격만)
rat ingest --repo <url> --branch main

# 재인덱싱
rat ingest ./my-service --force

# 인덱싱 상태 확인
rat status
```
