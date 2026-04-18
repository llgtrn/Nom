/// AuthorSession — brainstorm→nomx authoring motion for nom-compose.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorPhase {
    Brainstorm,
    Draft,
    Refinement,
    NomxConversion,
    Complete,
}

impl AuthorPhase {
    pub fn phase_name(&self) -> &str {
        match self {
            AuthorPhase::Brainstorm => "brainstorm",
            AuthorPhase::Draft => "draft",
            AuthorPhase::Refinement => "refinement",
            AuthorPhase::NomxConversion => "nomx_conversion",
            AuthorPhase::Complete => "complete",
        }
    }

    pub fn next_phase(&self) -> Option<AuthorPhase> {
        match self {
            AuthorPhase::Brainstorm => Some(AuthorPhase::Draft),
            AuthorPhase::Draft => Some(AuthorPhase::Refinement),
            AuthorPhase::Refinement => Some(AuthorPhase::NomxConversion),
            AuthorPhase::NomxConversion => Some(AuthorPhase::Complete),
            AuthorPhase::Complete => None,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, AuthorPhase::Complete)
    }
}

pub struct AuthorNote {
    pub phase: AuthorPhase,
    pub content: String,
    pub timestamp_ms: u64,
}

impl AuthorNote {
    pub fn new(phase: AuthorPhase, content: impl Into<String>, timestamp_ms: u64) -> Self {
        AuthorNote {
            phase,
            content: content.into(),
            timestamp_ms,
        }
    }
}

pub struct AuthorSession {
    pub id: u64,
    pub current_phase: AuthorPhase,
    pub notes: Vec<AuthorNote>,
}

impl AuthorSession {
    pub fn new(id: u64) -> Self {
        AuthorSession {
            id,
            current_phase: AuthorPhase::Brainstorm,
            notes: Vec::new(),
        }
    }

    pub fn add_note(&mut self, content: impl Into<String>, timestamp_ms: u64) {
        let note = AuthorNote::new(self.current_phase.clone(), content, timestamp_ms);
        self.notes.push(note);
    }

    /// Moves to the next phase. Returns false if already at Complete.
    pub fn advance_phase(&mut self) -> bool {
        match self.current_phase.next_phase() {
            Some(next) => {
                self.current_phase = next;
                true
            }
            None => false,
        }
    }

    pub fn notes_in_phase(&self, phase: &AuthorPhase) -> Vec<&AuthorNote> {
        self.notes.iter().filter(|n| &n.phase == phase).collect()
    }

    pub fn word_count_total(&self) -> usize {
        self.notes
            .iter()
            .map(|n| n.content.split_whitespace().count())
            .sum()
    }
}

pub struct NomxConversionResult {
    pub source_notes: usize,
    pub nomx_lines: usize,
    pub conversion_score: f32,
}

impl NomxConversionResult {
    pub fn estimate(session: &AuthorSession) -> Self {
        let source_notes = session.notes.len();
        let total_words = session.word_count_total();
        let nomx_lines = total_words / 5;
        let conversion_score = (nomx_lines as f32 / 20.0).min(1.0);
        NomxConversionResult {
            source_notes,
            nomx_lines,
            conversion_score,
        }
    }
}

#[cfg(test)]
mod author_session_tests {
    use super::*;

    // Test 1: AuthorPhase next_phase() chain
    #[test]
    fn phase_next_phase_chain() {
        assert_eq!(AuthorPhase::Brainstorm.next_phase(), Some(AuthorPhase::Draft));
        assert_eq!(AuthorPhase::Draft.next_phase(), Some(AuthorPhase::Refinement));
        assert_eq!(AuthorPhase::Refinement.next_phase(), Some(AuthorPhase::NomxConversion));
        assert_eq!(AuthorPhase::NomxConversion.next_phase(), Some(AuthorPhase::Complete));
        assert_eq!(AuthorPhase::Complete.next_phase(), None);
    }

    // Test 2: AuthorPhase is_complete()
    #[test]
    fn phase_is_complete() {
        assert!(!AuthorPhase::Brainstorm.is_complete());
        assert!(!AuthorPhase::Draft.is_complete());
        assert!(!AuthorPhase::Refinement.is_complete());
        assert!(!AuthorPhase::NomxConversion.is_complete());
        assert!(AuthorPhase::Complete.is_complete());
    }

    // Test 3: AuthorSession starts at Brainstorm
    #[test]
    fn session_starts_at_brainstorm() {
        let session = AuthorSession::new(1);
        assert_eq!(session.current_phase, AuthorPhase::Brainstorm);
        assert_eq!(session.id, 1);
        assert!(session.notes.is_empty());
    }

    // Test 4: add_note() increments notes count
    #[test]
    fn add_note_increments_count() {
        let mut session = AuthorSession::new(2);
        assert_eq!(session.notes.len(), 0);
        session.add_note("first note", 1000);
        assert_eq!(session.notes.len(), 1);
        session.add_note("second note", 2000);
        assert_eq!(session.notes.len(), 2);
    }

    // Test 5: advance_phase() moves forward
    #[test]
    fn advance_phase_moves_forward() {
        let mut session = AuthorSession::new(3);
        assert_eq!(session.current_phase, AuthorPhase::Brainstorm);
        let moved = session.advance_phase();
        assert!(moved);
        assert_eq!(session.current_phase, AuthorPhase::Draft);
        session.advance_phase();
        assert_eq!(session.current_phase, AuthorPhase::Refinement);
    }

    // Test 6: advance_phase() returns false at Complete
    #[test]
    fn advance_phase_false_at_complete() {
        let mut session = AuthorSession::new(4);
        // Advance through all phases to Complete
        assert!(session.advance_phase()); // Draft
        assert!(session.advance_phase()); // Refinement
        assert!(session.advance_phase()); // NomxConversion
        assert!(session.advance_phase()); // Complete
        // Now at Complete — should return false
        assert!(!session.advance_phase());
        assert_eq!(session.current_phase, AuthorPhase::Complete);
    }

    // Test 7: notes_in_phase() filters correctly
    #[test]
    fn notes_in_phase_filters_correctly() {
        let mut session = AuthorSession::new(5);
        session.add_note("brainstorm idea one", 100);
        session.add_note("brainstorm idea two", 200);
        session.advance_phase(); // -> Draft
        session.add_note("draft note", 300);

        let brainstorm_notes = session.notes_in_phase(&AuthorPhase::Brainstorm);
        assert_eq!(brainstorm_notes.len(), 2);

        let draft_notes = session.notes_in_phase(&AuthorPhase::Draft);
        assert_eq!(draft_notes.len(), 1);
        assert_eq!(draft_notes[0].content, "draft note");

        let refinement_notes = session.notes_in_phase(&AuthorPhase::Refinement);
        assert_eq!(refinement_notes.len(), 0);
    }

    // Test 8: word_count_total() sums all notes
    #[test]
    fn word_count_total_sums_all_notes() {
        let mut session = AuthorSession::new(6);
        session.add_note("one two three", 100);   // 3 words
        session.add_note("four five", 200);         // 2 words
        session.advance_phase();
        session.add_note("six seven eight nine ten", 300); // 5 words
        assert_eq!(session.word_count_total(), 10);
    }

    // Test 9: NomxConversionResult::estimate() score proportional
    #[test]
    fn nomx_conversion_result_estimate_score_proportional() {
        let mut session = AuthorSession::new(7);
        // Add 100 words total: nomx_lines = 100/5 = 20, score = min(1.0, 20/20) = 1.0
        let words_100 = "word ".repeat(100);
        session.add_note(words_100.trim(), 1000);

        let result = NomxConversionResult::estimate(&session);
        assert_eq!(result.source_notes, 1);
        assert_eq!(result.nomx_lines, 20);
        assert!((result.conversion_score - 1.0).abs() < 1e-5, "score must be 1.0");

        // With 10 words: nomx_lines = 2, score = 2/20 = 0.1
        let mut session2 = AuthorSession::new(8);
        session2.add_note("a b c d e f g h i j", 1000);
        let result2 = NomxConversionResult::estimate(&session2);
        assert_eq!(result2.nomx_lines, 2);
        assert!((result2.conversion_score - 0.1).abs() < 1e-5, "score must be 0.1, got {}", result2.conversion_score);
    }
}
