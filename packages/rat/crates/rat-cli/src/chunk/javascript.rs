use tree_sitter::Node;

use super::code::Language;

pub struct JavaScript;

impl Language for JavaScript {
    fn name(&self) -> &'static str {
        "javascript"
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        &[
            "function_declaration",
            "class_declaration",
            "method_definition",
            "export_statement",
            "lexical_declaration",
            "expression_statement",
        ]
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        &["import_statement"]
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &["export_statement"]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn is_import(&self, node: &Node, source: &str) -> bool {
        if self.import_nodes().contains(&node.kind()) {
            return true;
        }
        if node.kind() == "lexical_declaration" || node.kind() == "expression_statement" {
            let text = &source[node.byte_range()];
            return text.contains("require(");
        }
        false
    }

    // import { Foo, Bar } from 'baz' → ["Foo", "Bar"]
    // const express = require('express') → ["express"]
    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        js_import_symbols(node, source)
    }
}

pub fn js_import_symbols(node: &Node, source: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    collect_js_import_names(node, source, &mut symbols);
    symbols
}

fn collect_js_import_names(node: &Node, source: &str, symbols: &mut Vec<String>) {
    match node.kind() {
        "import_clause" | "named_imports" | "import_specifier" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                symbols.push(source[alias.byte_range()].to_string());
                return;
            }
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(source[name.byte_range()].to_string());
                return;
            }
        }
        "identifier" => {
            // default import or require variable name
            let parent = node.parent();
            let is_binding = parent.is_some_and(|p| {
                matches!(
                    p.kind(),
                    "import_clause" | "variable_declarator" | "import_specifier"
                )
            });
            if is_binding {
                symbols.push(source[node.byte_range()].to_string());
                return;
            }
        }
        "namespace_import" => {
            // import * as Foo → "Foo"
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    symbols.push(source[child.byte_range()].to_string());
                    return;
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_js_import_names(&child, source, symbols);
    }
}
