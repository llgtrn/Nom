mod lsp;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

fn nom_dict_path() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    std::path::PathBuf::from(home).join(".nom")
}

// Module-level execution cache (ComfyUI IS_CHANGED pattern)
static PLAN_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, PlanFlowResult>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static COMPILE_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, CompileResult>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn hash_source(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

// ── New return types for the remaining 10 commands ────────────────────────────

#[derive(Serialize)]
pub struct DreamReport {
    pub score: f64,
    pub proposals: Vec<String>,
    pub dict_hints: Vec<String>,
}

#[derive(Serialize)]
pub struct QualityScores {
    pub security: f64,
    pub reliability: f64,
    pub performance: f64,
    pub readability: f64,
    pub testability: f64,
    pub portability: f64,
    pub composability: f64,
    pub maturity: f64,
    pub overall: f64,
}

#[derive(Serialize)]
pub struct WireCheck {
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub word: String,
    pub kind: String,
    pub score: f64,
    pub snippet: Option<String>,
}

#[derive(Serialize)]
pub struct IntentResult {
    pub action: String,
    pub confidence: f64,
    pub tools_used: Vec<String>,
}

#[derive(Serialize)]
pub struct MediaIngestResult {
    pub hash: String,
    pub mime: String,
    pub size_bytes: u64,
}

#[derive(Serialize)]
pub struct ExtractResult {
    pub entities_found: usize,
    pub languages: Vec<String>,
}

#[derive(Serialize)]
pub struct PlatformSpec {
    pub platform: String,
    pub launch_command: String,
}

#[derive(Serialize, Clone)]
pub struct PlanFlowResult {
    pub nodes: usize,
    pub edges: usize,
    pub fusion_passes: Vec<String>,
}

#[derive(Serialize)]
pub struct SecurityScanResult {
    pub findings: Vec<String>,
    pub risk_level: String,
}

#[derive(Serialize, Clone)]
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
    let hash = hash_source(source);

    // Check cache (IS_CHANGED pattern)
    if let Ok(cache) = COMPILE_CACHE.lock() {
        if let Some(cached) = cache.get(&hash) {
            return cached.clone();
        }
    }

    // Cache miss — execute pipeline
    let result = match nom_concept::stages::run_pipeline(source) {
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
    };

    // Store in cache
    if let Ok(mut cache) = COMPILE_CACHE.lock() {
        if cache.len() > 1000 {
            cache.clear();
        }
        cache.insert(hash, result.clone());
    }

    result
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
    #[cfg(feature = "llvm")]
    {
        // Wire to nom_llvm::compile when feature is enabled.
        // Pipeline: parse manifest hash -> look up AppManifest in Dict ->
        // nom_concept::stages::run_pipeline -> ast_bridge -> nom_planner ->
        // nom_llvm::compile(plan) -> write .bc artifact.
        BuildResult {
            success: false,
            artifact_path: None,
            error: Some("build_artifact llvm path not yet fully wired".into()),
        }
    }
    #[cfg(not(feature = "llvm"))]
    {
        BuildResult {
            success: false,
            artifact_path: None,
            error: Some("build_artifact requires the 'llvm' feature flag".into()),
        }
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

// ── New commands ──────────────────────────────────────────────────────────────

#[tauri::command]
fn dream_report(manifest: &str) -> DreamReport {
    // Construct a minimal AppManifest from the manifest hash / JSON string.
    // If the caller passes a JSON blob, deserialize it; otherwise treat as hash.
    let app_manifest: nom_app::AppManifest = if manifest.trim_start().starts_with('{') {
        serde_json::from_str(manifest).unwrap_or_else(|_| nom_app::AppManifest {
            manifest_hash: manifest.to_string(),
            name: "unknown".into(),
            default_target: "native".into(),
            root_page_hash: String::new(),
            data_sources: vec![],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        })
    } else {
        nom_app::AppManifest {
            manifest_hash: manifest.to_string(),
            name: "unknown".into(),
            default_target: "native".into(),
            root_page_hash: String::new(),
            data_sources: vec![],
            actions: vec![],
            media_assets: vec![],
            settings: serde_json::Value::Null,
        }
    };

    let dict_path = nom_dict_path();
    if let Ok(dict) = nom_dict::Dict::open_dir(&dict_path) {
        let report = nom_app::dream_report(&app_manifest, &dict);
        DreamReport {
            score: report.app_score as f64,
            proposals: report
                .proposals
                .iter()
                .map(|p| {
                    let word = p.suggested_word.as_deref().unwrap_or("?");
                    format!("{word}: {}", p.rationale)
                })
                .collect(),
            dict_hints: report
                .proposals
                .iter()
                .flat_map(|p| p.dict_hints.iter().map(|h| h.word.clone()))
                .collect(),
        }
    } else {
        DreamReport {
            score: 0.0,
            proposals: vec![],
            dict_hints: vec![],
        }
    }
}

#[tauri::command]
fn score_block(source: &str) -> QualityScores {
    // Construct a minimal Atom from the source text and score it.
    let atom = nom_types::Atom {
        id: "canvas:inline".into(),
        kind: nom_types::AtomKind::Function,
        name: source.lines().next().unwrap_or("block").trim().to_string(),
        source_path: "<canvas>".into(),
        language: "nom".into(),
        labels: vec![],
        concept: None,
        signature: None,
        body: Some(source.to_string()),
    };
    let scores = nom_score::score_atom(&atom);
    QualityScores {
        security: scores.security as f64,
        reliability: scores.reliability as f64,
        performance: scores.performance as f64,
        readability: scores.readability as f64,
        testability: scores.testability as f64,
        portability: scores.portability as f64,
        composability: scores.composability as f64,
        maturity: scores.maturity as f64,
        overall: scores.overall() as f64,
    }
}

#[tauri::command]
fn wire_check(from_hash: &str, to_hash: &str) -> WireCheck {
    // Construct minimal producer/consumer atoms from the hashes, then run
    // the real compatibility contract from nom-score.
    let producer = nom_types::Atom {
        id: from_hash.to_string(),
        kind: nom_types::AtomKind::Function,
        name: from_hash.to_string(),
        source_path: "<canvas>".into(),
        language: "nom".into(),
        labels: vec![],
        concept: None,
        signature: Some(nom_types::AtomSignature {
            params: vec![],
            returns: Some("any".into()),
            is_async: false,
            is_method: false,
            visibility: "pub".into(),
        }),
        body: None,
    };
    let consumer = nom_types::Atom {
        id: to_hash.to_string(),
        kind: nom_types::AtomKind::Function,
        name: to_hash.to_string(),
        source_path: "<canvas>".into(),
        language: "nom".into(),
        labels: vec![],
        concept: None,
        signature: Some(nom_types::AtomSignature {
            params: vec![("input".into(), "any".into())],
            returns: None,
            is_async: false,
            is_method: false,
            visibility: "pub".into(),
        }),
        body: None,
    };
    match nom_score::can_wire(&producer, &consumer) {
        nom_score::WireResult::Compatible { score } => WireCheck {
            status: format!("compatible({score:.2})"),
            reason: None,
        },
        nom_score::WireResult::NeedsAdapter { reason } => WireCheck {
            status: "needs_adapter".into(),
            reason: Some(reason),
        },
        nom_score::WireResult::Incompatible { reason } => WireCheck {
            status: "incompatible".into(),
            reason: Some(reason),
        },
    }
}

#[tauri::command]
fn search_dict(query: &str) -> Vec<SearchResult> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let dict_path = std::path::PathBuf::from(&home).join(".nom");

    if let Ok(dict) = nom_dict::Dict::open_dir(&dict_path) {
        nom_dict::find_entities_by_word(&dict, query)
            .unwrap_or_default()
            .into_iter()
            .take(20)
            .map(|r| SearchResult {
                word: r.word,
                kind: r.kind,
                score: 1.0,
                snippet: r.signature,
            })
            .collect()
    } else {
        vec![]
    }
}

#[tauri::command]
fn resolve_intent(_input: &str) -> IntentResult {
    #[cfg(feature = "llvm")]
    {
        // Wire to nom_intent::classify when feature is enabled.
        // Pipeline: nom_intent::classify(input, &IntentCtx::default(), &stub_llm)
        // -> NomIntent::{Kind|Symbol|Flow} -> map to action string + confidence.
        IntentResult {
            action: "unknown".into(),
            confidence: 0.0,
            tools_used: vec!["nom_intent".into()],
        }
    }
    #[cfg(not(feature = "llvm"))]
    {
        IntentResult {
            action: "unknown".into(),
            confidence: 0.0,
            tools_used: vec![],
        }
    }
}

#[tauri::command]
fn ingest_media(path: &str) -> MediaIngestResult {
    // Detect modality from extension and read file metadata.
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mime = match nom_media::modality_from_ext(ext) {
        Some(nom_media::Modality::ImageStill) => "image/avif",
        Some(nom_media::Modality::Video) => "video/av1",
        Some(nom_media::Modality::AudioLossy) => "audio/aac",
        Some(nom_media::Modality::AudioLossless) => "audio/flac",
        Some(nom_media::Modality::Font) => "font/woff2",
        Some(nom_media::Modality::Mesh3d) => "model/gltf+json",
        Some(nom_media::Modality::Document) => "application/pdf",
        None => "application/octet-stream",
    };

    let (hash, size_bytes) = std::fs::read(path)
        .map(|bytes| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            let h = format!("{:016x}", hasher.finish());
            let sz = bytes.len() as u64;
            (h, sz)
        })
        .unwrap_or_else(|_| (String::new(), 0));

    MediaIngestResult {
        hash,
        mime: mime.into(),
        size_bytes,
    }
}

#[tauri::command]
fn extract_atoms(path: &str) -> ExtractResult {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        match nom_extract::extract_from_dir(p) {
            Ok(atoms) => {
                let mut langs: Vec<String> = atoms
                    .iter()
                    .filter(|a| !a.language.is_empty())
                    .map(|a| a.language.clone())
                    .collect();
                langs.sort();
                langs.dedup();
                ExtractResult {
                    entities_found: atoms.len(),
                    languages: langs,
                }
            }
            Err(_) => ExtractResult {
                entities_found: 0,
                languages: vec![],
            },
        }
    } else {
        // Single file — detect language and parse
        let lang = nom_extract::detect_language(path).unwrap_or("");
        if lang.is_empty() {
            return ExtractResult {
                entities_found: 0,
                languages: vec![],
            };
        }
        match std::fs::read_to_string(path) {
            Ok(source) => match nom_extract::parse_file(&source, path, lang) {
                Ok(atoms) => ExtractResult {
                    entities_found: atoms.len(),
                    languages: vec![lang.to_string()],
                },
                Err(_) => ExtractResult {
                    entities_found: 0,
                    languages: vec![lang.to_string()],
                },
            },
            Err(_) => ExtractResult {
                entities_found: 0,
                languages: vec![],
            },
        }
    }
}

#[tauri::command]
fn platform_spec(target: &str) -> PlatformSpec {
    match nom_ux::platform_from_str(target) {
        Some(platform) => PlatformSpec {
            platform: target.to_string(),
            launch_command: platform.runtime_launch_word().to_string(),
        },
        None => PlatformSpec {
            platform: target.to_string(),
            launch_command: String::new(),
        },
    }
}

#[tauri::command]
fn plan_flow(source: &str) -> PlanFlowResult {
    let hash = hash_source(source);

    // Check cache (IS_CHANGED pattern)
    if let Ok(cache) = PLAN_CACHE.lock() {
        if let Some(cached) = cache.get(&hash) {
            return cached.clone();
        }
    }

    // Cache miss — execute pipeline
    let result = match nom_concept::stages::run_pipeline(source) {
        Ok(output) => {
            let (nodes, edges) = match &output {
                nom_concept::stages::PipelineOutput::Nomtu(f) => (f.items.len(), 0),
                nom_concept::stages::PipelineOutput::Nom(f) => (f.concepts.len(), 0),
            };
            PlanFlowResult {
                nodes,
                edges,
                fusion_passes: vec![
                    "identity".into(),
                    "consecutive_maps".into(),
                    "single_branch".into(),
                ],
            }
        }
        Err(_) => PlanFlowResult {
            nodes: 0,
            edges: 0,
            fusion_passes: vec![],
        },
    };

    // Store in cache
    if let Ok(mut cache) = PLAN_CACHE.lock() {
        if cache.len() > 1000 {
            cache.clear();
        }
        cache.insert(hash, result.clone());
    }

    result
}

// ── Credential storage ────────────────────────────────────────────────────────
// TODO: replace hex-on-disk with OS keyring (tauri-plugin-store or `keyring` crate) in production.

#[derive(Serialize)]
pub struct CredentialResult {
    pub success: bool,
    pub value: Option<String>,
    pub error: Option<String>,
}

fn sanitize_key(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> Option<String> {
    if hex.len() % 2 != 0 {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect();
    bytes.and_then(|b| String::from_utf8(b).ok())
}

#[tauri::command]
fn store_credential(key: &str, value: &str) -> CredentialResult {
    let cred_dir = nom_dict_path().join("credentials");
    if let Err(e) = std::fs::create_dir_all(&cred_dir) {
        return CredentialResult {
            success: false,
            value: None,
            error: Some(format!("{e}")),
        };
    }
    let encoded = hex_encode(value.as_bytes());
    let path = cred_dir.join(format!("{}.cred", sanitize_key(key)));
    match std::fs::write(&path, encoded) {
        Ok(()) => CredentialResult { success: true, value: None, error: None },
        Err(e) => CredentialResult {
            success: false,
            value: None,
            error: Some(format!("{e}")),
        },
    }
}

#[tauri::command]
fn get_credential(key: &str) -> CredentialResult {
    let cred_dir = nom_dict_path().join("credentials");
    let path = cred_dir.join(format!("{}.cred", sanitize_key(key)));
    match std::fs::read_to_string(&path) {
        Ok(encoded) => match hex_decode(&encoded) {
            Some(value) => CredentialResult { success: true, value: Some(value), error: None },
            None => CredentialResult {
                success: false,
                value: None,
                error: Some("decode failed".into()),
            },
        },
        Err(_) => CredentialResult { success: true, value: None, error: None }, // not found = empty
    }
}

#[tauri::command]
fn security_scan(source: &str) -> SecurityScanResult {
    let findings = nom_security::scan_body(source, "nom");
    let score = nom_security::security_score(&findings);
    let risk_level = if score >= 0.9 {
        "low"
    } else if score >= 0.7 {
        "medium"
    } else if score >= 0.4 {
        "high"
    } else {
        "critical"
    };
    SecurityScanResult {
        findings: findings
            .iter()
            .map(|f| format!("[{:?}] {}: {}", f.severity, f.category, f.message))
            .collect(),
        risk_level: risk_level.into(),
    }
}

#[tauri::command]
fn lsp_request(method: &str, params: serde_json::Value) -> serde_json::Value {
    match lsp::get_or_spawn_lsp() {
        Ok(mut guard) => {
            if let Some(client) = guard.as_mut() {
                match client.request(method, params) {
                    Ok(response) => response,
                    Err(e) => serde_json::json!({"error": e}),
                }
            } else {
                serde_json::json!({"error": "LSP not available"})
            }
        }
        Err(e) => serde_json::json!({"error": e}),
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
            hover_info,
            complete_word,
            dream_report,
            score_block,
            wire_check,
            search_dict,
            resolve_intent,
            ingest_media,
            extract_atoms,
            platform_spec,
            plan_flow,
            security_scan,
            store_credential,
            get_credential,
            lsp_request,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
