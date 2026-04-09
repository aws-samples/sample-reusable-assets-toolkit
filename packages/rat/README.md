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
├── repo_id: String          // git remote URL
├── source_path: String      // 레포 내 상대 경로
├── content: String          // 파일 전체 원본
└── chunks: Vec<ChunkEntry>  // 청크 목록
    ├── source_type: code | doc
    ├── start_line / end_line
    └── content: String      // imports + 청크 코드
```

Consumer Lambda 처리:
1. `files` 테이블에 파일 원본 저장
2. 청크 목록을 파일 컨텍스트와 함께 LLM에 전달 → 설명 생성
3. `snippets` 테이블에 청크별 저장 + 임베딩

## Crates

| Crate | 설명 |
|-------|------|
| `rat-core` | 핵심 로직 (git 연동, DB 모델) |
| `rat-cli` | CLI 인터페이스 |
| `rat-lambda` | AWS Lambda 핸들러 |
