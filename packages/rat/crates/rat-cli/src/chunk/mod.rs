mod code;
mod go;
mod java;
mod javascript;
mod markdown;
mod python;
mod rust;
mod typescript;

use std::path::Path;

use anyhow::Context;

#[derive(Debug)]
pub struct Chunk {
    pub source_path: String,
    pub imports: String,
    pub content: String,
    pub symbol_name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| chunker_for_ext(ext).is_some())
}

pub fn chunk_file(path: &Path) -> anyhow::Result<Vec<Chunk>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .context("cannot determine file extension")?;

    let chunker = chunker_for_ext(ext)
        .with_context(|| format!("unsupported file extension: .{ext}"))?;

    chunker(path)
}

type ChunkerFn = fn(&Path) -> anyhow::Result<Vec<Chunk>>;

fn chunker_for_ext(ext: &str) -> Option<ChunkerFn> {
    match ext {
        "rs" => Some(|p| code::chunk_code(p, Box::new(rust::Rust))),
        "js" | "mjs" | "cjs" | "jsx" => Some(|p| code::chunk_code(p, Box::new(javascript::JavaScript))),
        "ts" | "mts" | "cts" => Some(|p| code::chunk_code(p, Box::new(typescript::TypeScript))),
        "tsx" => Some(|p| code::chunk_code(p, Box::new(typescript::Tsx))),
        "py" | "pyi" => Some(|p| code::chunk_code(p, Box::new(python::Python))),
        "go" => Some(|p| code::chunk_code(p, Box::new(go::Go))),
        "java" => Some(|p| code::chunk_code(p, Box::new(java::Java))),
        "md" => Some(markdown::chunk_markdown),
        _ => None,
    }
}
