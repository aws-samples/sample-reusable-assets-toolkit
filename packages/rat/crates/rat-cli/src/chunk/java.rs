// SPDX-License-Identifier: MIT

use tree_sitter::Node;

use super::code::Language;

pub struct Java;

impl Language for Java {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_java::LANGUAGE.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        &[
            "method_declaration",
            "class_declaration",
            "interface_declaration",
            "enum_declaration",
        ]
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        &["import_declaration"]
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        &["annotation", "line_comment", "block_comment"]
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &[]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        &[
            "line_comment",
            "block_comment",
            "package_declaration",
            "annotation",
            "field_declaration",
        ]
    }

    // import java.util.Map; → ["Map"]
    // import java.util.*; → ["*"]
    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        // import 문의 마지막 식별자 또는 asterisk를 추출
        let text = source[node.byte_range()].trim().trim_end_matches(';');
        if text.ends_with(".*") {
            return vec!["*".to_string()];
        }
        if let Some(last_dot) = text.rfind('.') {
            return vec![text[last_dot + 1..].to_string()];
        }
        vec![]
    }
}
