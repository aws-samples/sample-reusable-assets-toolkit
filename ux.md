# rat 검색 UI — UX 설계

grep.app의 **골조**를 차용하되, rat이 하이브리드 검색(FTS + 벡터)·재사용 자산 저장소라는 특성을 살려 재구성한다.

## 1. 설계 원칙

1. **터미널의 웹 번역** — 무채색, 모노스페이스, 그림자·그라디언트 없음, 보더 기반 구분.
2. **장식 0, 밀도 高** — 여백으로 리듬을 주되 "꾸밈" 요소는 넣지 않는다.
3. **"왜 매치됐는지"를 항상 보여준다** — 하이브리드 검색은 단순 문자열 하이라이트만으로 이유를 전달 못 한다. `description` 한 줄을 1급 시민으로 둔다.
4. **검색자와 탐색자를 모두 지원** — grep.app는 검색어가 명확한 사람만 상정. rat은 "뭐가 있는지 모름" 상태의 사용자도 많다. 빈 상태에 디스커버리 슬롯을 둔다.
5. **코드와 문서를 동등하게 렌더** — `source_type`이 `code`면 모노스페이스 코드 카드, `doc`이면 산문 카드로 다르게 그린다.

## 2. 정보 구조 (페이지)

- **`/` (랜딩)** — 빈 상태. 중앙 검색바 + 디스커버리 섹션.
- **`/search?q=...&mode=...&repo=...&...`** — 결과 페이지. 좌측 필터 + 우측 결과 스트림.
- **`/repo/:repo_id`** *(2차 목표)* — 레포 상세. description, 최근 스니펫, 내부 검색.

URL 상태(쿼리, 모드, 필터, 정렬, 페이지)는 전부 query string에 저장해 공유·히스토리에 친화적으로.

## 3. 헤더

```
▲  rat                    [검색바 ......................  ⌨ L S H]   [Feedback]
```

- 좌측 브랜드: 심볼 + `rat` 워드마크. grep.app의 `▲ / grep` 체이닝은 쓰지 않는다 (Vercel 브랜드가 아님).
- 결과 페이지에서 검색바가 헤더로 승격 (grep.app 동일 거동).
- 검색바 내부 우측 토글: **L(exical) / S(emantic) / H(ybrid)** 단일 세그먼트. grep.app의 `Aa / ab / .*` 자리를 대체.

## 4. 검색 입력

**중앙 배치(랜딩) → 헤더로 이동(결과)**. grep.app 거동 그대로.

- 플레이스홀더: 자연어 예시 회전 — `"SQS publisher", "auth middleware", "pgvector query"`.
- 검색 모드 3종:
  | 모드 | 동작 | 기본 |
  |---|---|---|
  | Lexical | FTS only (`websearch_to_tsquery`) | |
  | Semantic | 벡터만 | |
  | Hybrid | RRF 퓨전 (현재 API 기본값) | ✅ |
- 엔터 시 `/search`로 전이. 입력 중에는 디바운스 후 자동 갱신.

## 5. 좌측 필터 패널

grep.app 구조 차용, 섹션만 rat에 맞게 조정.

| 섹션 | 내용 | 비고 |
|---|---|---|
| **Source** | `Code` / `Doc` 세그먼트 토글 | rat 고유. 한 번에 하나 선택 or 둘 다. |
| **Repository** | 검색 가능한 리스트. 항목당 `repo_id`, **description 한 줄**, 매치 카운트. | grep.app는 로고지만 내부 repo는 description이 더 유용. |
| **Language** | 언어별 카운트 리스트. | `SnippetRow.language`로 집계. |
| **Path** | 접두사 필터 입력(`src/`, `packages/`) + 상위 N개 프리셋. | grep.app과 동일. |

- 선택 상태는 칩/체크로 표시, 섹션 상단에 "Clear" 링크.
- 각 섹션은 접힘 가능. 초기엔 Source·Repository만 펼침.

## 6. 결과 카드

### 공통 헤더
```
repo_id  source/path/to/file.rs            [N matches]   [score ▸]
```
- `repo_id`: bold. 클릭 시 repo 상세(2차).
- path: 회색. 클릭 시 파일 원본(권한 있으면).
- 우측: 파일 내 매치 수 (API 확장 필요) · score 페이지는 debug 토글 시에만.

### `Code` 카드
```
[description 한 줄 — LLM 요약]

 12 |   pub async fn publish(...) {
 13 |       let msg = SqsMessage::new(...);    ← 매치 하이라이트 옅은 블루
 14 |       client.send(msg).await?;
 15 |   }
```
- 상단 1줄 description (이탤릭·회색). **벡터 매치의 "왜"를 여기서 설명.**
- 아래 코드 스니펫: 라인 번호, 신택스 하이라이팅, 매치 부위는 옅은 강조 배경.
- Lexical 모드: 매치 하이라이트 강화.
- Semantic 모드: 매치 하이라이트 생략, description 강조.
- Hybrid: 둘 다.

### `Doc` 카드
```
[description 한 줄]

>  본문 발췌 (proportional, serif-adjacent).
>  몇 줄 프리뷰 — 코드 카드와 구분되도록 레이아웃을 산문형으로.
```
- 모노스페이스·라인 번호 쓰지 않음.
- 인용 바(border-left) 한 줄로 "문서" 감 전달.

### Compact / Expanded
grep.app 그대로 차용. Compact는 description + 매치된 라인 ±1, Expanded는 ±N.

## 7. 빈 상태 / 디스커버리 (랜딩)

grep.app엔 없는 섹션. 재사용 자산 저장소 UX의 핵심.

```
                    [큰 검색바]

                    ─ examples ─
   [SQS publisher] [auth middleware] [pgvector query]

                    ─ browse ─
   Recently indexed
     · repo-a   — description...      120 snippets
     · repo-b   — description...       84 snippets
     · ...

   All repositories →
```

- 예시 쿼리 칩: 클릭 시 그대로 검색 실행.
- Recently indexed: `RepoGet` / `List`에서 가져옴 (필요시 `updated_at` 정렬 추가).
- "All repositories" → 별도 페이지 또는 좌측 패널만 있는 결과 페이지로 이동.

## 8. 상태

| 상태 | 화면 |
|---|---|
| 로딩 | 결과 영역 상단에 얇은 인디케이터 스트라이프 (1~2px). 스피너 쓰지 않음. |
| 결과 없음 | "No matches. Try Semantic mode, or broaden filters." 모드 스위치 강조. |
| 에러 | "Search failed: {메시지}." + Retry. 스택 노출 안 함. |
| 필터만 있고 쿼리 없음 | 해당 필터 조건의 repo/파일 리스트 (브라우저 모드). |

## 9. 비주얼 언어

- **컬러**: 무채색 계조. Light·Dark 대응. primary accent는 단 하나 (매치 하이라이트에만), 나머지는 회색·검정·흰색.
- **타이포**: 본문 system sans (`ui-sans-serif`), 코드/심볼 monospace (`ui-monospace`).
- **보더**: 1px 저채도 선으로 구분. shadow·radius 최소.
- **여백**: 랜딩은 중앙 검색바가 유일한 시각 앵커가 되도록 여백을 크게.
- **아이콘**: 텍스트 심볼 선호(`L·S·H`, `⌨`, `↵`). SVG 아이콘은 필수 자리에만.

## 10. grep.app 차용 vs 재구성 표

| 요소 | 판단 | 비고 |
|---|---|---|
| 무채색 팔레트 | **Adopt** | 그대로. |
| 모노스페이스 중심 코드 렌더 | **Adopt** | `Code` 카드에 한정. |
| 좌측 필터 패널 구조 | **Adopt** | 섹션은 rat에 맞게 교체. |
| 검색바 → 헤더 승격 전환 | **Adopt** | 그대로. |
| 결과 카드: repo/path 헤더 + 스니펫 | **Adopt** | 상단 description 1줄 추가. |
| Compact / Expanded 토글 | **Adopt** | 그대로. |
| `Aa / ab / .*` 토글 3종 | **Replace** | `L / S / H` 모드 스위치로 교체. |
| "X,000,000 results found" 카운트 | **Drop** | 내부 스케일에 어울리지 않음. `N repositories · M snippets`로. |
| 헤드라인 모션블러 `fast` 연출 | **Drop** | grep.app의 시그니처. 복제하면 흉내가 됨. 자체 톤 확립. |
| 결과 내 매치 하이라이트 | **Adapt** | Lexical만 강조, Semantic은 description 강조. |
| 레포 항목 로고 | **Replace** | 로고 대신 description 한 줄. |
| 디스커버리 섹션 | **Add** | grep.app엔 없음. 재사용 자산 저장소 필수. |
| Source type 세그먼트 (`code`/`doc`) | **Add** | rat 고유. |

## 11. 오픈 이슈 (구현 전에 확정할 것)

- [ ] 검색 모드 기본값: `Hybrid` 확정?
- [ ] 랜딩 디스커버리에 "인기 쿼리" 노출할지 (로그 수집 필요 → 스코프 상승)
- [ ] 매치 하이라이트 오프셋을 서버에서 내려줄지(ts_headline) vs 클라에서 계산할지
- [ ] 다크 모드를 v1에 포함할지
- [ ] repo 상세 페이지를 v1에 포함할지, v2로 미룰지
