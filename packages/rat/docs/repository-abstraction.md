# Repository Abstraction

현재 RAT은 Git 저장소만 지원합니다. 향후 일반 디렉토리도 지원하려면 아래와 같은 추상화가 필요합니다.

## 설계 방향

`Repository` trait을 도입하여 저장소 타입별 구현을 분리합니다.

```rust
pub trait Repository {
    /// 인덱싱 대상 파일 경로 목록을 반환합니다.
    fn list_files(&self) -> anyhow::Result<Vec<PathBuf>>;

    /// 파일 내용을 읽어 반환합니다.
    fn read_file(&self, path: &Path) -> anyhow::Result<Vec<u8>>;
}
```

## 구현 비교

| 타입 | 구현 | 파일 목록 전략 |
|------|------|---------------|
| Git | `git2::Repository` | `index.iter()` — `.gitignore`가 이미 반영된 tracked files |
| Plain directory | `walkdir` 등 | 프로젝트 구조별 ignore 패턴 필요 |

### Plain directory의 ignore 전략

Git 저장소는 `.gitignore`로 ignore 규칙이 명확하지만, 일반 디렉토리는 프로젝트 구조에 따라 ignore 대상이 달라집니다.

- **Node.js**: `node_modules/`, `dist/`, `.next/`
- **Rust**: `target/`
- **Python**: `__pycache__/`, `.venv/`, `*.pyc`
- **Java/Kotlin**: `build/`, `.gradle/`

프로젝트 타입을 감지하여(예: `package.json` 존재 → Node.js, `Cargo.toml` 존재 → Rust) 적절한 ignore 패턴을 적용하는 전략이 필요합니다.

## 적용 단계

1. `rat-core`에 `Repository` trait 정의
2. 기존 `git::list_files`를 `GitRepository` struct로 리팩토링하여 trait 구현
3. `PlainDirectory` 구현 — 프로젝트 타입 감지 및 ignore 패턴 적용
4. CLI에서 target 경로를 분석하여 적절한 구현체를 선택하는 팩토리 함수 작성

## 저장소 타입 감지

```rust
fn detect_repository(path: &Path) -> anyhow::Result<Box<dyn Repository>> {
    if path.join(".git").exists() {
        Ok(Box::new(GitRepository::open(path)?))
    } else {
        Ok(Box::new(PlainDirectory::new(path)?))
    }
}
```
