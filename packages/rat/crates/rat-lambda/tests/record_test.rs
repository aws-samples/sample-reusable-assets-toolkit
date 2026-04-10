use rat_lambda::{build_file_record, build_snippet_records, FileMessage};

fn sample_upsert_json() -> &'static str {
    r#"{
        "action": "upsert",
        "repo_id": "https://github.com/example/repo",
        "commit_id": "abc123def",
        "source_path": "src/main.rs",
        "content": "use std::io;\n\nfn main() {\n    println!(\"hello\");\n}\n\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        "chunks": [
            {
                "source_type": "code",
                "start_line": 3,
                "end_line": 5,
                "content": "use std::io;\n\nfn main() {\n    println!(\"hello\");\n}"
            },
            {
                "source_type": "code",
                "start_line": 7,
                "end_line": 9,
                "content": "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}"
            }
        ]
    }"#
}

fn sample_delete_json() -> &'static str {
    r#"{
        "action": "delete",
        "repo_id": "https://github.com/example/repo",
        "commit_id": "abc123def",
        "source_path": "src/old.rs"
    }"#
}

fn sample_purge_json() -> &'static str {
    r#"{
        "action": "purge",
        "repo_id": "https://github.com/example/repo",
        "commit_id": "abc123def"
    }"#
}

#[test]
fn test_upsert_records() {
    let msg: FileMessage = serde_json::from_str(sample_upsert_json()).unwrap();
    let file_rec = build_file_record(&msg).unwrap();

    println!("=== File Record ===");
    println!("repo_id:     {}", file_rec.repo_id);
    println!("source_path: {}", file_rec.source_path);
    println!("commit_id:   {}", file_rec.commit_id);
    println!("language:    {:?}", file_rec.language);
    println!("content:     ({} bytes)", file_rec.content.len());

    let snippet_recs = build_snippet_records(&msg);
    println!("\n=== Snippet Records ({}) ===", snippet_recs.len());
    for (i, rec) in snippet_recs.iter().enumerate() {
        println!("--- snippet[{}] ---", i);
        println!("  repo_id:     {}", rec.repo_id);
        println!("  source_type: {}", rec.source_type);
        println!("  start_line:  {}", rec.start_line);
        println!("  end_line:    {}", rec.end_line);
        println!("  description: {}", rec.description);
        println!("  content:\n{}", rec.content);
    }
}

#[test]
fn test_delete_record() {
    let msg: FileMessage = serde_json::from_str(sample_delete_json()).unwrap();

    println!("=== Delete ===");
    println!("repo_id:     {}", msg.repo_id);
    println!("source_path: {:?}", msg.source_path);
    println!("chunks:      {} (should be 0)", msg.chunks.len());
}

#[test]
fn test_purge_record() {
    let msg: FileMessage = serde_json::from_str(sample_purge_json()).unwrap();

    println!("=== Purge ===");
    println!("repo_id:     {}", msg.repo_id);
    println!("source_path: {:?} (should be None)", msg.source_path);
    println!("chunks:      {} (should be 0)", msg.chunks.len());
}
