//! Multi-phase narrative → storyboard → video composition backend.
//!
//! Two pipelines live here, both using the same phase-composition pattern:
//!   1. `StoryboardPipeline` — 4 phases (plan → cinematography ∥ acting → detail → assembly)
//!   2. `NarrativePipeline` — 5 phases (analyse → script → characters → storyboard → video)
//!
//! Both are DATA-ONLY structures describing the pipeline + expected phase
//! outputs; actual LLM calls + asset rendering live in runtime crates.
//! This module owns the typed handoff records and phase-progression logic.
#![deny(unsafe_code)]

use crate::backend_trait::{CompositionBackend, ComposeSpec, ComposeOutput, ComposeError, InterruptFlag, ProgressSink};
use crate::kind::NomKind;

// ── Storyboard (4-phase) ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryboardPhase { Decompose, Cinematography, Acting, Detail, Assembly }

impl StoryboardPhase {
    pub const ORDER: &'static [StoryboardPhase] = &[
        Self::Decompose, Self::Cinematography, Self::Acting, Self::Detail, Self::Assembly,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Decompose      => 0,
            Self::Cinematography => 1,
            Self::Acting         => 2,
            Self::Detail         => 3,
            Self::Assembly       => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryboardPanel {
    pub panel_id: String,
    pub duration_ms: u32,
    pub scene_description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhotographyRule {
    pub panel_id: String,
    pub camera_angle: String,
    pub shot_scale: String,   // e.g. "wide", "medium", "close-up"
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActingDirection {
    pub panel_id: String,
    pub character: String,
    pub expression: String,
    pub body_language: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryboardResult {
    pub clip_id: String,
    pub panels: Vec<StoryboardPanel>,
    pub photography: Vec<PhotographyRule>,
    pub acting: Vec<ActingDirection>,
}

impl StoryboardResult {
    pub fn new(clip_id: impl Into<String>) -> Self {
        Self {
            clip_id: clip_id.into(),
            panels: vec![],
            photography: vec![],
            acting: vec![],
        }
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.panels.iter().map(|p| p.duration_ms as u64).sum()
    }

    /// Cross-check that every photography + acting entry references an existing panel_id.
    pub fn validate_references(&self) -> Result<(), StoryboardError> {
        let panel_ids: std::collections::HashSet<&str> =
            self.panels.iter().map(|p| p.panel_id.as_str()).collect();
        for p in &self.photography {
            if !panel_ids.contains(p.panel_id.as_str()) {
                return Err(StoryboardError::UnknownPanelRef(p.panel_id.clone()));
            }
        }
        for a in &self.acting {
            if !panel_ids.contains(a.panel_id.as_str()) {
                return Err(StoryboardError::UnknownPanelRef(a.panel_id.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoryboardError {
    #[error("photography/acting references unknown panel '{0}'")]
    UnknownPanelRef(String),
    #[error("empty storyboard (no panels)")]
    EmptyStoryboard,
}

// ── Narrative (5-phase) ─────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NarrativePhase { Analysis, Script, Characters, Storyboard, Video }

impl NarrativePhase {
    pub const ORDER: &'static [NarrativePhase] = &[
        Self::Analysis, Self::Script, Self::Characters, Self::Storyboard, Self::Video,
    ];
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NarrativeAnalysis {
    pub themes: Vec<String>,
    pub arc_summary: String,
    pub estimated_chapters: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ScriptDocument {
    pub scenes: Vec<String>,
    pub dialogue_lines: u32,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CharacterDesign {
    pub names: Vec<String>,
    pub voice_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NarrativeResult {
    pub analysis: NarrativeAnalysis,
    pub script: ScriptDocument,
    pub characters: CharacterDesign,
    pub storyboard: StoryboardResult,
    /// Content hash of the final rendered video artifact. `None` until the
    /// Video phase has produced output; distinguishing "storyboard done, video
    /// pending" from "video done" in `completed_phase()`.
    pub video_output_hash: Option<String>,
    pub usd_cost_cents: u64,
}

impl Default for NarrativeResult {
    fn default() -> Self {
        Self {
            analysis: NarrativeAnalysis::default(),
            script: ScriptDocument::default(),
            characters: CharacterDesign::default(),
            storyboard: StoryboardResult::new(""),
            video_output_hash: None,
            usd_cost_cents: 0,
        }
    }
}

impl NarrativeResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn completed_phase(&self) -> Option<NarrativePhase> {
        if self.analysis.arc_summary.is_empty() {
            if !self.analysis.themes.is_empty() {
                return Some(NarrativePhase::Analysis);
            }
            return None;
        }
        // arc_summary is populated — at least Analysis is done
        if self.script.scenes.is_empty() {
            return Some(NarrativePhase::Analysis);
        }
        if self.characters.names.is_empty() {
            return Some(NarrativePhase::Script);
        }
        if self.storyboard.panels.is_empty() {
            return Some(NarrativePhase::Characters);
        }
        if self.video_output_hash.is_none() {
            return Some(NarrativePhase::Storyboard);
        }
        Some(NarrativePhase::Video)
    }
}

// ── Stub backends ───────────────────────────────────────────────────────────

pub struct StubStoryboardBackend;

impl CompositionBackend for StubStoryboardBackend {
    fn kind(&self) -> NomKind {
        NomKind::MediaStoryboard
    }
    fn name(&self) -> &str {
        "stub-storyboard"
    }
    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "application/json".to_string(),
            cost_cents: 0,
        })
    }
}

pub struct StubNarrativeBackend;

impl CompositionBackend for StubNarrativeBackend {
    fn kind(&self) -> NomKind {
        NomKind::MediaNovelVideo
    }
    fn name(&self) -> &str {
        "stub-narrative"
    }
    fn compose(
        &self,
        _spec: &ComposeSpec,
        _progress: &dyn ProgressSink,
        _interrupt: &InterruptFlag,
    ) -> Result<ComposeOutput, ComposeError> {
        Ok(ComposeOutput {
            bytes: Vec::new(),
            mime_type: "video/mp4".to_string(),
            cost_cents: 0,
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_trait::InterruptFlag;

    struct NoopSink;
    impl ProgressSink for NoopSink {
        fn notify(&self, _percent: u32, _message: &str) {}
    }

    // StoryboardPhase

    #[test]
    fn storyboard_phase_order_has_five_entries() {
        assert_eq!(StoryboardPhase::ORDER.len(), 5);
    }

    #[test]
    fn storyboard_phase_indexes_zero_to_four() {
        assert_eq!(StoryboardPhase::Decompose.index(), 0);
        assert_eq!(StoryboardPhase::Cinematography.index(), 1);
        assert_eq!(StoryboardPhase::Acting.index(), 2);
        assert_eq!(StoryboardPhase::Detail.index(), 3);
        assert_eq!(StoryboardPhase::Assembly.index(), 4);
    }

    // StoryboardResult basics

    #[test]
    fn storyboard_result_new_defaults_empty() {
        let r = StoryboardResult::new("clip-01");
        assert_eq!(r.clip_id, "clip-01");
        assert!(r.panels.is_empty());
        assert!(r.photography.is_empty());
        assert!(r.acting.is_empty());
    }

    #[test]
    fn panel_count_and_total_duration() {
        let mut r = StoryboardResult::new("c");
        r.panels.push(StoryboardPanel { panel_id: "p1".into(), duration_ms: 1000, scene_description: "A".into() });
        r.panels.push(StoryboardPanel { panel_id: "p2".into(), duration_ms: 2000, scene_description: "B".into() });
        assert_eq!(r.panel_count(), 2);
        assert_eq!(r.total_duration_ms(), 3000);
    }

    #[test]
    fn validate_references_ok_when_all_exist() {
        let mut r = StoryboardResult::new("c");
        r.panels.push(StoryboardPanel { panel_id: "p1".into(), duration_ms: 500, scene_description: "x".into() });
        r.photography.push(PhotographyRule { panel_id: "p1".into(), camera_angle: "low".into(), shot_scale: "wide".into() });
        r.acting.push(ActingDirection { panel_id: "p1".into(), character: "hero".into(), expression: "happy".into(), body_language: "arms open".into() });
        assert!(r.validate_references().is_ok());
    }

    #[test]
    fn validate_references_unknown_photography_ref() {
        let mut r = StoryboardResult::new("c");
        r.panels.push(StoryboardPanel { panel_id: "p1".into(), duration_ms: 500, scene_description: "x".into() });
        r.photography.push(PhotographyRule { panel_id: "p99".into(), camera_angle: "high".into(), shot_scale: "close-up".into() });
        let err = r.validate_references().unwrap_err();
        assert!(matches!(err, StoryboardError::UnknownPanelRef(ref id) if id == "p99"));
    }

    #[test]
    fn validate_references_unknown_acting_ref() {
        let mut r = StoryboardResult::new("c");
        r.panels.push(StoryboardPanel { panel_id: "p1".into(), duration_ms: 500, scene_description: "x".into() });
        r.acting.push(ActingDirection { panel_id: "ghost".into(), character: "x".into(), expression: "x".into(), body_language: "x".into() });
        let err = r.validate_references().unwrap_err();
        assert!(matches!(err, StoryboardError::UnknownPanelRef(ref id) if id == "ghost"));
    }

    // NarrativePhase

    #[test]
    fn narrative_phase_order_has_five_entries() {
        assert_eq!(NarrativePhase::ORDER.len(), 5);
    }

    // NarrativeResult

    #[test]
    fn narrative_result_new_default() {
        let r = NarrativeResult::new();
        assert_eq!(r.usd_cost_cents, 0);
        assert!(r.analysis.themes.is_empty());
        assert!(r.script.scenes.is_empty());
    }

    #[test]
    fn completed_phase_none_on_fresh_result() {
        let r = NarrativeResult::new();
        assert_eq!(r.completed_phase(), None);
    }

    #[test]
    fn completed_phase_analysis_when_themes_only() {
        let mut r = NarrativeResult::new();
        r.analysis.themes.push("redemption".into());
        // arc_summary still empty → only themes → Analysis
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Analysis));
    }

    #[test]
    fn completed_phase_analysis_when_arc_summary_set_but_no_script() {
        let mut r = NarrativeResult::new();
        r.analysis.arc_summary = "hero rises".into();
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Analysis));
    }

    #[test]
    fn completed_phase_progression_script_characters_storyboard_video() {
        let mut r = NarrativeResult::new();

        r.analysis.arc_summary = "arc".into();
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Analysis));

        r.script.scenes.push("scene 1".into());
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Script));

        r.characters.names.push("Alice".into());
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Characters));

        r.storyboard.panels.push(StoryboardPanel { panel_id: "p1".into(), duration_ms: 500, scene_description: "x".into() });
        // Storyboard panels present but video_output_hash is None → Storyboard, not Video.
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Storyboard));

        r.video_output_hash = Some("abc123".into());
        assert_eq!(r.completed_phase(), Some(NarrativePhase::Video));
    }

    // Stub backends

    #[test]
    fn stub_storyboard_backend_kind() {
        let b = StubStoryboardBackend;
        assert_eq!(b.kind(), NomKind::MediaStoryboard);
    }

    #[test]
    fn stub_narrative_backend_kind() {
        let b = StubNarrativeBackend;
        assert_eq!(b.kind(), NomKind::MediaNovelVideo);
    }

    #[test]
    fn stub_storyboard_compose_returns_empty_json() {
        let b = StubStoryboardBackend;
        let spec = ComposeSpec { kind: NomKind::MediaStoryboard, params: vec![] };
        let out = b.compose(&spec, &NoopSink, &InterruptFlag::new()).unwrap();
        assert_eq!(out.mime_type, "application/json");
        assert!(out.bytes.is_empty());
    }

    #[test]
    fn stub_narrative_compose_returns_mp4() {
        let b = StubNarrativeBackend;
        let spec = ComposeSpec { kind: NomKind::MediaNovelVideo, params: vec![] };
        let out = b.compose(&spec, &NoopSink, &InterruptFlag::new()).unwrap();
        assert_eq!(out.mime_type, "video/mp4");
    }
}
