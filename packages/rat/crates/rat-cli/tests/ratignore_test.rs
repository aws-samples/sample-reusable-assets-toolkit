// SPDX-License-Identifier: MIT

use std::io::Write;
use std::path::Path;

use rat_cli::ratignore;

#[test]
fn test_ratignore_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    let ratignore_path = dir.path().join(".ratignore");
    let mut f = std::fs::File::create(&ratignore_path).unwrap();
    writeln!(f, "**/fixtures/").unwrap();

    let ignore = ratignore::load(dir.path(), &[]);

    // 相対パス
    let rel = Path::new("crates/rat-cli/tests/fixtures/sample.rs");
    let abs = dir.path().join(rel);
    println!("rel ignored: {}", ratignore::is_ignored(&ignore, rel, false));
    println!("abs ignored: {}", ratignore::is_ignored(&ignore, &abs, false));

    assert!(ratignore::is_ignored(&ignore, &abs, false));
}
