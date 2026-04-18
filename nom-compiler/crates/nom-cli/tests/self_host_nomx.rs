//! Integration tests for self-hosting .nomx parser files (GAP-9).
//! Verifies that the Nom parser written in Nom compiles through S1-S6.

use nom_concept::stages::{PipelineOutput, run_pipeline};

fn compile_nomx(source: &str) -> PipelineOutput {
    run_pipeline(source).expect("pipeline should succeed")
}

fn entity_names(output: &PipelineOutput) -> Vec<String> {
    match output {
        PipelineOutput::Nomtu(f) => f
            .items
            .iter()
            .map(|item| match item {
                nom_concept::NomtuItem::Entity(e) => e.word.clone(),
                nom_concept::NomtuItem::Composition(c) => c.word.clone(),
            })
            .collect(),
        PipelineOutput::Nom(f) => f.concepts.iter().map(|c| c.name.clone()).collect(),
    }
}

#[test]
fn tokenizer_functions_compile() {
    let source = r#"
the function tokenize is
  intended to split source text into a stream of tokens.
  given source of text, returns text.
  requires source is not empty.
  benefit fast_path.

the function skip_whitespace is
  intended to advance past whitespace characters in the source.
  given source of text, returns text.

the function peek_token is
  intended to look at the next token without consuming it.
  given source of text, returns text.

the function consume_token is
  intended to read and consume the next token from the source.
  given source of text, returns text.
  requires source has remaining tokens.

the function classify_token is
  intended to determine if a token is a keyword identifier or literal.
  given token of text, returns text.
"#;
    let output = compile_nomx(source);
    let names = entity_names(&output);
    assert!(names.contains(&"tokenize".to_string()));
    assert!(names.contains(&"skip_whitespace".to_string()));
    assert!(names.contains(&"peek_token".to_string()));
    assert!(names.contains(&"consume_token".to_string()));
    assert!(names.contains(&"classify_token".to_string()));
    assert_eq!(names.len(), 5);
}

#[test]
fn parser_functions_compile() {
    let source = r#"
the function parse_entity_declaration is
  intended to parse a complete entity declaration from tokens.
  given source of text, returns text.
  requires source starts with the keyword the.

the function parse_kind is
  intended to extract the entity kind from a declaration.
  given source of text, returns text.

the function parse_signature is
  intended to extract the function signature including parameters and return type.
  given source of text, returns text.

the function parse_contracts is
  intended to extract requires and ensures clauses from a declaration.
  given source of text, returns text.

the function parse_effects is
  intended to extract benefit and hazard clauses from a declaration.
  given source of text, returns text.
"#;
    let output = compile_nomx(source);
    let names = entity_names(&output);
    assert!(names.contains(&"parse_entity_declaration".to_string()));
    assert!(names.contains(&"parse_kind".to_string()));
    assert_eq!(names.len(), 5);
}

#[test]
fn module_composition_compiles() {
    let source = r#"
the module self_host_parser is
  intended to compose all parser functions into the pipeline.
  uses the function tokenize,
       the function parse_entity_declaration.
"#;
    let output = compile_nomx(source);
    let names = entity_names(&output);
    assert!(names.contains(&"self_host_parser".to_string()));
}

#[test]
fn concept_declaration_compiles() {
    let source = r#"
the concept nom_parser is
  intended to be the self-hosting parser for the Nom language.
  uses the module self_host_parser.
  exposes tokenize.
  this works when all entity declarations parse correctly.
  favor composability then readability then performance.
"#;
    let output = compile_nomx(source);
    match &output {
        PipelineOutput::Nom(f) => {
            assert_eq!(f.concepts.len(), 1);
            assert_eq!(f.concepts[0].name, "nom_parser");
        }
        _ => panic!("expected Nom output for concept"),
    }
}
