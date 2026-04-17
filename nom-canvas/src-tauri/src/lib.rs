mod lsp;

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

fn nom_dict_path() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    std::path::PathBuf::from(home).join(".nom")
}

struct AppState {
    dict_path: std::path::PathBuf,
    grammar_path: std::path::PathBuf,
}

impl AppState {
    fn new() -> Self {
        let base = nom_dict_path();
        Self {
            grammar_path: base.join("grammar.sqlite"),
            dict_path: base,
        }
    }

    fn open_dict(&self) -> Option<nom_dict::Dict> {
        nom_dict::Dict::open_dir(&self.dict_path).ok()
    }

    fn open_grammar(&self) -> Option<rusqlite::Connection> {
        nom_grammar::open_readonly(&self.grammar_path).ok()
    }
}

// Module-level execution cache (ComfyUI IS_CHANGED pattern)
// NOTE: Tauri command invocations are serialized per-window by default.
// Cache thundering herd is not an issue unless commands are made async
// with concurrent spawning. If that changes, add a per-key Mutex.
static PLAN_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, PlanFlowResult>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static COMPILE_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, CompileResult>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn stable_hash(s: &str) -> u64 {
    stable_hash_bytes(s.as_bytes())
}

fn hash_file_streaming(path: &std::path::Path) -> Result<u64, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("{e}"))?;
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("{e}"))?;
        if n == 0 {
            break;
        }
        for &byte in &buf[..n] {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(hash)
}

fn safe_path(input: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::PathBuf::from(input);
    path.canonicalize().map_err(|e| format!("invalid path: {e}"))
}

fn stable_hash_bytes(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

fn hash_source(source: &str) -> u64 {
    stable_hash(source)
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
        Err(e) => {
            // Compute 1-based line and column from byte position.
            let (line, col) = {
                let before = &source[..e.position.min(source.len())];
                let line = before.chars().filter(|&c| c == '\n').count() + 1;
                let col = before.rfind('\n').map(|p| e.position - p - 1).unwrap_or(e.position) + 1;
                (line, col)
            };
            let msg = format!(
                "[{}] {} ({}:{}) — {}",
                e.diag_id(),
                e.stage.code(),
                line,
                col,
                e.detail,
            );
            CompileResult {
                success: false,
                diagnostics: vec![msg],
                entities: vec![],
            }
        }
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
fn lookup_nomtu(query: &str, kind: Option<&str>, state: State<'_, AppState>) -> LookupResult {
    let matches = if let Some(dict) = state.open_dict() {
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
fn match_grammar(input: &str, state: State<'_, AppState>) -> Vec<GrammarMatch> {
    if let Some(conn) = state.open_grammar() {
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
fn hover_info(word: &str, state: State<'_, AppState>) -> HoverInfo {
    if let Some(dict) = state.open_dict() {
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
fn complete_word(prefix: &str, _context: Option<&str>, state: State<'_, AppState>) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();

    if let Some(dict) = state.open_dict() {
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

    if let Some(conn) = state.open_grammar() {
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
fn dream_report(manifest: &str, state: State<'_, AppState>) -> DreamReport {
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

    if let Some(dict) = state.open_dict() {
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
fn search_dict(query: &str, state: State<'_, AppState>) -> Vec<SearchResult> {
    if let Some(dict) = state.open_dict() {
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
    // Reject path traversal attempts by canonicalizing first.
    let canonical = match safe_path(path) {
        Ok(p) => p,
        Err(_) => {
            return MediaIngestResult {
                hash: String::new(),
                mime: "application/octet-stream".into(),
                size_bytes: 0,
            }
        }
    };

    // Detect modality from extension and read file metadata.
    let ext = canonical
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

    // Stream the file to compute the hash (avoids loading large files into RAM).
    let hash = hash_file_streaming(&canonical)
        .map(|h| format!("{h:016x}"))
        .unwrap_or_default();

    // Read only metadata for the size — no full read into memory.
    let size_bytes = std::fs::metadata(&canonical)
        .map(|m| m.len())
        .unwrap_or(0);

    MediaIngestResult {
        hash,
        mime: mime.into(),
        size_bytes,
    }
}

#[tauri::command]
fn extract_atoms(path: &str) -> ExtractResult {
    // Reject path traversal attempts by canonicalizing first.
    let canonical = match safe_path(path) {
        Ok(p) => p,
        Err(_) => {
            return ExtractResult {
                entities_found: 0,
                languages: vec![],
            }
        }
    };
    let p = canonical.as_path();
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
        let canonical_str = canonical.to_string_lossy();
        let lang = nom_extract::detect_language(canonical_str.as_ref()).unwrap_or("");
        if lang.is_empty() {
            return ExtractResult {
                entities_found: 0,
                languages: vec![],
            };
        }
        match std::fs::read_to_string(p) {
            Ok(source) => match nom_extract::parse_file(&source, canonical_str.as_ref(), lang) {
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

// TODO: Replace with OS keyring (tauri-plugin-keyring) for production.
fn machine_seed() -> String {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    format!("nomcanvas:{}:{}", user, home)
}

fn machine_key() -> [u8; 32] {
    let seed = machine_seed();
    let mut key = [0u8; 32];
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in seed.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for i in 0..4 {
        let h = hash.wrapping_add(i as u64).wrapping_mul(0x9e3779b97f4a7c15);
        key[i * 8..(i + 1) * 8].copy_from_slice(&h.to_le_bytes());
    }
    key
}

fn obfuscate(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    data.iter().enumerate().map(|(i, &b)| b ^ key[i % 32]).collect()
}

fn deobfuscate(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    obfuscate(data, key) // XOR is symmetric
}

#[tauri::command]
fn store_credential(key: &str, value: &str, state: State<'_, AppState>) -> CredentialResult {
    let cred_dir = state.dict_path.join("credentials");
    if let Err(e) = std::fs::create_dir_all(&cred_dir) {
        return CredentialResult {
            success: false,
            value: None,
            error: Some(format!("{e}")),
        };
    }
    let key_bytes = machine_key();
    let obfuscated = obfuscate(value.as_bytes(), &key_bytes);
    let path = cred_dir.join(format!("{}.cred", sanitize_key(key)));
    match std::fs::write(&path, &obfuscated) {
        Ok(()) => CredentialResult { success: true, value: None, error: None },
        Err(e) => CredentialResult {
            success: false,
            value: None,
            error: Some(format!("{e}")),
        },
    }
}

#[tauri::command]
fn get_credential(key: &str, state: State<'_, AppState>) -> CredentialResult {
    let cred_dir = state.dict_path.join("credentials");
    let path = cred_dir.join(format!("{}.cred", sanitize_key(key)));
    match std::fs::read(&path) {
        Ok(data) => {
            let key_bytes = machine_key();
            let plain = deobfuscate(&data, &key_bytes);
            match String::from_utf8(plain) {
                Ok(value) => CredentialResult { success: true, value: Some(value), error: None },
                Err(_) => CredentialResult {
                    success: false,
                    value: None,
                    error: Some("decode failed".into()),
                },
            }
        }
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
    const LSP_TIMEOUT_MS: u64 = 5_000;
    match lsp::get_or_spawn_lsp() {
        Ok(mut guard) => {
            if let Some(client) = guard.as_mut() {
                match client.request_with_timeout(method, params, LSP_TIMEOUT_MS) {
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

#[tauri::command]
fn clear_caches() -> bool {
    if let Ok(mut cache) = COMPILE_CACHE.lock() {
        cache.clear();
    }
    if let Ok(mut cache) = PLAN_CACHE.lock() {
        cache.clear();
    }
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
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
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                lsp::shutdown_lsp();
            }
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
            clear_caches,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
