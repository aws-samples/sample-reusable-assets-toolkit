use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, Message, SystemContentBlock};
use aws_sdk_bedrockruntime::Client;
use tracing::info;

const SYSTEM_PROMPT: &str = "\
You are a code summarizer. Given a code snippet, produce a concise English description \
that captures what the code does, its key identifiers, and its purpose. \
The description will be used for full-text search indexing, so include relevant \
technical terms and keywords. Keep it under 200 words. Output only the description.";

pub async fn generate_summary(
    client: &Client,
    model_id: &str,
    content: &str,
) -> Result<String, aws_sdk_bedrockruntime::Error> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    info!(content_len = trimmed.len(), model_id, "Generating summary");

    let response = client
        .converse()
        .model_id(model_id)
        .system(SystemContentBlock::Text(SYSTEM_PROMPT.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(trimmed.to_string()))
                .build()
                .unwrap(),
        )
        .send()
        .await?;

    let text = response
        .output()
        .and_then(|o| o.as_message().ok())
        .and_then(|m| m.content().first())
        .and_then(|c| c.as_text().ok())
        .cloned()
        .unwrap_or_default();

    info!(summary_len = text.len(), "Summary generated");
    Ok(text)
}
