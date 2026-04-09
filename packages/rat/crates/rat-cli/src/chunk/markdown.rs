use std::path::Path;

use anyhow::Context;

use super::Chunk;

const MAX_LINES: usize = 200;

/// Markdown 파일을 `##` 단위로 청킹한다.
/// 200줄을 넘으면 `###` 단위로 분할한다.
pub fn chunk_markdown(path: &Path) -> anyhow::Result<Vec<Chunk>> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let path_str = path.to_string_lossy();
    let sections = split_by_heading(&source, 2);

    let mut chunks = Vec::new();
    for section in sections {
        let line_count = section.end_line - section.start_line + 1;
        if line_count > MAX_LINES {
            // 하위 헤딩(###)으로 재분할
            let sub_sections = split_by_heading(&section.content, 3);
            for sub in sub_sections {
                chunks.push(Chunk {
                    source_path: path_str.to_string(),
                    imports: String::new(),
                    content: sub.content,
                    symbol_name: sub.heading,
                    start_line: section.start_line + sub.start_line - 1,
                    end_line: section.start_line + sub.end_line - 1,
                });
            }
        } else {
            chunks.push(Chunk {
                source_path: path_str.to_string(),
                imports: String::new(),
                content: section.content,
                symbol_name: section.heading,
                start_line: section.start_line,
                end_line: section.end_line,
            });
        }
    }

    Ok(chunks)
}

struct Section {
    heading: Option<String>,
    content: String,
    start_line: usize,
    end_line: usize,
}

/// 지정된 헤딩 레벨로 분할한다.
/// level=2 → `##`에서 분할, level=3 → `###`에서 분할
fn split_by_heading(source: &str, level: usize) -> Vec<Section> {
    let prefix = "#".repeat(level);
    let mut sections = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_start: usize = 1;

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;

        if is_heading_at_level(line, &prefix) {
            if !current_lines.is_empty() {
                let content = current_lines.join("\n").trim().to_string();
                if !content.is_empty() {
                    sections.push(Section {
                        heading: current_heading.take(),
                        content,
                        start_line: current_start,
                        end_line: line_num - 1,
                    });
                }
            }

            current_lines.clear();
            current_lines.push(line);
            current_heading = Some(heading_text(line).to_string());
            current_start = line_num;
        } else {
            current_lines.push(line);
        }
    }

    if !current_lines.is_empty() {
        let content = current_lines.join("\n").trim().to_string();
        if !content.is_empty() {
            sections.push(Section {
                heading: current_heading.take(),
                content,
                start_line: current_start,
                end_line: source.lines().count(),
            });
        }
    }

    sections
}

/// `## Foo`는 level=2에서 매칭, `### Bar`는 level=3에서 매칭.
/// `### Bar`는 level=2에서는 매칭하지 않는다 (상위 섹션에 포함).
/// `# Foo` (level 1)는 level=2에서도 매칭한다 (최상위 헤딩).
fn is_heading_at_level(line: &str, prefix: &str) -> bool {
    if !line.starts_with('#') {
        return false;
    }
    let hashes = line.len() - line.trim_start_matches('#').len();
    let target_level = prefix.len();
    // 타겟 레벨 이하(상위 포함)의 헤딩에서 분할
    hashes <= target_level && line[hashes..].starts_with(' ')
}

fn heading_text(line: &str) -> &str {
    line.trim_start_matches('#').trim()
}
