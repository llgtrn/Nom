//! Bootstrap fixpoint proof — tracks the multi-stage self-hosting compilation.
//!
//! The proof is: Stage2 == Stage3 (byte-identical binaries produced by
//! successive self-hosted compilation passes).
//!
//! Stage0 is the baseline Rust compiler.
//! Stage1 is compiled by Stage0 from Nom source.
//! Stage2 is compiled by Stage1 from the same source (first self-host).
//! Stage3 is compiled by Stage2 from the same source (fixpoint candidate).
//! Fixpoint = Stage2.binary_hash == Stage3.binary_hash.

// ── Stages ───────────────────────────────────────────────────────────────────

/// One of the four compilation stages in the bootstrap fixpoint proof.
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapStage {
    /// Baseline Rust compiler (Stage 0).
    Stage0,
    /// Compiled by Stage0 from Nom source (Stage 1).
    Stage1,
    /// Compiled by Stage1 — first self-host (Stage 2).
    Stage2,
    /// Compiled by Stage2 — fixpoint candidate (Stage 3).
    Stage3,
}

impl BootstrapStage {
    /// Return the next stage in sequence, or `None` after Stage3.
    pub fn next(self) -> Option<BootstrapStage> {
        match self {
            BootstrapStage::Stage0 => Some(BootstrapStage::Stage1),
            BootstrapStage::Stage1 => Some(BootstrapStage::Stage2),
            BootstrapStage::Stage2 => Some(BootstrapStage::Stage3),
            BootstrapStage::Stage3 => None,
        }
    }

    /// Returns `true` only for Stage3, the fixpoint candidate.
    pub fn is_fixpoint_stage(&self) -> bool {
        matches!(self, BootstrapStage::Stage3)
    }

    /// Numeric index (0–3) used to address `BootstrapProof::stages`.
    fn index(&self) -> usize {
        match self {
            BootstrapStage::Stage0 => 0,
            BootstrapStage::Stage1 => 1,
            BootstrapStage::Stage2 => 2,
            BootstrapStage::Stage3 => 3,
        }
    }
}

// ── StageBuild ───────────────────────────────────────────────────────────────

/// Record of one compilation attempt for a single stage.
#[derive(Debug, Clone)]
pub struct StageBuild {
    /// Which stage this record describes.
    pub stage: BootstrapStage,
    /// SHA-256 of the output binary (`None` if not built yet).
    pub binary_hash: Option<String>,
    /// Hash of the compiler source used to produce this binary.
    pub compiler_hash: Option<String>,
    /// Whether the compilation completed without error.
    pub built: bool,
    /// Error message when compilation failed.
    pub error: Option<String>,
}

impl StageBuild {
    /// Create an unbuilt record for the given stage.
    pub fn new(stage: BootstrapStage) -> Self {
        Self {
            stage,
            binary_hash: None,
            compiler_hash: None,
            built: false,
            error: None,
        }
    }

    /// Builder: record a successful build with its hashes.
    pub fn mark_built(mut self, binary_hash: &str, compiler_hash: &str) -> Self {
        self.binary_hash = Some(binary_hash.to_string());
        self.compiler_hash = Some(compiler_hash.to_string());
        self.built = true;
        self.error = None;
        self
    }

    /// Builder: record a failed build with its error message.
    pub fn mark_failed(mut self, error: &str) -> Self {
        self.built = false;
        self.error = Some(error.to_string());
        self
    }

    /// Returns `true` when `built` is set and no error is recorded.
    pub fn is_successful(&self) -> bool {
        self.built && self.error.is_none()
    }
}

// ── BootstrapProof ────────────────────────────────────────────────────────────

/// Accumulated evidence of the multi-stage fixpoint proof.
#[derive(Debug)]
pub struct BootstrapProof {
    /// One entry per stage (indices 0–3 = Stage0–Stage3).
    pub stages: Vec<StageBuild>,
    /// `true` once Stage2 and Stage3 produce byte-identical binaries.
    pub fixpoint_achieved: bool,
    /// ISO-8601 date string recorded when fixpoint was first confirmed.
    pub fixpoint_date: Option<String>,
}

impl Default for BootstrapProof {
    fn default() -> Self {
        Self::new()
    }
}

impl BootstrapProof {
    /// Create a proof with four unbuilt stage records.
    pub fn new() -> Self {
        Self {
            stages: vec![
                StageBuild::new(BootstrapStage::Stage0),
                StageBuild::new(BootstrapStage::Stage1),
                StageBuild::new(BootstrapStage::Stage2),
                StageBuild::new(BootstrapStage::Stage3),
            ],
            fixpoint_achieved: false,
            fixpoint_date: None,
        }
    }

    /// Builder: replace the record for `build.stage` with `build`.
    pub fn record_stage(mut self, build: StageBuild) -> Self {
        let idx = build.stage.index();
        self.stages[idx] = build;
        self
    }

    /// Check whether Stage2 and Stage3 produced identical binaries.
    ///
    /// Sets `self.fixpoint_achieved = true` and returns `true` when both
    /// hashes are present, non-empty, and equal.
    pub fn check_fixpoint(&mut self) -> bool {
        let s2 = self.stages[2]
            .binary_hash
            .as_deref()
            .filter(|h| !h.is_empty());
        let s3 = self.stages[3]
            .binary_hash
            .as_deref()
            .filter(|h| !h.is_empty());

        let achieved = matches!((s2, s3), (Some(a), Some(b)) if a == b);
        self.fixpoint_achieved = achieved;
        achieved
    }

    /// Return `(s1_hash, s2_hash, s3_hash)` when all three stages built
    /// successfully, `None` otherwise.
    pub fn proof_tuple(&self) -> Option<(String, String, String)> {
        let s1 = self.stages[1]
            .binary_hash
            .as_deref()
            .filter(|h| !h.is_empty())?;
        let s2 = self.stages[2]
            .binary_hash
            .as_deref()
            .filter(|h| !h.is_empty())?;
        let s3 = self.stages[3]
            .binary_hash
            .as_deref()
            .filter(|h| !h.is_empty())?;

        if self.stages[1].is_successful()
            && self.stages[2].is_successful()
            && self.stages[3].is_successful()
        {
            Some((s1.to_string(), s2.to_string(), s3.to_string()))
        } else {
            None
        }
    }

    /// Count how many stages have `built == true`.
    pub fn stages_complete(&self) -> usize {
        self.stages.iter().filter(|s| s.built).count()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_stage_next() {
        assert_eq!(BootstrapStage::Stage0.next(), Some(BootstrapStage::Stage1));
        assert_eq!(BootstrapStage::Stage1.next(), Some(BootstrapStage::Stage2));
        assert_eq!(BootstrapStage::Stage2.next(), Some(BootstrapStage::Stage3));
        assert_eq!(BootstrapStage::Stage3.next(), None);
    }

    #[test]
    fn bootstrap_stage_is_fixpoint() {
        assert!(!BootstrapStage::Stage0.is_fixpoint_stage());
        assert!(!BootstrapStage::Stage1.is_fixpoint_stage());
        assert!(!BootstrapStage::Stage2.is_fixpoint_stage());
        assert!(BootstrapStage::Stage3.is_fixpoint_stage());
    }

    #[test]
    fn stage_build_new_and_mark_built() {
        let build = StageBuild::new(BootstrapStage::Stage1).mark_built("abc123", "src456");
        assert!(build.built);
        assert_eq!(build.binary_hash.as_deref(), Some("abc123"));
        assert_eq!(build.compiler_hash.as_deref(), Some("src456"));
        assert!(build.error.is_none());
        assert!(build.is_successful());
    }

    #[test]
    fn stage_build_mark_failed_is_not_successful() {
        let build = StageBuild::new(BootstrapStage::Stage2).mark_failed("link error");
        assert!(!build.built);
        assert_eq!(build.error.as_deref(), Some("link error"));
        assert!(!build.is_successful());
    }

    #[test]
    fn bootstrap_proof_new_has_four_unbuilt_stages() {
        let proof = BootstrapProof::new();
        assert_eq!(proof.stages.len(), 4);
        assert!(proof.stages.iter().all(|s| !s.built));
        assert!(!proof.fixpoint_achieved);
        assert_eq!(proof.stages_complete(), 0);
    }

    #[test]
    fn bootstrap_proof_record_stage_and_stages_complete() {
        let proof = BootstrapProof::new()
            .record_stage(StageBuild::new(BootstrapStage::Stage0).mark_built("h0", "c0"))
            .record_stage(StageBuild::new(BootstrapStage::Stage1).mark_built("h1", "c1"));
        assert_eq!(proof.stages_complete(), 2);
        assert_eq!(proof.stages[0].binary_hash.as_deref(), Some("h0"));
        assert_eq!(proof.stages[1].binary_hash.as_deref(), Some("h1"));
    }

    #[test]
    fn bootstrap_proof_check_fixpoint() {
        // Matching hashes → fixpoint achieved.
        let mut proof = BootstrapProof::new()
            .record_stage(StageBuild::new(BootstrapStage::Stage2).mark_built("same", "c2"))
            .record_stage(StageBuild::new(BootstrapStage::Stage3).mark_built("same", "c3"));
        assert!(proof.check_fixpoint());
        assert!(proof.fixpoint_achieved);

        // Mismatched hashes → fixpoint not achieved.
        let mut proof2 = BootstrapProof::new()
            .record_stage(StageBuild::new(BootstrapStage::Stage2).mark_built("aaa", "c2"))
            .record_stage(StageBuild::new(BootstrapStage::Stage3).mark_built("bbb", "c3"));
        assert!(!proof2.check_fixpoint());
        assert!(!proof2.fixpoint_achieved);
    }
}
