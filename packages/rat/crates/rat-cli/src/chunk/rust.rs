use tree_sitter::Node;

use super::code::Language;

pub struct Rust;

impl Language for Rust {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        &[
            "function_item",
            "impl_item",
            "struct_item",
            "enum_item",
            "trait_item",
            "macro_definition",
            "type_item",
        ]
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        &["use_declaration"]
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        &["attribute_item", "inner_attribute_item", "line_comment", "block_comment"]
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &[]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        &[
            "line_comment",
            "block_comment",
            "attribute_item",
            "inner_attribute_item",
            "const_item",
            "static_item",
        ]
    }

    // use std::collections::HashMap; → ["HashMap"]
    // use anyhow::{bail, Context}; → ["bail", "Context"]
    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        collect_use_symbols(node, source, &mut symbols);
        symbols
    }
}

fn collect_use_symbols(node: &Node, source: &str, symbols: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "type_identifier" => {
            symbols.push(source[node.byte_range()].to_string());
        }
        "use_as_clause" => {
            // use Foo as Bar → "Bar"
            if let Some(alias) = node.child_by_field_name("alias") {
                symbols.push(source[alias.byte_range()].to_string());
            }
        }
        "scoped_identifier" => {
            // use std::collections::HashMap → last segment "HashMap"
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(source[name.byte_range()].to_string());
            }
        }
        "use_wildcard" => {
            // use foo::* → wildcard, include everything
            symbols.push("*".to_string());
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_use_symbols(&child, source, symbols);
            }
        }
    }
}
