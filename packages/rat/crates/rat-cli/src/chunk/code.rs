use std::path::Path;

use anyhow::Context;
use tree_sitter::{Node, Parser};

use super::Chunk;

pub trait Language {
    fn ts_language(&self) -> tree_sitter::Language;

    /// 청크로 추출할 최상위 노드 종류
    fn target_nodes(&self) -> &'static [&'static str];
    /// import/use 문으로 취급할 노드 종류
    fn import_nodes(&self) -> &'static [&'static str];
    /// 선언 위에 붙는 어트리뷰트/데코레이터 노드 종류 (다음 선언과 합쳐짐)
    fn attribute_nodes(&self) -> &'static [&'static str];
    /// 내부에 타겟 노드를 감싸는 래퍼 노드 종류 (예: export_statement)
    fn wrapper_nodes(&self) -> &'static [&'static str];
    /// 나머지 청크 수집 시 무시할 노드 종류
    fn skip_nodes(&self) -> &'static [&'static str];

    /// import 여부 판별 (기본: import_nodes 매칭, JS require 등은 오버라이드)
    fn is_import(&self, node: &Node, _source: &str) -> bool {
        self.import_nodes().contains(&node.kind())
    }

    /// import 노드에서 참조 심볼 이름들을 추출
    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String>;
}

struct ImportEntry {
    text: String,
    symbols: Vec<String>,
}

pub fn chunk_code(path: &Path, lang: Box<dyn Language>) -> anyhow::Result<Vec<Chunk>> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.ts_language())
        .context("failed to set parser language")?;

    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let tree = parser
        .parse(&source, None)
        .context("tree-sitter parse returned None")?;

    let root = tree.root_node();
    let path_str = path.to_string_lossy();
    let import_entries = collect_imports(lang.as_ref(), &root, &source);
    let mut chunks = extract_top_level_chunks(lang.as_ref(), &root, &source, &import_entries, &path_str);

    let remaining = collect_remaining(lang.as_ref(), &root, &source, &chunks, &import_entries, &path_str);
    chunks.extend(remaining);

    chunks.sort_by_key(|c| c.start_line);
    Ok(chunks)
}

fn collect_imports(lang: &dyn Language, root: &Node, source: &str) -> Vec<ImportEntry> {
    let mut cursor = root.walk();
    root.children(&mut cursor)
        .filter(|child| lang.is_import(child, source))
        .map(|child| ImportEntry {
            text: source[child.byte_range()].to_string(),
            symbols: lang.import_symbols(&child, source),
        })
        .collect()
}

fn filter_imports(imports: &[ImportEntry], content: &str) -> String {
    imports
        .iter()
        .filter(|entry| entry.symbols.iter().any(|sym| content.contains(sym.as_str())))
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn make_chunk(source_path: &str, content: String, symbol_name: Option<String>, start_line: usize, end_line: usize, imports: &[ImportEntry]) -> Chunk {
    Chunk {
        source_path: source_path.to_string(),
        imports: filter_imports(imports, &content),
        content,
        symbol_name,
        start_line,
        end_line,
    }
}

fn extract_top_level_chunks(
    lang: &dyn Language,
    root: &Node,
    source: &str,
    imports: &[ImportEntry],
    source_path: &str,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut cursor = root.walk();
    let children: Vec<_> = root.children(&mut cursor).collect();
    let target_types = lang.target_nodes();

    let mut i = 0;
    while i < children.len() {
        let child = &children[i];

        if lang.is_import(child, source) {
            // import는 스킵 (imports 필드로 별도 수집됨)
        } else if target_types.contains(&child.kind()) {
            let attr_start = find_leading_attributes(lang.attribute_nodes(), &children, i);
            let start_node = &children[attr_start];
            let content = source[start_node.byte_range().start..child.byte_range().end].to_string();
            let symbol_name = extract_symbol_name(child, source);
            chunks.push(make_chunk(source_path, content, symbol_name, start_node.start_position().row + 1, child.end_position().row + 1, imports));
        } else if lang.wrapper_nodes().contains(&child.kind()) {
            let mut inner_cursor = child.walk();
            let mut found_inner = false;
            for inner in child.children(&mut inner_cursor) {
                if target_types.contains(&inner.kind()) {
                    let content = source[child.byte_range()].to_string();
                    let symbol_name = extract_symbol_name(&inner, source);
                    chunks.push(make_chunk(source_path, content, symbol_name, child.start_position().row + 1, child.end_position().row + 1, imports));
                    found_inner = true;
                    break;
                }
            }
            if !found_inner {
                let content = source[child.byte_range()].to_string();
                chunks.push(make_chunk(source_path, content, None, child.start_position().row + 1, child.end_position().row + 1, imports));
            }
        }

        i += 1;
    }

    chunks
}

fn find_leading_attributes(attr_nodes: &[&str], children: &[Node], target_idx: usize) -> usize {
    let mut start = target_idx;
    while start > 0 {
        if attr_nodes.contains(&children[start - 1].kind()) {
            start -= 1;
        } else {
            break;
        }
    }
    start
}

fn extract_symbol_name<'a>(node: &Node<'a>, source: &'a str) -> Option<String> {
    let name_node = node.child_by_field_name("name")?;
    Some(source[name_node.byte_range()].to_string())
}

fn collect_remaining(
    lang: &dyn Language,
    root: &Node,
    source: &str,
    chunks: &[Chunk],
    imports: &[ImportEntry],
    source_path: &str,
) -> Vec<Chunk> {
    let mut remaining = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        let start = child.start_position().row + 1;
        let end = child.end_position().row + 1;

        let covered = chunks
            .iter()
            .any(|c| c.start_line <= start && c.end_line >= end);
        if covered {
            continue;
        }

        if lang.skip_nodes().contains(&child.kind()) || lang.is_import(&child, source) {
            continue;
        }

        let content = source[child.byte_range()].trim();
        if content.is_empty() {
            continue;
        }

        remaining.push(make_chunk(source_path, content.to_string(), None, start, end, imports));
    }

    merge_and_split_remaining(remaining)
}

fn merge_and_split_remaining(chunks: Vec<Chunk>) -> Vec<Chunk> {
    if chunks.is_empty() {
        return chunks;
    }

    let mut iter = chunks.into_iter();
    let mut groups: Vec<Chunk> = vec![iter.next().unwrap()];

    for chunk in iter {
        let last = groups.last().unwrap();
        if chunk.start_line <= last.end_line + 2 {
            let last = groups.last_mut().unwrap();
            last.content = format!("{}\n{}", last.content, chunk.content);
            last.end_line = chunk.end_line;
        } else {
            groups.push(chunk);
        }
    }

    let mut result = Vec::new();
    for chunk in groups {
        let line_count = chunk.end_line - chunk.start_line + 1;
        if line_count <= 200 {
            result.push(chunk);
            continue;
        }

        let mut current_lines = Vec::new();
        let mut current_start = chunk.start_line;

        for (i, line) in chunk.content.lines().enumerate() {
            let is_blank = line.trim().is_empty();
            let accumulated = current_lines.len();

            if is_blank && accumulated >= 50 {
                result.push(Chunk {
                    source_path: chunk.source_path.clone(),
                    imports: chunk.imports.clone(),
                    content: current_lines.join("\n"),
                    symbol_name: None,
                    start_line: current_start,
                    end_line: chunk.start_line + i - 1,
                });
                current_lines.clear();
                current_start = chunk.start_line + i + 1;
            } else {
                current_lines.push(line.to_string());
            }
        }

        if !current_lines.is_empty() {
            result.push(Chunk {
                source_path: chunk.source_path.clone(),
                imports: chunk.imports.clone(),
                content: current_lines.join("\n"),
                symbol_name: None,
                start_line: current_start,
                end_line: chunk.end_line,
            });
        }
    }

    result
}
