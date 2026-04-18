//! Self-hosting bootstrap stubs — parser.nom, resolver.nom, type_checker.nom
//! as .nomx-like entry representations.
//!
//! Tracks the five compiler stages toward full Nom-in-Nom self-hosting.
//! The fixpoint proof tuple `(s1_hash, s2_hash, s3_hash)` is recorded
//! permanently once s2 == s3 (byte-identical outputs).

/// The five compiler stages on the path to self-hosting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfHostStage {
    Lexer,
    Parser,
    Resolver,
    TypeChecker,
    Codegen,
}

impl SelfHostStage {
    /// Human-readable stage name.
    pub fn stage_name(&self) -> &str {
        match self {
            SelfHostStage::Lexer => "lexer",
            SelfHostStage::Parser => "parser",
            SelfHostStage::Resolver => "resolver",
            SelfHostStage::TypeChecker => "type_checker",
            SelfHostStage::Codegen => "codegen",
        }
    }

    /// Zero-based ordering index.
    pub fn stage_index(&self) -> u8 {
        match self {
            SelfHostStage::Lexer => 0,
            SelfHostStage::Parser => 1,
            SelfHostStage::Resolver => 2,
            SelfHostStage::TypeChecker => 3,
            SelfHostStage::Codegen => 4,
        }
    }

    /// Corresponding `.nomx` source filename for this stage.
    pub fn nomx_filename(&self) -> &str {
        match self {
            SelfHostStage::Lexer => "lexer.nomx",
            SelfHostStage::Parser => "parser.nomx",
            SelfHostStage::Resolver => "resolver.nomx",
            SelfHostStage::TypeChecker => "type_checker.nomx",
            SelfHostStage::Codegen => "codegen.nomx",
        }
    }
}

/// A single stage's bootstrap entry — tracks source size, content hash,
/// and whether that stage has been rewritten in Nom.
#[derive(Debug, Clone)]
pub struct SelfHostEntry {
    pub stage: SelfHostStage,
    pub content_hash: u64,
    pub source_lines: usize,
    pub is_bootstrapped: bool,
}

impl SelfHostEntry {
    /// Create a new, not-yet-bootstrapped entry.
    ///
    /// `content_hash` is seeded deterministically from the stage index
    /// so entries can be compared without real compilation output.
    pub fn new(stage: SelfHostStage, source_lines: usize) -> Self {
        let content_hash = stage.stage_index() as u64 * 1000;
        SelfHostEntry {
            stage,
            content_hash,
            source_lines,
            is_bootstrapped: false,
        }
    }

    /// Mark this stage as bootstrapped (builder pattern).
    pub fn mark_bootstrapped(mut self) -> Self {
        self.is_bootstrapped = true;
        self
    }
}

/// Registry of all five self-host entries.
#[derive(Debug, Clone)]
pub struct SelfHostRegistry {
    entries: Vec<SelfHostEntry>,
}

impl SelfHostRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        SelfHostRegistry { entries: Vec::new() }
    }

    /// Pre-populated registry reflecting current project state:
    /// the Lexer stage is complete (255 lines, bootstrapped); the
    /// remaining four stages are stubs awaiting implementation.
    pub fn seed() -> Self {
        let entries = vec![
            SelfHostEntry::new(SelfHostStage::Lexer, 255).mark_bootstrapped(),
            SelfHostEntry::new(SelfHostStage::Parser, 0),
            SelfHostEntry::new(SelfHostStage::Resolver, 0),
            SelfHostEntry::new(SelfHostStage::TypeChecker, 0),
            SelfHostEntry::new(SelfHostStage::Codegen, 0),
        ];
        SelfHostRegistry { entries }
    }

    /// Slice of all registered entries in stage order.
    pub fn entries(&self) -> &[SelfHostEntry] {
        &self.entries
    }

    /// Count of stages that have been bootstrapped.
    pub fn bootstrapped_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_bootstrapped).count()
    }

    /// First stage that has not yet been bootstrapped, if any.
    pub fn next_stage(&self) -> Option<&SelfHostEntry> {
        self.entries.iter().find(|e| !e.is_bootstrapped)
    }
}

impl Default for SelfHostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixpoint proof tuple recorded when the bootstrap compiler achieves
/// byte-identical output across two successive self-compilation rounds.
///
/// The proof is valid when `s2_hash == s3_hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfHostBootstrapProof {
    pub s1_hash: u64,
    pub s2_hash: u64,
    pub s3_hash: u64,
    pub fixpoint_reached: bool,
}

impl SelfHostBootstrapProof {
    /// Build a proof tuple; `fixpoint_reached` is set automatically.
    pub fn new(s1: u64, s2: u64, s3: u64) -> Self {
        SelfHostBootstrapProof {
            s1_hash: s1,
            s2_hash: s2,
            s3_hash: s3,
            fixpoint_reached: s2 == s3,
        }
    }

    /// Returns `true` when stage-2 and stage-3 hashes are identical —
    /// the formal proof that the Nom compiler can reproduce itself.
    pub fn is_valid_fixpoint(&self) -> bool {
        self.s2_hash == self.s3_hash
    }
}

#[cfg(test)]
mod selfhost_tests {
    use super::*;

    #[test]
    fn stage_name_matches_variant() {
        assert_eq!(SelfHostStage::Lexer.stage_name(), "lexer");
        assert_eq!(SelfHostStage::Parser.stage_name(), "parser");
        assert_eq!(SelfHostStage::Resolver.stage_name(), "resolver");
        assert_eq!(SelfHostStage::TypeChecker.stage_name(), "type_checker");
        assert_eq!(SelfHostStage::Codegen.stage_name(), "codegen");
    }

    #[test]
    fn nomx_filename_matches_stage() {
        assert_eq!(SelfHostStage::Lexer.nomx_filename(), "lexer.nomx");
        assert_eq!(SelfHostStage::Parser.nomx_filename(), "parser.nomx");
        assert_eq!(SelfHostStage::Resolver.nomx_filename(), "resolver.nomx");
        assert_eq!(SelfHostStage::TypeChecker.nomx_filename(), "type_checker.nomx");
        assert_eq!(SelfHostStage::Codegen.nomx_filename(), "codegen.nomx");
    }

    #[test]
    fn stage_index_ordering() {
        assert_eq!(SelfHostStage::Lexer.stage_index(), 0);
        assert_eq!(SelfHostStage::Parser.stage_index(), 1);
        assert_eq!(SelfHostStage::Resolver.stage_index(), 2);
        assert_eq!(SelfHostStage::TypeChecker.stage_index(), 3);
        assert_eq!(SelfHostStage::Codegen.stage_index(), 4);
    }

    #[test]
    fn new_entry_is_not_bootstrapped() {
        let entry = SelfHostEntry::new(SelfHostStage::Parser, 42);
        assert!(!entry.is_bootstrapped);
        assert_eq!(entry.source_lines, 42);
        assert_eq!(entry.content_hash, 1 * 1000); // Parser index = 1
    }

    #[test]
    fn mark_bootstrapped_sets_flag() {
        let entry = SelfHostEntry::new(SelfHostStage::Resolver, 10).mark_bootstrapped();
        assert!(entry.is_bootstrapped);
    }

    #[test]
    fn seed_creates_five_entries() {
        let registry = SelfHostRegistry::seed();
        assert_eq!(registry.entries().len(), 5);
    }

    #[test]
    fn seed_lexer_is_bootstrapped() {
        let registry = SelfHostRegistry::seed();
        let lexer = &registry.entries()[0];
        assert_eq!(lexer.stage, SelfHostStage::Lexer);
        assert!(lexer.is_bootstrapped);
        assert_eq!(lexer.source_lines, 255);
    }

    #[test]
    fn next_stage_returns_parser() {
        let registry = SelfHostRegistry::seed();
        let next = registry.next_stage().expect("should have a next stage");
        assert_eq!(next.stage, SelfHostStage::Parser);
        assert!(!next.is_bootstrapped);
    }

    #[test]
    fn bootstrap_proof_valid_fixpoint_when_s2_eq_s3() {
        let proof_valid = SelfHostBootstrapProof::new(100, 200, 200);
        assert!(proof_valid.is_valid_fixpoint());
        assert!(proof_valid.fixpoint_reached);

        let proof_invalid = SelfHostBootstrapProof::new(100, 200, 201);
        assert!(!proof_invalid.is_valid_fixpoint());
        assert!(!proof_invalid.fixpoint_reached);
    }
}
