//! Bridges the `nom-concept` AST (PipelineOutput) to the legacy `nom-ast::SourceFile`.
//! This is a temporary validation bypass layer designed to fulfill GAP-4 (nom-parser deletion)
//! while backend modules (nom-llvm, nom-resolver) migrate fully to `.nom` and `.nomtu` formats.

use nom_ast::{SourceFile, Declaration, Classifier, Identifier, Span};
use nom_concept::stages::PipelineOutput;
use nom_concept::NomtuItem;

pub fn bridge_to_ast(pipeline_out: &PipelineOutput, source_path: Option<String>) -> SourceFile {
    let mut declarations = Vec::new();

    match pipeline_out {
        PipelineOutput::Nom(nom_file) => {
            // Concepts do not have a direct imperative mapping in nom-ast.
            // We map them as `Classifier::Nom` with empty bodies, preserving their name.
            for concept in &nom_file.concepts {
                declarations.push(Declaration {
                    classifier: Classifier::Nom,
                    name: Identifier {
                        name: concept.name.clone(),
                        span: Span::default(),
                    },
                    statements: vec![],
                    span: Span::default(),
                });
            }
        }
        PipelineOutput::Nomtu(nomtu_file) => {
            // Entities and Compositions.
            for item in &nomtu_file.items {
                match item {
                    NomtuItem::Entity(ent) => {
                        let classifier = Classifier::from_str(&ent.kind).unwrap_or(Classifier::Nom);
                        declarations.push(Declaration {
                            classifier,
                            name: Identifier {
                                name: ent.word.clone(),
                                span: Span::default(),
                            },
                            statements: vec![], // Legacy pipeline expects body bytes or LLVM will fail on empty functions.
                            span: Span::default(),
                        });
                    }
                    NomtuItem::Composition(comp) => {
                        declarations.push(Declaration {
                            classifier: Classifier::System, // Treating composition as a System block
                            name: Identifier {
                                name: comp.word.clone(),
                                span: Span::default(),
                            },
                            statements: vec![],
                            span: Span::default(),
                        });
                    }
                }
            }
        }
    }

    SourceFile {
        path: source_path,
        locale: None,
        declarations,
    }
}
