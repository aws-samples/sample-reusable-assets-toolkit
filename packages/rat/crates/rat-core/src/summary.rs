use std::fmt::Write;

use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, Message, SystemContentBlock};
use aws_sdk_bedrockruntime::Client;
use tracing::info;

pub struct SummaryContext<'a> {
    pub source_path: &'a str,
    pub language: Option<&'a str>,
    pub source_type: &'a str,
}

const SYSTEM_PROMPT: &str = "\
You are a code summarizer. Given a code snippet, output a description in EXACTLY the \
following plain-text format, with no markdown, no code fences, no extra lines, and no \
preamble:\n\
\n\
SUMMARY: <one sentence, 15-25 words, present tense, describing what the code does>\n\
IDENTIFIERS: <comma-separated function, type, method, and module names defined or \
directly referenced; lowercase preserved as in source; omit language keywords>\n\
KEYWORDS: <comma-separated technical terms, libraries, domain concepts, and operations \
present in the code; lowercase; 5-15 items>\n\
\n\
Rules:\n\
- Output all three lines, in this exact order, with the exact labels shown.\n\
- Do not speculate about use cases or intent that are not evident in the code.\n\
- Do not mention that the input is a snippet or that you are summarizing.\n\
- Do not wrap output in quotes or backticks.";

pub async fn generate_summary(
    client: &Client,
    model_id: &str,
    content: &str,
    ctx: &SummaryContext<'_>,
) -> Result<String, aws_sdk_bedrockruntime::Error> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    info!(
        content_len = trimmed.len(),
        model_id,
        source_path = ctx.source_path,
        "Generating summary"
    );

    let mut user_message = String::new();
    let _ = writeln!(user_message, "File: {}", ctx.source_path);
    if let Some(lang) = ctx.language {
        let _ = writeln!(user_message, "Language: {}", lang);
    }
    let _ = writeln!(user_message, "Type: {}", ctx.source_type);
    user_message.push_str("\n---\n");
    user_message.push_str(trimmed);

    let response = client
        .converse()
        .model_id(model_id)
        .system(SystemContentBlock::Text(SYSTEM_PROMPT.to_string()))
        .messages(
            Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(user_message))
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
