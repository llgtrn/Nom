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
fn hover_info(_file: &str, _line: u32, _col: u32) -> HoverInfo {
    HoverInfo {
        markdown: "Hover info not yet implemented".into(),
        definition_file: None,
        definition_line: None,
    }
}

#[tauri::command]
fn complete_word(prefix: &str, _context: Option<&str>) -> Vec<CompletionItem> {
    vec![CompletionItem {
        word: format!("{prefix}..."),
        kind: "function".into(),
        score: 0.0,
    }]
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
