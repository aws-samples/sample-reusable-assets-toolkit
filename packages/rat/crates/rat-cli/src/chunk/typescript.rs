use tree_sitter::Node;

use super::code::Language;
use super::javascript::js_import_symbols;

pub struct TypeScript;

impl Language for TypeScript {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        TS_TARGET_NODES
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        TS_IMPORT_NODES
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        TS_ATTRIBUTE_NODES
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &["export_statement"]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        TS_SKIP_NODES
    }

    fn is_import(&self, node: &Node, source: &str) -> bool {
        ts_is_import(self, node, source)
    }

    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        js_import_symbols(node, source)
    }
}

pub struct Tsx;

impl Language for Tsx {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        TS_TARGET_NODES
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        TS_IMPORT_NODES
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        TS_ATTRIBUTE_NODES
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &["export_statement"]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        TS_SKIP_NODES
    }

    fn is_import(&self, node: &Node, source: &str) -> bool {
        ts_is_import(self, node, source)
    }

    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        js_import_symbols(node, source)
    }
}

const TS_TARGET_NODES: &[&str] = &[
    "function_declaration",
    "class_declaration",
    "method_definition",
    "export_statement",
    "lexical_declaration",
    "expression_statement",
];

const TS_IMPORT_NODES: &[&str] = &["import_statement"];
const TS_ATTRIBUTE_NODES: &[&str] = &["comment", "decorator"];
const TS_SKIP_NODES: &[&str] = &["comment"];

fn ts_is_import(lang: &dyn Language, node: &Node, source: &str) -> bool {
    if lang.import_nodes().contains(&node.kind()) {
        return true;
    }
    if node.kind() == "lexical_declaration" || node.kind() == "expression_statement" {
        let text = &source[node.byte_range()];
        return text.contains("require(");
    }
    false
}
