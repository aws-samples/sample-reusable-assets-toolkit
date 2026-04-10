use rat_core::summary;
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

fn sample_markdown_json() -> &'static str {
    r###"{
        "action": "upsert",
        "repo_id": "https://github.com/example/repo",
        "commit_id": "abc123def",
        "source_path": "docs/getting-started.md",
        "content": "# 시작하기\n\n이 가이드는 프로젝트 설정 과정을 안내합니다.\n\n## 사전 요구사항\n\n- Rust 1.75 이상\n- PostgreSQL 15 이상\n- AWS CLI 설정 완료\n\n## 설치\n\n```bash\ncargo install rat-cli\nrat configure\nrat login\n```\n\n## 사용법\n\n설치 후 저장소를 인덱싱합니다:\n\n```bash\nrat ingest --repo https://github.com/example/repo\nrat search \"인증은 어떻게 동작하나요?\"\n```\n",
        "chunks": [
            {
                "source_type": "doc",
                "start_line": 1,
                "end_line": 10,
                "content": "# 시작하기\n\n이 가이드는 프로젝트 설정 과정을 안내합니다.\n\n## 사전 요구사항\n\n- Rust 1.75 이상\n- PostgreSQL 15 이상\n- AWS CLI 설정 완료"
            },
            {
                "source_type": "doc",
                "start_line": 12,
                "end_line": 25,
                "content": "## 설치\n\n```bash\ncargo install rat-cli\nrat configure\nrat login\n```\n\n## 사용법\n\n설치 후 저장소를 인덱싱합니다:\n\n```bash\nrat ingest --repo https://github.com/example/repo\nrat search \"인증은 어떻게 동작하나요?\"\n```"
            }
        ]
    }"###
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
        println!("  content:\n{}", rec.content);
    }
}

#[tokio::test]
async fn test_summary_generation() {
    let model_id = "global.anthropic.claude-haiku-4-5-20251001-v1:0";
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let bedrock = aws_sdk_bedrockruntime::Client::new(&aws_config);

    let msg: FileMessage = serde_json::from_str(sample_upsert_json()).unwrap();
    let snippet_recs = build_snippet_records(&msg);

    for (i, rec) in snippet_recs.iter().enumerate() {
        let description = summary::generate_summary(&bedrock, model_id, rec.content)
            .await
            .unwrap();
        println!("--- snippet[{i}] summary ---");
        println!("  content:     {}", rec.content);
        println!("  description: {description}");
    }
}

#[test]
fn test_markdown_upsert_records() {
    let msg: FileMessage = serde_json::from_str(sample_markdown_json()).unwrap();
    let file_rec = build_file_record(&msg).unwrap();

    println!("=== File Record (markdown) ===");
    println!("repo_id:     {}", file_rec.repo_id);
    println!("source_path: {}", file_rec.source_path);
    println!("commit_id:   {}", file_rec.commit_id);
    println!("language:    {:?}", file_rec.language);
    assert_eq!(file_rec.language, Some("markdown"));

    let snippet_recs = build_snippet_records(&msg);
    println!("\n=== Snippet Records ({}) ===", snippet_recs.len());
    assert_eq!(snippet_recs.len(), 2);
    for (i, rec) in snippet_recs.iter().enumerate() {
        println!("--- snippet[{}] ---", i);
        println!("  source_type: {}", rec.source_type);
        assert_eq!(rec.source_type, "doc");
        println!("  content:\n{}", rec.content);
    }
}

#[tokio::test]
async fn test_markdown_summary_generation() {
    let model_id = "global.anthropic.claude-haiku-4-5-20251001-v1:0";
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let bedrock = aws_sdk_bedrockruntime::Client::new(&aws_config);

    let msg: FileMessage = serde_json::from_str(sample_markdown_json()).unwrap();
    let snippet_recs = build_snippet_records(&msg);

    for (i, rec) in snippet_recs.iter().enumerate() {
        let description = summary::generate_summary(&bedrock, model_id, rec.content)
            .await
            .unwrap();
        println!("--- markdown snippet[{i}] summary ---");
        println!("  content:     {}", rec.content);
        println!("  description: {description}");
    }
}

#[tokio::test]
async fn test_embedding_generation() {
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .load()
        .await;
    let bedrock = aws_sdk_bedrockruntime::Client::new(&aws_config);

    let text = "fn main() {\n    println!(\"hello\");\n}";
    let embedding = rat_core::embedding::generate_embedding(&bedrock, text, "GENERIC_INDEX")
        .await
        .unwrap();

    println!("embedding dimensions: {}", embedding.len());
    assert_eq!(embedding.len(), 1024);
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
