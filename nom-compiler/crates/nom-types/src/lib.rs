//! Shared types for the Nom compiler engine pipeline.
//!
//! Contains atom types, UIR entities, relationship kinds, and the v2
//! content-addressed `Entry` family (Entry, EntryScores, EntryMeta,
//! EntrySignature, SecurityFinding, EntryRef, GraphEdge, Translation).
//! All engine crates depend on this for common types.

use serde::{Deserialize, Serialize};

pub mod canonical;
pub use canonical::{canonical_bytes, entry_id};

// ── UIR (Unified Intermediate Representation) ────────────────────────

pub const UIR_SCHEMA_VERSION: &str = "0.2.0";

/// A node in the Unified Intermediate Representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UirEntity {
    pub id: String,
    pub kind: String,
    pub source_path: String,
    pub language: Option<String>,
    pub labels: Vec<String>,
}

/// The semantic kind of a UIR node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UirKind {
    Function,
    Method,
    Class,
    Struct,
    Trait,
    Interface,
    Module,
    ApiEndpoint,
    SqlQuery,
    StateMachine,
    EventHandler,
    UiComponent,
    CliCommand,
    TestCase,
    Schema,
    ConfigPattern,
    Workflow,
    Pipeline,
}

impl UirKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Module => "module",
            Self::ApiEndpoint => "api_endpoint",
            Self::SqlQuery => "sql_query",
            Self::StateMachine => "state_machine",
            Self::EventHandler => "event_handler",
            Self::UiComponent => "ui_component",
            Self::CliCommand => "cli_command",
            Self::TestCase => "test_case",
            Self::Schema => "schema",
            Self::ConfigPattern => "config_pattern",
            Self::Workflow => "workflow",
            Self::Pipeline => "pipeline",
        }
    }

    pub fn all() -> &'static [&'static str] {
        &[
            "function",
            "method",
            "class",
            "struct",
            "trait",
            "interface",
            "module",
            "api_endpoint",
            "sql_query",
            "state_machine",
            "event_handler",
            "ui_component",
            "cli_command",
            "test_case",
            "schema",
            "config_pattern",
            "workflow",
            "pipeline",
        ]
    }
}

// ── Atom Types ───────────────────────────────────────────────────────

/// All supported atom kinds in the software dictionary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomKind {
    // Code atoms
    Function,
    Method,
    ApiEndpoint,
    SqlQuery,
    StateMachine,
    EventHandler,
    UiComponent,
    CliCommand,
    TestCase,
    Schema,
    ConfigPattern,
    Workflow,
    Pipeline,

    // Logic atoms (higher-level patterns)
    AuthFlow,
    RetryLogic,
    PaginationLoop,
    CacheStrategy,
    WebhookHandler,
    QueueConsumer,
    RateLimiter,
    OAuthFlow,
    EtlPipeline,
    RagPipeline,
    AgentToolLoop,

    // Infrastructure atoms
    DockerPattern,
    CiWorkflow,
    DeployConfig,
    NixModule,
    K8sManifest,
    ReverseProxy,
    TracingSetup,

    // OS atoms (for composition)
    ServiceUnit,
    PackageRecipe,
    SecurityProfile,
    BootStage,
    KernelModule,
    FilesystemLayout,
    NetworkConfig,
}

impl AtomKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::ApiEndpoint => "api_endpoint",
            Self::SqlQuery => "sql_query",
            Self::StateMachine => "state_machine",
            Self::EventHandler => "event_handler",
            Self::UiComponent => "ui_component",
            Self::CliCommand => "cli_command",
            Self::TestCase => "test_case",
            Self::Schema => "schema",
            Self::ConfigPattern => "config_pattern",
            Self::Workflow => "workflow",
            Self::Pipeline => "pipeline",
            Self::AuthFlow => "auth_flow",
            Self::RetryLogic => "retry_logic",
            Self::PaginationLoop => "pagination_loop",
            Self::CacheStrategy => "cache_strategy",
            Self::WebhookHandler => "webhook_handler",
            Self::QueueConsumer => "queue_consumer",
            Self::RateLimiter => "rate_limiter",
            Self::OAuthFlow => "oauth_flow",
            Self::EtlPipeline => "etl_pipeline",
            Self::RagPipeline => "rag_pipeline",
            Self::AgentToolLoop => "agent_tool_loop",
            Self::DockerPattern => "docker_pattern",
            Self::CiWorkflow => "ci_workflow",
            Self::DeployConfig => "deploy_config",
            Self::NixModule => "nix_module",
            Self::K8sManifest => "k8s_manifest",
            Self::ReverseProxy => "reverse_proxy",
            Self::TracingSetup => "tracing_setup",
            Self::ServiceUnit => "service_unit",
            Self::PackageRecipe => "package_recipe",
            Self::SecurityProfile => "security_profile",
            Self::BootStage => "boot_stage",
            Self::KernelModule => "kernel_module",
            Self::FilesystemLayout => "filesystem_layout",
            Self::NetworkConfig => "network_config",
        }
    }
}

/// Returns all atom kind labels.
pub fn all_atom_kinds() -> Vec<&'static str> {
    vec![
        "function",
        "method",
        "api_endpoint",
        "sql_query",
        "state_machine",
        "event_handler",
        "ui_component",
        "cli_command",
        "test_case",
        "schema",
        "config_pattern",
        "workflow",
        "pipeline",
        "auth_flow",
        "retry_logic",
        "pagination_loop",
        "cache_strategy",
        "webhook_handler",
        "queue_consumer",
        "rate_limiter",
        "oauth_flow",
        "etl_pipeline",
        "rag_pipeline",
        "agent_tool_loop",
        "docker_pattern",
        "ci_workflow",
        "deploy_config",
        "nix_module",
        "k8s_manifest",
        "reverse_proxy",
        "tracing_setup",
        "service_unit",
        "package_recipe",
        "security_profile",
        "boot_stage",
        "kernel_module",
        "filesystem_layout",
        "network_config",
    ]
}

/// Parse a string into an AtomKind. Falls back to Function for unknown kinds.
pub fn parse_atom_kind(s: &str) -> AtomKind {
    match s {
        "function" => AtomKind::Function,
        "method" => AtomKind::Method,
        "api_endpoint" => AtomKind::ApiEndpoint,
        "sql_query" => AtomKind::SqlQuery,
        "state_machine" => AtomKind::StateMachine,
        "event_handler" => AtomKind::EventHandler,
        "ui_component" => AtomKind::UiComponent,
        "cli_command" => AtomKind::CliCommand,
        "test_case" => AtomKind::TestCase,
        "schema" => AtomKind::Schema,
        "config_pattern" => AtomKind::ConfigPattern,
        "workflow" => AtomKind::Workflow,
        "pipeline" => AtomKind::Pipeline,
        "auth_flow" => AtomKind::AuthFlow,
        "retry_logic" => AtomKind::RetryLogic,
        "pagination_loop" => AtomKind::PaginationLoop,
        "cache_strategy" => AtomKind::CacheStrategy,
        "webhook_handler" => AtomKind::WebhookHandler,
        "queue_consumer" => AtomKind::QueueConsumer,
        "rate_limiter" => AtomKind::RateLimiter,
        "oauth_flow" => AtomKind::OAuthFlow,
        "etl_pipeline" => AtomKind::EtlPipeline,
        "rag_pipeline" => AtomKind::RagPipeline,
        "agent_tool_loop" => AtomKind::AgentToolLoop,
        "docker_pattern" => AtomKind::DockerPattern,
        "ci_workflow" => AtomKind::CiWorkflow,
        "deploy_config" => AtomKind::DeployConfig,
        "nix_module" => AtomKind::NixModule,
        "k8s_manifest" => AtomKind::K8sManifest,
        "reverse_proxy" => AtomKind::ReverseProxy,
        "tracing_setup" => AtomKind::TracingSetup,
        "service_unit" => AtomKind::ServiceUnit,
        "package_recipe" => AtomKind::PackageRecipe,
        "security_profile" => AtomKind::SecurityProfile,
        "boot_stage" => AtomKind::BootStage,
        "kernel_module" => AtomKind::KernelModule,
        "filesystem_layout" => AtomKind::FilesystemLayout,
        "network_config" => AtomKind::NetworkConfig,
        _ => AtomKind::Function, // fallback
    }
}

// ── Atom Signature ───────────────────────────────────────────────────

/// A function/method signature -- parameters and return type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtomSignature {
    pub params: Vec<(String, String)>,
    pub returns: Option<String>,
    pub is_async: bool,
    pub is_method: bool,
    pub visibility: String,
}

// ── Atom ─────────────────────────────────────────────────────────────

/// A semantic software atom -- the smallest unit of software meaning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub id: String,
    pub kind: AtomKind,
    pub name: String,
    pub source_path: String,
    pub language: String,
    pub labels: Vec<String>,
    pub concept: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<AtomSignature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

// ── Relationship Types ───────────────────────────────────────────────

/// All supported relationship types between atoms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipKind {
    // Dependency edges
    DependsOn,
    Imports,
    Calls,

    // Composition edges
    Provides,
    Requires,
    ConnectsTo,
    CompatibleWith,
    SubstitutesFor,

    // Structural edges
    ContainedIn,
    DeclaresProfile,
    ImplementsConcept,

    // Provenance edges
    ExtractedFrom,
    ReferencesDonor,
    CanonicalizedAs,
}

impl RelationshipKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DependsOn => "depends_on",
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::Provides => "provides",
            Self::Requires => "requires",
            Self::ConnectsTo => "connects_to",
            Self::CompatibleWith => "compatible_with",
            Self::SubstitutesFor => "substitutes_for",
            Self::ContainedIn => "contained_in",
            Self::DeclaresProfile => "declares_profile",
            Self::ImplementsConcept => "implements_concept",
            Self::ExtractedFrom => "extracted_from",
            Self::ReferencesDonor => "references_donor",
            Self::CanonicalizedAs => "canonicalized_as",
        }
    }
}

/// Returns all relationship labels.
pub fn all_relationships() -> Vec<&'static str> {
    vec![
        "depends_on",
        "imports",
        "calls",
        "provides",
        "requires",
        "connects_to",
        "compatible_with",
        "substitutes_for",
        "contained_in",
        "declares_profile",
        "implements_concept",
        "extracted_from",
        "references_donor",
        "canonicalized_as",
    ]
}

// ── v2 Entry types (hash-identity schema) ────────────────────────────
//
// The v2 dictionary schema keys every entry on
// `id = sha256(canonicalize(ast, contract))`, so two symbols that parse
// to the same AST collapse into one row, and two symbols that differ by
// a single literal produce different rows even if they share a name.
//
// Structured data (scores, findings, signatures, translations, graph
// edges) lives in typed side tables — no more JSON-in-TEXT.

/// Semantic kind of a v2 `Entry`. This categorises what the entry IS,
/// independent of the source language it was extracted from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryKind {
    Function,
    Method,
    Schema,
    ApiEndpoint,
    Ffi,
    ExternalOpaque,
    Module,
    Trait,
    Struct,
    Enum,
    TestCase,
    // §5.11.5 deliverable (2026-04-12): UX kinds promoted into
    // the shared EntryKind enum so v2 entries can carry them.
    UxPattern,
    DesignRule,
    Screen,
    UserFlow,
    Skill,
    // §5.12 app-composition kinds (2026-04-12).
    AppManifest,
    DataSource,
    Query,
    AppAction,
    AppVariable,
    Page,
    // §5.13 / §5.14 side-table-bound kinds.
    BenchmarkRun,
    FlowArtifact,
    // §5.16 media kinds.
    MediaUnit,
    Codec,
    Container,
    Other,
}

impl EntryKind {
    /// Every EntryKind in a stable order. Matches the `as_str`/
    /// `from_str` tables below — `entry_kind_roundtrip` locks the
    /// drift. Iteration 22 established the `ALL` pattern on
    /// `body_kind`; this mirrors it for the top-level EntryKind.
    pub const ALL: &'static [EntryKind] = &[
        EntryKind::Function,
        EntryKind::Method,
        EntryKind::Schema,
        EntryKind::ApiEndpoint,
        EntryKind::Ffi,
        EntryKind::ExternalOpaque,
        EntryKind::Module,
        EntryKind::Trait,
        EntryKind::Struct,
        EntryKind::Enum,
        EntryKind::TestCase,
        EntryKind::UxPattern,
        EntryKind::DesignRule,
        EntryKind::Screen,
        EntryKind::UserFlow,
        EntryKind::Skill,
        EntryKind::AppManifest,
        EntryKind::DataSource,
        EntryKind::Query,
        EntryKind::AppAction,
        EntryKind::AppVariable,
        EntryKind::Page,
        EntryKind::BenchmarkRun,
        EntryKind::FlowArtifact,
        EntryKind::MediaUnit,
        EntryKind::Codec,
        EntryKind::Container,
        EntryKind::Other,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Schema => "schema",
            Self::ApiEndpoint => "api_endpoint",
            Self::Ffi => "ffi",
            Self::ExternalOpaque => "external_opaque",
            Self::Module => "module",
            Self::Trait => "trait",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::TestCase => "test_case",
            Self::UxPattern => "ux_pattern",
            Self::DesignRule => "design_rule",
            Self::Screen => "screen",
            Self::UserFlow => "user_flow",
            Self::Skill => "skill",
            Self::AppManifest => "app_manifest",
            Self::DataSource => "data_source",
            Self::Query => "query",
            Self::AppAction => "app_action",
            Self::AppVariable => "app_variable",
            Self::Page => "page",
            Self::BenchmarkRun => "benchmark_run",
            Self::FlowArtifact => "flow_artifact",
            Self::MediaUnit => "media_unit",
            Self::Codec => "codec",
            Self::Container => "container",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "function" => Self::Function,
            "method" => Self::Method,
            "schema" => Self::Schema,
            "api_endpoint" => Self::ApiEndpoint,
            "ffi" => Self::Ffi,
            "external_opaque" => Self::ExternalOpaque,
            "module" => Self::Module,
            "trait" => Self::Trait,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "test_case" => Self::TestCase,
            "ux_pattern" => Self::UxPattern,
            "design_rule" => Self::DesignRule,
            "screen" => Self::Screen,
            "user_flow" => Self::UserFlow,
            "skill" => Self::Skill,
            "app_manifest" => Self::AppManifest,
            "data_source" => Self::DataSource,
            "query" => Self::Query,
            "app_action" => Self::AppAction,
            "app_variable" => Self::AppVariable,
            "page" => Self::Page,
            "benchmark_run" => Self::BenchmarkRun,
            "flow_artifact" => Self::FlowArtifact,
            "media_unit" => Self::MediaUnit,
            "codec" => Self::Codec,
            "container" => Self::Container,
            _ => Self::Other,
        }
    }
}

/// Translation / analysis completeness of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryStatus {
    /// Fully analysed, body_nom is canonical.
    Complete,
    /// Partially analysed — body may exist but contract/scores may be absent.
    Partial,
    /// Only the signature is known; body is unavailable (FFI, external opaque).
    Opaque,
}

impl EntryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Opaque => "opaque",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "complete" => Self::Complete,
            "opaque" => Self::Opaque,
            _ => Self::Partial,
        }
    }
}

/// The contract (pre/post + I/O types) attached to an `Entry`. Contracts
/// participate in hash identity: two entries with the same AST but
/// different contracts are distinct.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contract {
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
}

/// The primary identity row. `id` is the hex-encoded SHA-256 of
/// `canonicalize(ast, contract)`, so identity survives whitespace and
/// comment changes but reacts to any semantic edit.
///
/// `body_bytes` stores the canonical-format binary payload per §4.4.6
/// invariant 15 (PNG, AVIF, FLAC, .bc, etc.). Legacy / source-only
/// entries leave it `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub word: String,
    pub variant: Option<String>,
    pub kind: EntryKind,
    pub language: String,
    pub describe: Option<String>,
    pub concept: Option<String>,
    pub body: Option<String>,
    pub body_nom: Option<String>,
    /// Canonical-format bytes per §4.4.6 invariant 15 (PNG, AVIF, FLAC,
    /// .bc, etc.). `None` for legacy/source-only entries.
    pub body_bytes: Option<Vec<u8>>,
    /// §4.4.6: canonical format tag for `body`. Values from
    /// [`body_kind`] module (`NOM_SOURCE`, `BC`, `AVIF`, `AV1`, `AAC`,
    /// `FLAC`, …). `None` means untagged (legacy rows).
    pub body_kind: Option<String>,
    pub contract: Contract,
    pub status: EntryStatus,
    pub translation_score: Option<f32>,
    pub is_canonical: bool,
    pub deprecated_by: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Per-axis quality scores for an entry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntryScores {
    pub id: String,
    pub security: Option<f32>,
    pub reliability: Option<f32>,
    pub performance: Option<f32>,
    pub readability: Option<f32>,
    pub testability: Option<f32>,
    pub portability: Option<f32>,
    pub composability: Option<f32>,
    pub maturity: Option<f32>,
    pub overall_score: Option<f32>,
}

/// Entity-attribute-value metadata. Keyed by `(id, key, value)` so an
/// entry can carry multiple values for the same key (e.g. an entry
/// sourced from several repos gets one row per repo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryMeta {
    pub id: String,
    pub key: String,
    pub value: String,
}

/// Structured signature (1:1 with Entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntrySignature {
    pub id: String,
    pub visibility: Option<String>,
    pub is_async: bool,
    pub is_method: bool,
    pub return_type: Option<String>,
    /// JSON-encoded `[{"name":"x","type":"i32"}, ...]`. The column stays
    /// JSON because parameter lists are variable-length and we want to
    /// preserve ordering exactly.
    pub params_json: String,
}

/// Severity of a security finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Info" => Self::Info,
            "Low" => Self::Low,
            "Medium" => Self::Medium,
            "High" => Self::High,
            "Critical" => Self::Critical,
            _ => Self::Info,
        }
    }
}

/// A security-audit finding attached to an entry. 0..N per entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub finding_id: i64,
    pub id: String,
    pub severity: Severity,
    pub category: String,
    pub rule_id: Option<String>,
    pub message: Option<String>,
    pub evidence: Option<String>,
    pub line: Option<i64>,
    pub remediation: Option<String>,
}

/// A simple reference edge (entry -> entry). Used by closure walkers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryRef {
    pub from_id: String,
    pub to_id: String,
}

/// Typed graph edge between entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    Calls,
    Imports,
    Implements,
    DependsOn,
    SimilarTo,
    // §5.11.5 deliverable (2026-04-12): UX edges.
    Styles,
    Constrains,
    Recommends,
    InteractsWith,
    TransitionsTo,
    // §5.11.2 cross-platform + §5.12 app-composition.
    Specializes,
    BindsTo,
    Triggers,
    Reads,
    Writes,
    NavigatesTo,
    RunsOn,
    // §5.14 flow.
    HasFlowArtifact,
    FlowsTo,
    // §5.16 media graph.
    Encodes,
    ContainedIn,
    UsesColor,
    UsesPalette,
    Derives,
    EmbeddedGlyph,
    Frame,
    RendersOn,
    // §4.3 supersession + §5.10 lifecycle.
    SupersededBy,
    ContractMatches,
}

impl EdgeType {
    /// Every EdgeType in a stable order. `edge_type_roundtrip` locks
    /// the drift against `as_str`/`from_str`. Mirrors `EntryKind::ALL`
    /// (iter 27) and the scaffold-crate `ALL` arrays.
    pub const ALL: &'static [EdgeType] = &[
        EdgeType::Calls,
        EdgeType::Imports,
        EdgeType::Implements,
        EdgeType::DependsOn,
        EdgeType::SimilarTo,
        EdgeType::Styles,
        EdgeType::Constrains,
        EdgeType::Recommends,
        EdgeType::InteractsWith,
        EdgeType::TransitionsTo,
        EdgeType::Specializes,
        EdgeType::BindsTo,
        EdgeType::Triggers,
        EdgeType::Reads,
        EdgeType::Writes,
        EdgeType::NavigatesTo,
        EdgeType::RunsOn,
        EdgeType::HasFlowArtifact,
        EdgeType::FlowsTo,
        EdgeType::Encodes,
        EdgeType::ContainedIn,
        EdgeType::UsesColor,
        EdgeType::UsesPalette,
        EdgeType::Derives,
        EdgeType::EmbeddedGlyph,
        EdgeType::Frame,
        EdgeType::RendersOn,
        EdgeType::SupersededBy,
        EdgeType::ContractMatches,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Implements => "implements",
            Self::DependsOn => "depends_on",
            Self::SimilarTo => "similar_to",
            Self::Styles => "styles",
            Self::Constrains => "constrains",
            Self::Recommends => "recommends",
            Self::InteractsWith => "interacts_with",
            Self::TransitionsTo => "transitions_to",
            Self::Specializes => "specializes",
            Self::BindsTo => "binds_to",
            Self::Triggers => "triggers",
            Self::Reads => "reads",
            Self::Writes => "writes",
            Self::NavigatesTo => "navigates_to",
            Self::RunsOn => "runs_on",
            Self::HasFlowArtifact => "has_flow_artifact",
            Self::FlowsTo => "flows_to",
            Self::Encodes => "encodes",
            Self::ContainedIn => "contained_in",
            Self::UsesColor => "uses_color",
            Self::UsesPalette => "uses_palette",
            Self::Derives => "derives",
            Self::EmbeddedGlyph => "embedded_glyph",
            Self::Frame => "frame",
            Self::RendersOn => "renders_on",
            Self::SupersededBy => "superseded_by",
            Self::ContractMatches => "contract_matches",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "calls" => Self::Calls,
            "imports" => Self::Imports,
            "implements" => Self::Implements,
            "depends_on" => Self::DependsOn,
            "similar_to" => Self::SimilarTo,
            "styles" => Self::Styles,
            "constrains" => Self::Constrains,
            "recommends" => Self::Recommends,
            "interacts_with" => Self::InteractsWith,
            "transitions_to" => Self::TransitionsTo,
            "specializes" => Self::Specializes,
            "binds_to" => Self::BindsTo,
            "triggers" => Self::Triggers,
            "reads" => Self::Reads,
            "writes" => Self::Writes,
            "navigates_to" => Self::NavigatesTo,
            "runs_on" => Self::RunsOn,
            "has_flow_artifact" => Self::HasFlowArtifact,
            "flows_to" => Self::FlowsTo,
            "encodes" => Self::Encodes,
            "contained_in" => Self::ContainedIn,
            "uses_color" => Self::UsesColor,
            "uses_palette" => Self::UsesPalette,
            "derives" => Self::Derives,
            "embedded_glyph" => Self::EmbeddedGlyph,
            "frame" => Self::Frame,
            "renders_on" => Self::RendersOn,
            "superseded_by" => Self::SupersededBy,
            "contract_matches" => Self::ContractMatches,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub edge_id: i64,
    pub from_id: String,
    pub to_id: String,
    pub edge_type: EdgeType,
    pub confidence: f32,
}

/// A translation of an entry body into a target language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Translation {
    pub translation_id: i64,
    pub id: String,
    pub target_language: String,
    pub body: String,
    pub confidence: Option<f32>,
    pub translator_version: Option<String>,
    pub created_at: String,
}

// ── NomtuEntry (v1, legacy) ──────────────────────────────────────────
//
// Superseded by v2 `Entry` above. Retained so nom-resolver and
// nom-graph keep compiling until Task B migrates them off. New code
// MUST use `Entry` + typed side tables.

/// Canonical body-content-kind tags per §4.4.6 (plan doc 2026-04-12).
/// A body's kind identifies what format the `body` bytes/string are in.
/// Per the lean-DB directive (pivot A): the DB stores only compiled
/// artifacts and canonical media. `NOM_SOURCE` is retained for
/// user-authored `.nom` entries. Source-language tags were removed.
pub mod body_kind {
    pub const NOM_SOURCE: &str = "nom_source";
    pub const BC: &str = "bc";
    pub const AVIF: &str = "avif";
    pub const PNG: &str = "png";
    pub const JPEG: &str = "jpeg";
    pub const AV1: &str = "av1";
    pub const AAC: &str = "aac";
    pub const FLAC: &str = "flac";
    pub const OPUS: &str = "opus";
    pub const WOFF2: &str = "woff2";
    pub const GLTF: &str = "gltf";
    pub const PDF: &str = "pdf";
    /// WebM/Matroska container (§5.16.13 order #8). Carries AV1 video +
    /// Opus audio in the Matroska-flavoured container format. Distinct
    /// from raw AV1 IVF (`body_kind::AV1`) or Ogg-Opus (`body_kind::OPUS`).
    pub const WEBM: &str = "webm";
    /// MP4/ISOBMFF container (§5.16.13 order #9). Carries AV1+AAC (or
    /// legacy H.264+AAC) in the ISO Base Media File Format box structure.
    /// Distinct from raw AV1 IVF (`body_kind::AV1`) or bare AAC
    /// (`body_kind::AAC`).
    pub const MP4: &str = "mp4";
    /// HEVC/H.265 Annex-B bitstream (§5.16.13 order #10). Decode-only:
    /// read legacy iPhone videos and extract metadata. No re-encode is
    /// planned per §5.16.13. `canonical_bytes` is always the identity
    /// copy of the input bitstream.
    pub const HEVC: &str = "hevc";

    /// Every recognized body_kind tag, in a stable order. Use when
    /// enumerating kinds in help output, stats displays, or tests.
    /// MUST stay in sync with [`is_known`] — the test
    /// `body_kind_all_matches_is_known` locks the invariant.
    pub const ALL: &[&str] = &[
        NOM_SOURCE, BC, AVIF, PNG, JPEG, AV1, AAC, FLAC, OPUS, WOFF2, GLTF, PDF, WEBM, MP4, HEVC,
    ];

    /// Returns true if the string is a recognized body_kind tag.
    pub const fn is_known(s: &str) -> bool {
        // `const fn` can't use `matches!` on &str directly, so compare by bytes.
        let b = s.as_bytes();
        const fn eq(a: &[u8], b: &[u8]) -> bool {
            if a.len() != b.len() {
                return false;
            }
            let mut i = 0;
            while i < a.len() {
                if a[i] != b[i] {
                    return false;
                }
                i += 1;
            }
            true
        }
        eq(b, NOM_SOURCE.as_bytes())
            || eq(b, BC.as_bytes())
            || eq(b, AVIF.as_bytes())
            || eq(b, PNG.as_bytes())
            || eq(b, JPEG.as_bytes())
            || eq(b, AV1.as_bytes())
            || eq(b, AAC.as_bytes())
            || eq(b, FLAC.as_bytes())
            || eq(b, OPUS.as_bytes())
            || eq(b, WOFF2.as_bytes())
            || eq(b, GLTF.as_bytes())
            || eq(b, PDF.as_bytes())
            || eq(b, WEBM.as_bytes())
            || eq(b, MP4.as_bytes())
            || eq(b, HEVC.as_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomtuEntry {
    pub id: i64,
    pub word: String,
    pub variant: Option<String>,
    pub kind: String,
    pub hash: Option<String>,
    pub body_hash: Option<String>,
    pub describe: Option<String>,
    pub concept: Option<String>,
    pub labels: Vec<String>,
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub effects: Vec<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
    pub signature: Option<String>,
    pub depends_on: Vec<String>,
    pub security: f64,
    pub reliability: f64,
    pub performance: f64,
    pub readability: f64,
    pub testability: f64,
    pub portability: f64,
    pub composability: f64,
    pub maturity: f64,
    pub overall_score: f64,
    pub audit_passed: bool,
    pub audit_max_severity: Option<String>,
    pub audit_findings: Option<String>,
    pub language: String,
    pub body: Option<String>,
    pub body_kind: Option<String>,
    pub rust_body: Option<String>,
    pub translate_confidence: Option<f64>,
    pub community_id: Option<String>,
    pub callers_count: i64,
    pub callees_count: i64,
    pub is_entry_point: bool,
    pub bc_path: Option<String>,
    pub bc_hash: Option<String>,
    pub bc_size: Option<i64>,
    pub capabilities: Option<String>,
    pub supervision: Option<String>,
    pub schedule: Option<String>,
    pub version: Option<String>,
    pub tests: i64,
    pub is_canonical: bool,
    pub deprecated_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl NomtuEntry {
    /// Legacy: check whether a score dimension meets a threshold.
    /// Preserved so nom-resolver tests keep passing; new code uses
    /// `EntryScores` queries directly.
    pub fn satisfies_score(&self, dimension: &str, threshold: f64) -> bool {
        let value = match dimension {
            "security" => self.security,
            "reliability" => self.reliability,
            "performance" => self.performance,
            "readability" => self.readability,
            "testability" => self.testability,
            "portability" => self.portability,
            "composability" => self.composability,
            "maturity" => self.maturity,
            "overall_score" | "overall" => self.overall_score,
            _ => return false,
        };
        value >= threshold
    }
}

impl Default for NomtuEntry {
    fn default() -> Self {
        Self {
            id: 0,
            word: String::new(),
            variant: None,
            kind: String::new(),
            hash: None,
            body_hash: None,
            describe: None,
            concept: None,
            labels: vec![],
            input_type: None,
            output_type: None,
            effects: vec![],
            pre: None,
            post: None,
            signature: None,
            depends_on: vec![],
            security: 0.0,
            reliability: 0.0,
            performance: 0.0,
            readability: 0.0,
            testability: 0.0,
            portability: 0.0,
            composability: 0.0,
            maturity: 0.0,
            overall_score: 0.0,
            audit_passed: false,
            audit_max_severity: None,
            audit_findings: None,
            language: "rust".to_owned(),
            body: None,
            body_kind: None,
            rust_body: None,
            translate_confidence: None,
            community_id: None,
            callers_count: 0,
            callees_count: 0,
            is_entry_point: false,
            bc_path: None,
            bc_hash: None,
            bc_size: None,
            capabilities: None,
            supervision: None,
            schedule: None,
            version: None,
            tests: 0,
            is_canonical: false,
            deprecated_by: None,
            created_at: None,
            updated_at: None,
        }
    }
}

#[cfg(test)]
mod v2_tests {
    use super::*;

    #[test]
    fn entry_kind_roundtrip() {
        for kind in EntryKind::ALL {
            assert_eq!(EntryKind::from_str(kind.as_str()), *kind);
        }
        // Drift check: update this when growing the enum.
        assert_eq!(EntryKind::ALL.len(), 28);
    }

    #[test]
    fn entry_status_roundtrip() {
        for s in [EntryStatus::Complete, EntryStatus::Partial, EntryStatus::Opaque] {
            assert_eq!(EntryStatus::from_str(s.as_str()), s);
        }
    }

    #[test]
    fn severity_roundtrip() {
        for s in [
            Severity::Info,
            Severity::Low,
            Severity::Medium,
            Severity::High,
            Severity::Critical,
        ] {
            assert_eq!(Severity::from_str(s.as_str()), s);
        }
    }

    #[test]
    fn edge_type_roundtrip() {
        for e in EdgeType::ALL {
            assert_eq!(EdgeType::from_str(e.as_str()), Some(*e));
        }
        assert_eq!(EdgeType::from_str("nope"), None);
        // Drift check: update this when growing the enum.
        assert_eq!(EdgeType::ALL.len(), 29);
    }

    #[test]
    fn body_kind_all_matches_is_known() {
        // Every tag in ALL must be recognized by is_known, and
        // is_known must reject nothing in ALL. This catches drift
        // when someone adds a constant to body_kind but forgets to
        // update either ALL or the is_known match.
        for tag in body_kind::ALL {
            assert!(
                body_kind::is_known(tag),
                "body_kind::ALL contains {tag} but is_known rejects it"
            );
        }
        assert_eq!(body_kind::ALL.len(), 15); // update when growing the list
    }

    #[test]
    fn contract_default_is_empty() {
        let c = Contract::default();
        assert!(c.input_type.is_none() && c.output_type.is_none() && c.pre.is_none() && c.post.is_none());
    }
}
