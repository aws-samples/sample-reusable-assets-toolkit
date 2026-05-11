// SPDX-License-Identifier: MIT

use tree_sitter::Node;

use super::code::Language;

pub struct Go;

impl Language for Go {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_go::LANGUAGE.into()
    }

    fn target_nodes(&self) -> &'static [&'static str] {
        &[
            "function_declaration",
            "method_declaration",
            "type_declaration",
        ]
    }

    fn import_nodes(&self) -> &'static [&'static str] {
        &["import_declaration"]
    }

    fn attribute_nodes(&self) -> &'static [&'static str] {
        &["comment"]
    }

    fn wrapper_nodes(&self) -> &'static [&'static str] {
        &[]
    }

    fn skip_nodes(&self) -> &'static [&'static str] {
        &[
            "comment",
            "package_clause",
            "const_declaration",
            "var_declaration",
        ]
    }

    // import "fmt" → ["fmt"]
    // import ( "fmt" \n "strings" ) → ["fmt", "strings"]
    // import alias "pkg/path" → ["alias"]
    fn import_symbols(&self, node: &Node, source: &str) -> Vec<String> {
        let mut symbols = Vec::new();
        collect_go_import_names(node, source, &mut symbols);
        symbols
    }
}

fn collect_go_import_names(node: &Node, source: &str, symbols: &mut Vec<String>) {
    match node.kind() {
        "import_spec" => {
            // import alias "path" or import "path"
            if let Some(name) = node.child_by_field_name("name") {
                // alias: import f "fmt"
                symbols.push(source[name.byte_range()].to_string());
                return;
            }
            if let Some(path) = node.child_by_field_name("path") {
                // "fmt" → fmt (strip quotes, take last segment)
                let raw = source[path.byte_range()].trim_matches('"');
                if let Some(last) = raw.rsplit('/').next() {
                    symbols.push(last.to_string());
                }
                return;
            }
        }
        "interpreted_string_literal" => {
            // standalone import "fmt"
            let raw = source[node.byte_range()].trim_matches('"');
            if let Some(last) = raw.rsplit('/').next() {
                symbols.push(last.to_string());
            }
            return;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_go_import_names(&child, source, symbols);
    }
}
