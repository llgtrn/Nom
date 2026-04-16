use serde::Serialize;

#[derive(Serialize)]
pub struct CompileResult {
    pub success: bool,
    pub diagnostics: Vec<String>,
    pub entities: Vec<String>,
}

#[derive(Serialize)]
pub struct LookupResult {
    pub matches: Vec<NomtuMatch>,
}

#[derive(Serialize)]
pub struct NomtuMatch {
    pub word: String,
    pub kind: String,
    pub score: f64,
}

#[derive(Serialize)]
pub struct GrammarMatch {
    pub pattern: String,
    pub score: f64,
}

#[derive(Serialize)]
pub struct BuildResult {
    pub success: bool,
    pub artifact_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct HoverInfo {
    pub markdown: String,
    pub definition_file: Option<String>,
    pub definition_line: Option<u32>,
}

#[derive(Serialize)]
pub struct CompletionItem {
    pub word: String,
    pub kind: String,
    pub score: f64,
}

#[tauri::command]
fn compile_block(source: &str) -> CompileResult {
    match nom_concept::stages::run_pipeline(source) {
        Ok(output) => {
            let entities: Vec<String> = match &output {
                nom_concept::stages::PipelineOutput::Nomtu(f) => {
                    f.items.iter().map(|item| match item {
                        nom_concept::NomtuItem::Entity(e) => e.word.clone(),
                        nom_concept::NomtuItem::Composition(c) => c.word.clone(),
                    }).collect()
                }
                nom_concept::stages::PipelineOutput::Nom(f) => {
                    f.concepts.iter().map(|c| c.name.clone()).collect()
                }
            };
            CompileResult {
                success: true,
                diagnostics: vec![],
                entities,
            }
        }
        Err(e) => CompileResult {
            success: false,
            diagnostics: vec![format!("{e:?}")],
            entities: vec![],
        },
    }
}

#[tauri::command]
fn lookup_nomtu(query: &str, kind: Option<&str>) -> LookupResult {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let dict_path = std::path::PathBuf::from(&home).join(".nom");

    let matches = if let Ok(dict) = nom_dict::Dict::open_dir(&dict_path) {
        let query_lower = query.to_lowercase();
        let rows = match kind {
            Some(k) => nom_dict::find_entities_by_kind(&dict, k).unwrap_or_default(),
            None => nom_dict::find_entities_by_word(&dict, query).unwrap_or_default(),
        };
        rows.into_iter()
            .filter(|r| r.word.to_lowercase().contains(&query_lower))
            .take(20)
            .map(|r| NomtuMatch {
                word: r.word,
                kind: r.kind,
                score: 1.0,
            })
            .collect()
    } else {
        vec![]
    };

    LookupResult { matches }
}

#[tauri::command]
fn match_grammar(input: &str) -> Vec<GrammarMatch> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let grammar_path = std::path::PathBuf::from(&home)
        .join(".nom")
        .join("grammar.sqlite");

    if let Ok(conn) = nom_grammar::open_readonly(&grammar_path) {
        nom_grammar::search_patterns(&conn, input, 0.1, 10)
            .unwrap_or_default()
            .into_iter()
            .map(|m| GrammarMatch {
                pattern: m.intent,
                score: m.score,
            })
            .collect()
    } else {
        vec![]
    }
}

#[tauri::command]
fn build_artifact(_manifest_hash: &str) -> BuildResult {
    // Stub — will be wired to nom-llvm compilation
    BuildResult {
        success: false,
        artifact_path: None,
        error: Some("build_artifact not yet implemented".into()),
    }
}

#[tauri::command]
fn hover_info(word: &str) -> HoverInfo {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let dict_path = std::path::PathBuf::from(&home).join(".nom");

    if let Ok(dict) = nom_dict::Dict::open_dir(&dict_path) {
        if let Ok(mut rows) = nom_dict::find_entities_by_word(&dict, word) {
            if let Some(entity) = rows.drain(..).next() {
                let definition_file = entity
                    .origin_ref
                    .clone()
                    .or_else(|| entity.authored_in.clone());

                let mut md = format!("**{}** `{}`\n", entity.word, entity.kind);
                if let Some(sig) = &entity.signature {
                    md.push_str(&format!("\n**Signature:** `{sig}`\n"));
                }
                if let Some(contracts_json) = &entity.contracts {
                    if !contracts_json.is_empty() && contracts_json != "[]" {
                        if let Ok(clauses) =
                            serde_json::from_str::<Vec<serde_json::Value>>(contracts_json)
                        {
                            let lines: Vec<String> = clauses
                                .into_iter()
                                .filter_map(|v| {
                                    if let Some(pred) =
                                        v.get("Requires").and_then(|r| r.as_str())
                                    {
                                        Some(format!("requires: {pred}"))
                                    } else if let Some(pred) =
                                        v.get("Ensures").and_then(|r| r.as_str())
                                    {
                                        Some(format!("ensures: {pred}"))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !lines.is_empty() {
                                md.push_str("\n**Contracts:**\n");
                                for line in &lines {
                                    md.push_str(&format!("- {line}\n"));
                                }
                            }
                        }
                    }
                }
                if let Some(bk) = &entity.body_kind {
                    md.push_str(&format!("\n**Body kind:** `{bk}`\n"));
                }
                let source = entity
                    .origin_ref
                    .as_deref()
                    .or(entity.authored_in.as_deref())
                    .unwrap_or("source unavailable");
                md.push_str(&format!("\n**Source:** `{source}`\n"));

                return HoverInfo {
                    markdown: md,
                    definition_file,
                    definition_line: None,
                };
            }
        }
    }

    HoverInfo {
        markdown: format!("No entity found for `{word}`"),
        definition_file: None,
        definition_line: None,
    }
}

#[tauri::command]
fn complete_word(prefix: &str, _context: Option<&str>) -> Vec<CompletionItem> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let dict_path = std::path::PathBuf::from(&home).join(".nom");

    let mut items: Vec<CompletionItem> = Vec::new();

    if let Ok(dict) = nom_dict::Dict::open_dir(&dict_path) {
        if let Ok(rows) = nom_dict::find_entities_by_word(&dict, prefix) {
            for entity in rows
                .into_iter()
                .filter(|r| r.word.to_lowercase().starts_with(&prefix.to_lowercase()))
                .take(20)
            {
                items.push(CompletionItem {
                    word: entity.word,
                    kind: entity.kind,
                    score: 1.0,
                });
            }
        }
    }

    let grammar_path = std::path::PathBuf::from(&home)
        .join(".nom")
        .join("grammar.sqlite");
    if let Ok(conn) = nom_grammar::open_readonly(&grammar_path) {
        if let Ok(matches) = nom_grammar::search_patterns(&conn, prefix, 0.1, 10) {
            for m in matches {
                items.push(CompletionItem {
                    word: m.intent,
                    kind: "pattern".into(),
                    score: m.score,
                });
            }
        }
    }

    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.truncate(20);
    items
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            compile_block,
            lookup_nomtu,
            match_grammar,
            build_artifact,
            hover_info,
            complete_word,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
