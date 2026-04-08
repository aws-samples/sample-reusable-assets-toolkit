use std::path::Path;

use rat_cli::chunk::chunk_file;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn print_chunks(chunks: &[rat_cli::chunk::Chunk]) {
    for (i, c) in chunks.iter().enumerate() {
        println!(
            "--- chunk {} (L{}-L{}) {} ---",
            i + 1,
            c.start_line,
            c.end_line,
            c.symbol_name.as_deref().unwrap_or("")
        );
        if !c.imports.is_empty() {
            println!("[imports]\n{}\n", c.imports);
        }
        println!("{}", c.content);
        println!();
    }
}

#[test]
fn chunk_rust() {
    let chunks = chunk_file(&fixture("sample.rs")).unwrap();
    print_chunks(&chunks);

    // HashMap을 사용하는 청크에만 import가 포함됨
    let config_chunk = chunks.iter().find(|c| c.symbol_name.as_deref() == Some("Config")).unwrap();
    assert!(config_chunk.imports.contains("use std::collections::HashMap"));

    // HashMap을 사용하지 않는 청크에는 import가 없음
    let process_chunk = chunks.iter().find(|c| c.symbol_name.as_deref() == Some("process")).unwrap();
    assert!(process_chunk.imports.is_empty());

    // #[derive(Debug, Clone)] + struct Config
    assert!(config_chunk.content.contains("#[derive(Debug, Clone)]"));

    // impl Config
    assert!(chunks.iter().any(|c| c.content.contains("impl Config")));

    // /// doc comment + fn process
    assert!(process_chunk.content.contains("/// Process the given config"));
}

#[test]
fn chunk_typescript() {
    let chunks = chunk_file(&fixture("sample.ts")).unwrap();
    print_chunks(&chunks);

    // Request/Response를 사용하는 handler에만 import 포함
    let handler_chunk = chunks.iter().find(|c| c.content.contains("const handler")).unwrap();
    assert!(handler_chunk.imports.contains("import { Request, Response }"));

    // 사용하지 않는 청크에는 import 없음
    let format_chunk = chunks.iter().find(|c| c.content.contains("function formatUser")).unwrap();
    assert!(format_chunk.imports.is_empty());

    // @decorator + class UserService
    let service_chunk = chunks.iter().find(|c| c.content.contains("class UserService")).unwrap();
    assert!(service_chunk.content.contains("@Injectable()"));
}

#[test]
fn chunk_python() {
    let chunks = chunk_file(&fixture("sample.py")).unwrap();
    print_chunks(&chunks);

    // @dataclass를 사용하는 청크에만 import 포함
    let config_chunk = chunks.iter().find(|c| c.content.contains("@dataclass")).unwrap();
    assert!(config_chunk.imports.contains("from dataclasses import dataclass"));

    // dataclass를 사용하지 않는 청크에는 해당 import 없음
    let processor_chunk = chunks.iter().find(|c| c.content.contains("class Processor")).unwrap();
    assert!(!processor_chunk.imports.contains("dataclass"));

    // def create_processor
    assert!(chunks.iter().any(|c| c.symbol_name.as_deref() == Some("create_processor")));
}

#[test]
fn chunk_go() {
    let chunks = chunk_file(&fixture("sample.go")).unwrap();
    print_chunks(&chunks);

    // fmt를 사용하는 String 메서드에만 import 포함
    let string_chunk = chunks.iter().find(|c| c.symbol_name.as_deref() == Some("String")).unwrap();
    assert!(string_chunk.imports.contains("import \"fmt\""));

    // fmt를 사용하지 않는 청크에는 import 없음
    let new_config_chunk = chunks.iter().find(|c| c.symbol_name.as_deref() == Some("NewConfig")).unwrap();
    assert!(new_config_chunk.imports.is_empty());

    // type Config struct
    assert!(chunks.iter().any(|c| c.content.contains("type Config struct")));
}

#[test]
fn chunk_java() {
    let chunks = chunk_file(&fixture("sample.java")).unwrap();
    print_chunks(&chunks);

    // @annotation + class が合体
    let service_chunk = chunks.iter().find(|c| c.content.contains("class UserService")).unwrap();
    assert!(service_chunk.content.contains("@SuppressWarnings"));

    // Map/HashMap을 사용하는 UserService에만 import 포함
    assert!(service_chunk.imports.contains("import java.util.Map"));
    assert!(service_chunk.imports.contains("import java.util.HashMap"));

    // Map/HashMap을 사용하지 않는 청크에는 import 없음
    let status_chunk = chunks.iter().find(|c| c.content.contains("enum Status")).unwrap();
    assert!(status_chunk.imports.is_empty());

    // interface Repository
    assert!(chunks.iter().any(|c| c.content.contains("interface Repository")));
}

#[test]
fn chunk_javascript() {
    let chunks = chunk_file(&fixture("sample.js")).unwrap();
    print_chunks(&chunks);

    // express를 사용하는 createApp에만 require import 포함
    let create_app_chunk = chunks.iter().find(|c| c.content.contains("function createApp")).unwrap();
    assert!(create_app_chunk.imports.contains("require('express')"));

    // express를 사용하지 않는 청크에는 import 없음
    let handler_chunk = chunks.iter().find(|c| c.content.contains("const handler")).unwrap();
    assert!(handler_chunk.imports.is_empty());

    // class Router
    assert!(chunks.iter().any(|c| c.content.contains("class Router")));
}
