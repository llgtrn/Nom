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
fn lookup_nomtu(query: &str, _kind: Option<&str>) -> LookupResult {
    // Stub — will be wired to nom-dict lookup when Dict path is available
    LookupResult {
        matches: vec![NomtuMatch {
            word: format!("stub:{query}"),
            kind: "function".into(),
            score: 0.0,
        }],
    }
}

#[tauri::command]
fn match_grammar(input: &str) -> Vec<GrammarMatch> {
    // Stub — will be wired to nom-grammar Jaccard search
    vec![GrammarMatch {
        pattern: format!("stub pattern for: {input}"),
        score: 0.0,
    }]
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
