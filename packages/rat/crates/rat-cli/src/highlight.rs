//! Terminal syntax highlighting via tree-sitter-highlight.
//!
//! Given source text and a language key, emits ANSI-colored text.
//! Unknown languages pass through unchanged.
//!
//! Nested highlights are handled via a style stack: on `HighlightEnd` the
//! parent's style is re-emitted so we don't lose outer styling when an
//! inner highlight closes.

use std::collections::HashMap;
use std::sync::OnceLock;

use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

/// Capture names we recognize. Order matters — the index returned by
/// `HighlightStart` is an index into this slice. More specific names
/// (e.g. `function.method`) should come after their less specific
/// counterpart (`function`) so tree-sitter picks the specific one.
const HIGHLIGHT_NAMES: &[&str] = &[
    // metadata
    "attribute",
    "comment",
    // literals
    "boolean",
    "character",
    "number",
    "string",
    "string.escape",
    "string.regex",
    "string.special",
    // constants
    "constant",
    "constant.builtin",
    "constructor",
    // functions
    "function",
    "function.builtin",
    "function.call",
    "function.macro",
    "function.method",
    // keywords
    "keyword",
    "keyword.control",
    "keyword.function",
    "keyword.operator",
    "keyword.return",
    // types
    "type",
    "type.builtin",
    // variables
    "variable",
    "variable.builtin",
    "variable.parameter",
    // structure
    "namespace",
    "label",
    "property",
    "tag",
    "tag.delimiter",
    // operators / punctuation
    "operator",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    // preprocessor / imports
    "embedded",
    "escape",
    "include",
    "preproc",
    // markup
    "markup.heading",
    "markup.list",
    "markup.bold",
    "markup.italic",
    "markup.link",
    "markup.link.url",
    "markup.raw",
    "markup.quote",
    // errors
    "error",
];

// ── ANSI helpers ────────────────────────────────────────────────

const RESET: &str = "\x1b[0m";

#[derive(Clone, Copy, Default)]
struct Style {
    /// 256-color foreground escape prefix (e.g. "\x1b[38;5;111m"). `None` = default.
    color: Option<&'static str>,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl Style {
    const fn fg(color: &'static str) -> Self {
        Self {
            color: Some(color),
            bold: false,
            italic: false,
            underline: false,
        }
    }

    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    fn to_ansi(self) -> String {
        let mut s = String::new();
        if self.bold {
            s.push_str("\x1b[1m");
        }
        if self.italic {
            s.push_str("\x1b[3m");
        }
        if self.underline {
            s.push_str("\x1b[4m");
        }
        if let Some(c) = self.color {
            s.push_str(c);
        }
        s
    }

    fn is_empty(self) -> bool {
        self.color.is_none() && !self.bold && !self.italic && !self.underline
    }
}

// Base colors (256-color palette, dark-terminal friendly)
const GRAY: &str = "\x1b[38;5;244m";
const RED: &str = "\x1b[38;5;203m";
const ORANGE: &str = "\x1b[38;5;216m";
const BRIGHT_ORANGE: &str = "\x1b[38;5;208m";
const YELLOW: &str = "\x1b[38;5;222m";
const SOFT_YELLOW: &str = "\x1b[38;5;180m";
const GREEN: &str = "\x1b[38;5;150m";
const CYAN: &str = "\x1b[38;5;110m";
const BLUE: &str = "\x1b[38;5;111m";
const LIGHT_BLUE: &str = "\x1b[38;5;117m";
const PURPLE: &str = "\x1b[38;5;141m";
const LIGHT_PURPLE: &str = "\x1b[38;5;176m";

fn style_for(name: &str) -> Style {
    match name {
        // metadata
        "attribute" => Style::fg(SOFT_YELLOW),
        "comment" => Style::fg(GRAY).italic(),

        // literals
        "boolean" => Style::fg(BRIGHT_ORANGE),
        "character" => Style::fg(GREEN),
        "number" => Style::fg(YELLOW),
        "string" => Style::fg(GREEN),
        "string.escape" => Style::fg(ORANGE),
        "string.regex" => Style::fg(ORANGE),
        "string.special" => Style::fg(ORANGE),

        // constants
        "constant" => Style::fg(ORANGE),
        "constant.builtin" => Style::fg(BRIGHT_ORANGE).bold(),
        "constructor" => Style::fg(ORANGE),

        // functions
        "function" => Style::fg(BLUE),
        "function.builtin" => Style::fg(LIGHT_BLUE),
        "function.call" => Style::fg(BLUE),
        "function.macro" => Style::fg(LIGHT_PURPLE),
        "function.method" => Style::fg(BLUE),

        // keywords
        "keyword" => Style::fg(PURPLE).bold(),
        "keyword.control" => Style::fg(PURPLE).bold(),
        "keyword.function" => Style::fg(PURPLE).bold(),
        "keyword.operator" => Style::fg(PURPLE),
        "keyword.return" => Style::fg(PURPLE).bold(),

        // types
        "type" => Style::fg(CYAN),
        "type.builtin" => Style::fg(CYAN).bold(),

        // variables
        "variable" => Style::default(),
        "variable.builtin" => Style::fg(BRIGHT_ORANGE),
        "variable.parameter" => Style::fg(SOFT_YELLOW),

        // structure
        "namespace" => Style::fg(CYAN),
        "label" => Style::fg(SOFT_YELLOW),
        "property" => Style::fg(SOFT_YELLOW),
        "tag" => Style::fg(PURPLE),
        "tag.delimiter" => Style::fg(GRAY),

        // operators / punctuation — default (no color) keeps code dense and readable
        "operator" => Style::default(),
        "punctuation" => Style::default(),
        "punctuation.bracket" => Style::default(),
        "punctuation.delimiter" => Style::default(),
        "punctuation.special" => Style::fg(PURPLE),

        // preprocessor / imports
        "embedded" => Style::default(),
        "escape" => Style::fg(ORANGE),
        "include" => Style::fg(PURPLE).bold(),
        "preproc" => Style::fg(PURPLE).bold(),

        // markup
        "markup.heading" => Style::fg(BLUE).bold(),
        "markup.list" => Style::fg(PURPLE),
        "markup.bold" => Style::default().bold(),
        "markup.italic" => Style::default().italic(),
        "markup.link" => Style::fg(BLUE),
        "markup.link.url" => Style::fg(CYAN),
        "markup.raw" => Style::fg(GREEN),
        "markup.quote" => Style::fg(GRAY).italic(),

        // errors
        "error" => Style::fg(RED).bold(),

        _ => Style::default(),
    }
}

// ── tree-sitter configurations ──────────────────────────────────

fn build_configs() -> HashMap<&'static str, HighlightConfiguration> {
    fn configure(
        lang: tree_sitter::Language,
        name: &str,
        highlights: &str,
        injections: &str,
        locals: &str,
    ) -> Option<HighlightConfiguration> {
        let mut cfg =
            HighlightConfiguration::new(lang, name, highlights, injections, locals).ok()?;
        cfg.configure(HIGHLIGHT_NAMES);
        Some(cfg)
    }

    let mut configs: HashMap<&'static str, HighlightConfiguration> = HashMap::new();

    if let Some(cfg) = configure(
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
        "",
    ) {
        configs.insert("rust", cfg);
    }

    if let Some(cfg) = configure(
        tree_sitter_javascript::LANGUAGE.into(),
        "javascript",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_javascript::INJECTIONS_QUERY,
        tree_sitter_javascript::LOCALS_QUERY,
    ) {
        configs.insert("javascript", cfg);
    }

    // TypeScript's HIGHLIGHTS_QUERY only contains TS-specific captures and
    // expects to inherit from JavaScript (via `; inherits: javascript`, which
    // tree-sitter-highlight does not process). Concatenate the JS query so
    // strings, comments, functions, numbers, etc. are highlighted too.
    let ts_query = format!(
        "{}\n{}",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_typescript::HIGHLIGHTS_QUERY
    );
    if let Some(cfg) = configure(
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "typescript",
        &ts_query,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
    ) {
        configs.insert("typescript", cfg);
    }

    if let Some(cfg) = configure(
        tree_sitter_python::LANGUAGE.into(),
        "python",
        tree_sitter_python::HIGHLIGHTS_QUERY,
        "",
        "",
    ) {
        configs.insert("python", cfg);
    }

    if let Some(cfg) = configure(
        tree_sitter_go::LANGUAGE.into(),
        "go",
        tree_sitter_go::HIGHLIGHTS_QUERY,
        "",
        "",
    ) {
        configs.insert("go", cfg);
    }

    if let Some(cfg) = configure(
        tree_sitter_java::LANGUAGE.into(),
        "java",
        tree_sitter_java::HIGHLIGHTS_QUERY,
        "",
        "",
    ) {
        configs.insert("java", cfg);
    }

    if let Some(cfg) = configure(
        tree_sitter_md::LANGUAGE.into(),
        "markdown",
        tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
        tree_sitter_md::INJECTION_QUERY_BLOCK,
        "",
    ) {
        configs.insert("markdown", cfg);
    }

    configs
}

fn configs() -> &'static HashMap<&'static str, HighlightConfiguration> {
    static CONFIGS: OnceLock<HashMap<&'static str, HighlightConfiguration>> = OnceLock::new();
    CONFIGS.get_or_init(build_configs)
}

/// Map a file extension to the language key used in `configs`.
pub fn language_from_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("rust"),
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "ts" | "mts" | "cts" | "tsx" => Some("typescript"),
        "py" | "pyi" => Some("python"),
        "go" => Some("go"),
        "java" => Some("java"),
        "md" | "markdown" => Some("markdown"),
        _ => None,
    }
}

/// Highlight `content` for the given language key. Returns the input unchanged
/// if the language is unknown or highlighting fails.
pub fn highlight(content: &str, language: Option<&str>) -> String {
    let Some(lang) = language else {
        return content.to_string();
    };
    let configs = configs();
    let Some(cfg) = configs.get(lang) else {
        return content.to_string();
    };

    let mut highlighter = Highlighter::new();
    let events = match highlighter.highlight(cfg, content.as_bytes(), None, |_| None) {
        Ok(it) => it,
        Err(_) => return content.to_string(),
    };

    let mut out = String::with_capacity(content.len() * 2);
    let mut stack: Vec<Style> = Vec::new();

    for event in events {
        match event {
            Ok(HighlightEvent::Source { start, end }) => {
                out.push_str(&content[start..end]);
            }
            Ok(HighlightEvent::HighlightStart(Highlight(idx))) => {
                let style = HIGHLIGHT_NAMES
                    .get(idx)
                    .map(|n| style_for(n))
                    .unwrap_or_default();
                stack.push(style);
                if !style.is_empty() {
                    out.push_str(&style.to_ansi());
                }
            }
            Ok(HighlightEvent::HighlightEnd) => {
                stack.pop();
                // Reset and re-apply the parent style (if any) so we don't
                // drop outer highlighting when an inner highlight closes.
                out.push_str(RESET);
                if let Some(parent) = stack.last() {
                    if !parent.is_empty() {
                        out.push_str(&parent.to_ansi());
                    }
                }
            }
            Err(_) => {}
        }
    }
    out
}
