use serde::Serialize;

/// SQS로 전송할 파일 단위 메시지.
/// Consumer Lambda가 수신하여 파일 원본 저장 + 청크별 LLM 설명 생성 + 임베딩 후 DB에 저장한다.
#[derive(Debug, Serialize)]
pub struct FileMessage {
    pub action: Action,
    pub repo_id: String,
    pub commit_id: String,
    /// Purge일 때는 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Upsert가 아닐 때는 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Upsert가 아닐 때는 비어있음
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chunks: Vec<ChunkEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// 파일 upsert (원본 저장 + 청크 저장)
    Upsert,
    /// 파일 삭제 (해당 source_path의 레코드 삭제)
    Delete,
    /// repo 전체 삭제 (force 재인덱싱 시)
    Purge,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChunkEntry {
    pub source_type: SourceType,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Code,
    Doc,
}
