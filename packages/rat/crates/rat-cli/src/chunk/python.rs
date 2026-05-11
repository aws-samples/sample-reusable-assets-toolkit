// SPDX-License-Identifier: MIT

use tree_sitter::Node;

use super::code::Language;

pub struct Python;

impl Language for Python {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        &[
            "function_definition",
            "class_definition",
            "decorated_definition",
        ]
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        &["import_statement", "import_from_statement"]
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        &["decorator", "comment"]
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &["decorated_definition"]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        &["comment", "decorator"]
    }

    // from dataclasses import dataclass → ["dataclass"]
    // from typing import Optional, List → ["Optional", "List"]
    // import os → ["os"]
    // import os as operating_system → ["operating_system"]
    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        collect_python_import_names(node, source, &mut symbols);
        symbols
    }
}

fn collect_python_import_names(node: &Node, source: &str, symbols: &mut Vec<String>) {
    match node.kind() {
        "aliased_import" => {
            // import X as Y → "Y"
            if let Some(alias) = node.child_by_field_name("alias") {
                symbols.push(source[alias.byte_range()].to_string());
                return;
            }
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(source[name.byte_range()].to_string());
                return;
            }
        }
        "dotted_name" => {
            // import foo.bar → "foo" (top-level name used in code)
            let mut cursor = node.walk();
            if let Some(first) = node.children(&mut cursor).next() {
                if first.kind() == "identifier" {
                    symbols.push(source[first.byte_range()].to_string());
                }
            }
            return;
        }
        "wildcard_import" => {
            symbols.push("*".to_string());
            return;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_python_import_names(&child, source, symbols);
    }
}
