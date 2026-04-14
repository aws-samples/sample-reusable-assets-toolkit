pub use rat_core::message::*;

pub fn language_from_path(path: &str) -> Option<&str> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rs" => Some("rust"),
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "ts" | "mts" | "cts" | "tsx" => Some("typescript"),
        "py" | "pyi" => Some("python"),
        "go" => Some("go"),
        "java" => Some("java"),
        "md" => Some("markdown"),
        _ => None,
    }
}

pub struct FileRecord<'a> {
    pub repo_id: &'a str,
    pub source_path: &'a str,
    pub content: &'a str,
    pub language: Option<&'a str>,
}

pub struct SnippetRecord<'a> {
    pub repo_id: &'a str,
    pub content: &'a str,
    pub source_type: &'a str,
    pub start_line: i32,
    pub end_line: i32,
}

pub fn build_file_record(msg: &FileMessage) -> Result<FileRecord<'_>, &'static str> {
    let source_path = msg.source_path.as_deref().ok_or("upsert requires source_path")?;
    let content = msg.content.as_deref().ok_or("upsert requires content")?;
    let language = language_from_path(source_path);
    Ok(FileRecord {
        repo_id: &msg.repo_id,
        source_path,
        content,
        language,
    })
}

pub fn build_snippet_records(msg: &FileMessage) -> Vec<SnippetRecord<'_>> {
    msg.chunks
        .iter()
        .map(|chunk| {
            SnippetRecord {
                repo_id: &msg.repo_id,
                content: &chunk.content,
                source_type: chunk.source_type.as_str(),
                start_line: chunk.start_line as i32,
                end_line: chunk.end_line as i32,
            }
        })
        .collect()
}
