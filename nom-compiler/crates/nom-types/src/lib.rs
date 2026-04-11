//! Shared types for the Nom compiler engine pipeline.
//!
//! Contains atom types, UIR entities, relationship kinds, and the NomtuEntry
//! dictionary entry type. All engine crates depend on this for common types.

use serde::{Deserialize, Serialize};

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

// ── NomtuEntry ───────────────────────────────────────────────────────

/// A unified nomtu entry -- identity, meaning, contract, scores,
/// provenance, and body all in one row. This IS the dictionary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomtuEntry {
    pub id: i64,
    // identity
    pub word: String,
    pub variant: Option<String>,
    pub hash: Option<String>,
    pub atom_id: Option<String>,
    // meaning
    pub describe: Option<String>,
    pub kind: Option<String>,
    pub labels: Vec<String>,
    pub concept: Option<String>,
    // contract
    pub input_type: Option<String>,
    pub output_type: Option<String>,
    pub effects: Vec<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
    // scores
    pub security: f64,
    pub performance: f64,
    pub quality: f64,
    pub reliability: f64,
    // provenance
    pub source: Option<String>,
    pub source_path: Option<String>,
    pub language: String,
    pub license: Option<String>,
    // body
    pub body: Option<String>,
    pub signature: Option<String>,
    // meta
    pub version: Option<String>,
    pub tests: i64,
    pub is_canonical: bool,
}

impl Default for NomtuEntry {
    fn default() -> Self {
        Self {
            id: 0,
            word: String::new(),
            variant: None,
            hash: None,
            atom_id: None,
            describe: None,
            kind: None,
            labels: vec![],
            concept: None,
            input_type: None,
            output_type: None,
            effects: vec![],
            pre: None,
            post: None,
            security: 0.0,
            performance: 0.0,
            quality: 0.0,
            reliability: 0.0,
            source: None,
            source_path: None,
            language: "rust".to_owned(),
            license: None,
            body: None,
            signature: None,
            version: None,
            tests: 0,
            is_canonical: false,
        }
    }
}

impl NomtuEntry {
    /// Returns true if this entry satisfies a named score threshold.
    pub fn satisfies_score(&self, metric: &str, threshold: f64) -> bool {
        let value = match metric {
            "security" => self.security,
            "performance" => self.performance,
            "quality" => self.quality,
            "reliability" => self.reliability,
            _ => 0.0,
        };
        value >= threshold
    }
}
