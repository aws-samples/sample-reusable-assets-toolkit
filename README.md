# Reusable Asset Toolkit

MCP Server + Skills 기반의 재사용 가능한 코드 자산(Reusable Assets) 검색 및 적용 툴킷.
AI 기반 개발 환경(Kiro) 내에서 큐레이션된 코드 자산을 직접 검색하고 프로젝트에 적용할 수 있습니다.

## 해결하려는 문제

- 조직 내에 좋은 코드 패턴, 유틸리티, 템플릿이 존재하지만 발견하기 어렵고 재사용이 드묾
- 위키에 문서화되어 있어도 코딩 시점에서 참조하기 불편
- 복사-붙여넣기한 코드가 현재 컨텍스트에 맞지 않아 추가 수정 비용 발생

## 솔루션 개요

개발자가 Kiro(AI 코딩 어시스턴트) 워크플로우 내에서 Reusable Asset을 검색하고 현재 프로젝트 컨텍스트에 맞게 적용할 수 있는 도구를 제공합니다.

## 핵심 컴포넌트

| 컴포넌트 | 설명 |
|----------|------|
| Asset DB | AWS Cloud에 호스팅된 Asset 데이터베이스. 분석된 Asset 메타데이터와 코드 저장 |
| MCP Server | Asset DB와 인터페이스하는 MCP 서버 (AWS Cloud 배포) |
| Kiro Skills | Reusable Asset 활용을 위한 Kiro 스킬 세트. 자연어 요청 기반으로 적절한 Asset 검색 및 적용 |

## 주요 기능

### 1. 컨텍스트 기반 Asset 검색 및 추천

사용자의 문제 도메인, 기술 스택, AWS 서비스가 식별되면 관련 Reusable Asset을 자동으로 검색하여 제시합니다.

- 대화, 프로젝트 파일, 에디터 컨텍스트에서 키워드 추출
- 키워드, 태그, 카테고리, 언어, AWS 서비스 등 복합 조건으로 Asset 카탈로그 검색
- 매칭된 Asset을 요약 형태로 제시하고, 선택 시 상세 정보 제공
- MCP 도구 예시: `asset_search`, `asset_get`, `asset_list`

### 2. 자동화된 Asset 설치 및 배포

선택한 Asset을 로컬 환경에 설치하거나 AWS Cloud에 배포합니다.
NanoClaw의 "skills over features" 접근 방식에서 영감을 받아, 설치 프로세스를 빌트인 기능이 아닌 Skill로 관리합니다.

- 각 Asset은 메타데이터에 설치/배포 워크플로우 단계를 포함
- MCP 서버가 워크플로우 정의를 데이터로 제공
- Skill이 워크플로우를 로드하고, AI가 CLI 명령을 단계별로 실행
- 단계 유형: 환경 확인, 파일 복사, CLI 실행, 사용자 안내
- 실패 시 각 단계별 에러 메시지 및 해결 가이드 제공

### 3. LLM 코드 생성을 위한 Pattern DB

PACE에서 관리하는 큐레이션된 코드 스니펫을 LLM 코드 생성의 참조 예시로 제공합니다.

- 특정 패턴의 코드 작성 시, 매칭되는 Asset을 검색하여 LLM 컨텍스트에 주입
- LLM이 검증된 예시를 기반으로 코드를 생성하여 일관된 패턴 보장
- 조직의 코딩 표준과 모범 사례를 반영한 코드 생성 유도
- 단순 코드 복사가 아닌, 현재 프로젝트 컨텍스트에 패턴을 적용하여 새로운 코드 생성

## Asset 수집

### Asset 소스 (PACE Reusable Asset만 해당)

- 내부 GitLab 리포지토리
- GitHub aws-samples 등 공개 리포지토리

### 수집 흐름

```
대상 리포 → Ingestion CLI (tree-sitter 파싱 + 청킹) → API Gateway → SQS
→ Consumer Lambda (LLM 설명 생성 + 임베딩) → Aurora PostgreSQL (pgvector)
```

Ingestion CLI가 레포의 소스코드, 문서, 설정 파일을 분석하여 코드 스니펫을 추출하고,
서버 사이드에서 LLM 설명 생성 및 벡터 임베딩을 거쳐 DB에 저장합니다.

상세 설계는 [Ingestion CLI](./ingestion-cli.md) 문서를 참조하세요.

## 접근 제어

MCP 서버에서 Midway 인증을 통한 접근 제어

## 기술 스택

(TBD)

## 향후 계획

(TBD)

## 기대 효과

- 검증된 코드 패턴의 재사용 증가
- 코딩 시점에서 직접 접근하여 컨텍스트 전환 비용 제거
- AI 어시스턴트가 조직의 코드 자산을 인식하고 활용할 수 있도록 지원

---

## 개발 환경

이 프로젝트는 [Nx](https://nx.dev) 워크스페이스와 [@aws/nx-plugin](https://awslabs.github.io/nx-plugin-for-aws)을 사용합니다.

### 빌드

```sh
# 단일 프로젝트 빌드
pnpm nx build <project-name>

# 전체 빌드
pnpm nx run-many --target build --all
```

### 린트

```sh
pnpm nx run-many --target lint --configuration=fix --all
```

### 테스트

```sh
pnpm nx run-many --target test --all
```
