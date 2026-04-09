use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

const RATIGNORE: &str = ".ratignore";

/// .ratignore 파일을 로드하여 Gitignore matcher를 반환한다.
/// repo root와 추가 경로(서브디렉토리 등) 양쪽의 .ratignore를 모두 적용한다.
pub fn load(repo_root: &Path, extra_dirs: &[&Path]) -> Gitignore {
    let mut builder = GitignoreBuilder::new(repo_root);

    let root_ignore = repo_root.join(RATIGNORE);
    if root_ignore.exists() {
        builder.add(&root_ignore);
    }

    for dir in extra_dirs {
        let path = dir.join(RATIGNORE);
        if path.exists() {
            builder.add(&path);
        }
    }

    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

/// 파일이 .ratignore에 의해 무시되는지 확인한다.
/// path는 상대 경로/절대 경로 모두 가능.
pub fn is_ignored(gitignore: &Gitignore, path: &Path, is_dir: bool) -> bool {
    gitignore
        .matched_path_or_any_parents(path, is_dir)
        .is_ignore()
}
