use serde::Serialize;

/// SQS로 전송할 파일 단위 메시지.
/// Consumer Lambda가 수신하여 파일 원본 저장 + 청크별 LLM 설명 생성 + 임베딩 후 DB에 저장한다.
#[derive(Debug, Serialize)]
pub struct FileMessage {
    pub repo_id: String,
    pub commit_id: String,
    pub source_path: String,
    pub content: String,
    pub chunks: Vec<ChunkEntry>,
}

#[derive(Debug, Serialize)]
pub struct ChunkEntry {
    pub source_type: SourceType,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Code,
    Doc,
}
