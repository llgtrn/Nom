//! Extract UirEntity structs from tree-sitter ASTs.
//!
//! Bridge between parsing and atom extraction. Tree-sitter gives
//! language-specific node types; this module normalizes them into
//! language-agnostic UirEntity structs.

use nom_types::{AtomSignature, UirEntity, UirKind};
use anyhow::Result;
use tree_sitter::Tree;

/// A fully extracted entity: UIR entity + optional signature + optional source body.
pub type ExtractedEntity = (UirEntity, Option<AtomSignature>, Option<String>);

/// Extract UIR entities from a parsed tree-sitter AST.
pub fn extract_entities(
    tree: &Tree,
    source: &str,
    file_path: &str,
    language: &str,
) -> Vec<UirEntity> {
    let root = tree.root_node();
    let mut entities = Vec::new();
    let mut cursor = root.walk();

    extract_recursive(&mut cursor, source, file_path, language, &mut entities);
    entities
}

fn extract_recursive(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    file_path: &str,
    language: &str,
    entities: &mut Vec<UirEntity>,
) {
    loop {
        let node = cursor.node();

        if let Some(kind) = map_node_to_uir(node.kind(), language) {
            let name = extract_name(&node, source, language);
            let start = node.start_position();

            entities.push(UirEntity {
                id: format!(
                    "{file_path}:{}:{}",
                    kind.as_str(),
                    name.as_deref().unwrap_or("anonymous")
                ),
                kind: kind.as_str().to_string(),
                source_path: format!("{file_path}:{}:{}", start.row + 1, start.column),
                language: Some(language.to_string()),
                labels: build_labels(&node, &name, language),
            });
        }

        if cursor.goto_first_child() {
            extract_recursive(cursor, source, file_path, language, entities);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Map a tree-sitter node type to a UIR kind.
fn map_node_to_uir(node_type: &str, language: &str) -> Option<UirKind> {
    match (language, node_type) {
        // Functions
        (_, "function_item") => Some(UirKind::Function),
        (_, "function_definition") => Some(UirKind::Function),
        (_, "function_declaration") => Some(UirKind::Function),
        (_, "arrow_function") => Some(UirKind::Function),
        (_, "method_definition") => Some(UirKind::Method),
        (_, "method_declaration") => Some(UirKind::Method),

        // Structs / Classes
        (_, "struct_item") => Some(UirKind::Struct),
        (_, "struct_specifier") => Some(UirKind::Struct),
        (_, "class_definition") => Some(UirKind::Class),
        (_, "class_declaration") => Some(UirKind::Class),
        (_, "class_specifier") => Some(UirKind::Class),
        (_, "type_declaration") => Some(UirKind::Schema),
        (_, "type_spec") => Some(UirKind::Schema),

        // Traits / Interfaces
        (_, "trait_item") => Some(UirKind::Trait),
        (_, "interface_declaration") => Some(UirKind::Interface),
        ("go", "interface_type") => Some(UirKind::Interface),

        // Type aliases / Enums
        (_, "type_alias_declaration") => Some(UirKind::Schema),
        (_, "enum_item") => Some(UirKind::Schema),
        (_, "enum_specifier") => Some(UirKind::Schema),

        // Modules / Namespaces
        (_, "mod_item") => Some(UirKind::Module),
        (_, "namespace_definition") => Some(UirKind::Module),
        (_, "impl_item") => Some(UirKind::Module),

        // Skip
        ("rust", "attribute_item") => None,
        (_, "preproc_include") => None,
        (_, "preproc_ifdef") => None,

        _ => None,
    }
}

/// Extract the name of a node from source code.
fn extract_name(node: &tree_sitter::Node, source: &str, _language: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "name" | "type_identifier" | "field_identifier" => {
                return Some(child.utf8_text(source.as_bytes()).ok()?.to_string());
            }
            "function_declarator" | "declarator" => {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "identifier" {
                        return Some(inner.utf8_text(source.as_bytes()).ok()?.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Build descriptive labels for a UIR entity.
fn build_labels(node: &tree_sitter::Node, name: &Option<String>, language: &str) -> Vec<String> {
    let mut labels = vec![language.to_string()];

    if let Some(n) = name {
        if n.starts_with("test_") || n.starts_with("test ") {
            labels.push("test".to_string());
        }
        if n == "main" {
            labels.push("entrypoint".to_string());
        }
        if language == "rust" {
            let source_text = node.parent().map(|p| p.kind());
            if let Some(parent_kind) = source_text
                && parent_kind == "visibility_modifier"
            {
                labels.push("public".to_string());
            }
        }
    }

    labels
}

/// Extract function signature from a tree-sitter function node.
pub fn extract_signature(
    node: &tree_sitter::Node,
    source: &str,
    language: &str,
) -> Option<AtomSignature> {
    let kind = node.kind();
    if !matches!(
        kind,
        "function_item"
            | "function_definition"
            | "function_declaration"
            | "arrow_function"
            | "method_definition"
            | "method_declaration"
    ) {
        return None;
    }

    let mut sig = AtomSignature::default();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "parameters" | "formal_parameters" => {
                let mut param_cursor = child.walk();
                for param in child.children(&mut param_cursor) {
                    if param.is_named() && param.kind() != "comment" {
                        let text = param.utf8_text(source.as_bytes()).unwrap_or_default();
                        if let Some((name, typ)) = text.split_once(':') {
                            let name = name
                                .trim()
                                .trim_start_matches('&')
                                .trim_start_matches("mut ");
                            let typ = typ.trim();
                            if name == "self" || name == "&self" || name == "&mut self" {
                                sig.is_method = true;
                            } else if !name.is_empty() {
                                sig.params.push((name.to_string(), typ.to_string()));
                            }
                        } else if text.trim() == "self"
                            || text.trim() == "&self"
                            || text.trim() == "&mut self"
                        {
                            sig.is_method = true;
                        } else if text.trim() != "," && !text.trim().is_empty() {
                            sig.params.push((text.trim().to_string(), String::new()));
                        }
                    }
                }
            }
            "type_identifier" | "generic_type" | "reference_type" | "scoped_type_identifier"
                if language == "rust" =>
            {
                sig.returns = Some(
                    child
                        .utf8_text(source.as_bytes())
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            "type_annotation" => {
                sig.returns = Some(
                    child
                        .utf8_text(source.as_bytes())
                        .unwrap_or_default()
                        .trim_start_matches(':')
                        .trim()
                        .to_string(),
                );
            }
            "async" => {
                sig.is_async = true;
            }
            "visibility_modifier" => {
                sig.visibility = child
                    .utf8_text(source.as_bytes())
                    .unwrap_or_default()
                    .to_string();
            }
            _ => {}
        }
    }

    if language == "python" {
        let parent_text = node.utf8_text(source.as_bytes()).unwrap_or_default();
        if parent_text.starts_with("async ") {
            sig.is_async = true;
        }
    }

    if sig.visibility.is_empty() {
        sig.visibility = "private".to_string();
    }

    Some(sig)
}

/// Parse a source file and extract all UIR entities in one call.
pub fn parse_and_extract(source: &str, file_path: &str, language: &str) -> Result<Vec<UirEntity>> {
    let tree = crate::parse_source(source, language)?;
    Ok(extract_entities(&tree, source, file_path, language))
}

/// Parse and extract entities with signatures (no bodies).
pub fn parse_and_extract_with_signatures(
    source: &str,
    file_path: &str,
    language: &str,
) -> Result<Vec<(UirEntity, Option<AtomSignature>)>> {
    let triples = parse_and_extract_full(source, file_path, language)?;
    Ok(triples
        .into_iter()
        .map(|(entity, sig, _body)| (entity, sig))
        .collect())
}

/// Parse and extract entities with signatures AND source bodies.
pub fn parse_and_extract_full(
    source: &str,
    file_path: &str,
    language: &str,
) -> Result<Vec<ExtractedEntity>> {
    let tree = crate::parse_source(source, language)?;
    let root = tree.root_node();
    let mut results = Vec::new();

    collect_full(&mut root.walk(), source, file_path, language, &mut results);
    Ok(results)
}

/// Max body size to store per atom (4KB).
const MAX_BODY_LEN: usize = 4_096;

/// Extract the source body of a node, capped at MAX_BODY_LEN.
fn extract_body(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let text = node.utf8_text(source.as_bytes()).ok()?;
    if text.len() <= MAX_BODY_LEN {
        Some(text.to_string())
    } else {
        let mut end = MAX_BODY_LEN;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = &text[..end];
        if let Some(last_nl) = truncated.rfind('\n') {
            Some(format!("{}\n// ... truncated", &truncated[..last_nl]))
        } else {
            Some(format!("{truncated}\n// ... truncated"))
        }
    }
}

fn collect_full(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    file_path: &str,
    language: &str,
    results: &mut Vec<ExtractedEntity>,
) {
    loop {
        let node = cursor.node();

        if let Some(kind) = map_node_to_uir(node.kind(), language) {
            let name = extract_name(&node, source, language);
            let start = node.start_position();
            let sig = extract_signature(&node, source, language);
            let body = extract_body(&node, source);

            results.push((
                UirEntity {
                    id: format!(
                        "{file_path}:{}:{}",
                        kind.as_str(),
                        name.as_deref().unwrap_or("anonymous")
                    ),
                    kind: kind.as_str().to_string(),
                    source_path: format!("{file_path}:{}:{}", start.row + 1, start.column),
                    language: Some(language.to_string()),
                    labels: build_labels(&node, &name, language),
                },
                sig,
                body,
            ));
        }

        if cursor.goto_first_child() {
            collect_full(cursor, source, file_path, language, results);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
