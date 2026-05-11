// SPDX-License-Identifier: MIT

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

const REPO_SYSTEM_PROMPT: &str = "\
You summarize git repositories. Given a README, output a clean 2-3 sentence description \
of what the repository is and what it does.\n\
\n\
Rules:\n\
- Present tense.\n\
- Mention the main purpose/domain and key technologies.\n\
- 2-3 sentences, 30-80 words.\n\
- No preamble, no markdown, no labels, no quotes, no code fences.\n\
- Base the description only on the README content provided.\n\
- Do not speculate beyond what is in the README.";

/// Generate a plain-text repository description from README content.
pub async fn generate_repo_description(
    client: &Client,
    model_id: &str,
    repo_id: &str,
    readme: &str,
) -> Result<String, aws_sdk_bedrockruntime::Error> {
    let trimmed = readme.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    info!(
        content_len = trimmed.len(),
        model_id, repo_id, "Generating repo description"
    );

    let mut user_message = String::new();
    let _ = writeln!(user_message, "Repository: {}", repo_id);
    user_message.push_str("\n--- README ---\n");
    user_message.push_str(trimmed);

    let response = client
        .converse()
        .model_id(model_id)
        .system(SystemContentBlock::Text(REPO_SYSTEM_PROMPT.to_string()))
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

    info!(description_len = text.len(), "Repo description generated");
    Ok(text.trim().to_string())
}
