#![deny(unsafe_code)]

/// Animation state for a reasoning card.
#[derive(Debug, Clone, PartialEq)]
pub enum CardState {
    Hidden,
    Entering,
    Visible,
    Exiting,
}

/// A reasoning hypothesis card that can be animated in and out of view.
#[derive(Debug, Clone)]
pub struct AnimatedReasoningCard {
    pub id: String,
    pub hypothesis: String,
    pub confidence: f32,
    pub state: CardState,
    /// Animation progress in the range `0.0..=1.0`.
    pub progress: f32,
    /// Tree depth used for indentation in the hierarchy.
    pub depth: u32,
}

impl AnimatedReasoningCard {
    /// Create a new card in the `Hidden` state.
    pub fn new(id: &str, hypothesis: &str, confidence: f32) -> Self {
        Self {
            id: id.to_string(),
            hypothesis: hypothesis.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
            state: CardState::Hidden,
            progress: 0.0,
            depth: 0,
        }
    }

    /// Advance the animation by `delta`.
    ///
    /// State transitions:
    /// - `Hidden` → `Entering` (any positive delta kicks off entry)
    /// - `Entering` at `progress >= 1.0` → `Visible`
    /// - `Exiting` at `progress >= 1.0` → `Hidden`
    pub fn advance(mut self, delta: f32) -> Self {
        match self.state {
            CardState::Hidden => {
                if delta > 0.0 {
                    self.state = CardState::Entering;
                    self.progress = (self.progress + delta).min(1.0);
                }
            }
            CardState::Entering => {
                self.progress = (self.progress + delta).min(1.0);
                if self.progress >= 1.0 {
                    self.state = CardState::Visible;
                }
            }
            CardState::Visible => {}
            CardState::Exiting => {
                self.progress = (self.progress + delta).min(1.0);
                if self.progress >= 1.0 {
                    self.state = CardState::Hidden;
                    self.progress = 0.0;
                }
            }
        }
        self
    }

    /// Transition a `Visible` card to `Exiting` and reset progress to `0.0`.
    pub fn start_exit(mut self) -> Self {
        if self.state == CardState::Visible {
            self.state = CardState::Exiting;
            self.progress = 0.0;
        }
        self
    }

    /// Returns `true` when the card is fully visible.
    pub fn is_visible(&self) -> bool {
        self.state == CardState::Visible
    }

    /// Opacity to apply when rendering.
    ///
    /// - `Hidden` → `0.0`
    /// - `Entering` → `progress`
    /// - `Visible` → `1.0`
    /// - `Exiting` → `1.0 - progress`
    pub fn display_opacity(&self) -> f32 {
        match self.state {
            CardState::Hidden => 0.0,
            CardState::Entering => self.progress,
            CardState::Visible => 1.0,
            CardState::Exiting => 1.0 - self.progress,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_card_is_hidden() {
        let card = AnimatedReasoningCard::new("c1", "gravity exists", 0.9);
        assert_eq!(card.id, "c1");
        assert_eq!(card.hypothesis, "gravity exists");
        assert!((card.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(card.state, CardState::Hidden);
        assert!((card.progress - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn advance_transitions_hidden_to_entering() {
        let card = AnimatedReasoningCard::new("c2", "light bends", 0.7);
        let card = card.advance(0.3);
        assert_eq!(card.state, CardState::Entering);
        assert!((card.progress - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn advance_completes_entering_to_visible() {
        let card = AnimatedReasoningCard::new("c3", "time dilates", 0.8);
        let card = card.advance(0.6).advance(0.6);
        assert_eq!(card.state, CardState::Visible);
        assert!((card.progress - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn start_exit_transitions_visible_to_exiting() {
        // Two advance calls needed: first takes Hidden→Entering, second Entering→Visible.
        let card = AnimatedReasoningCard::new("c4", "space curves", 0.6)
            .advance(0.5)
            .advance(0.6)
            .start_exit();
        assert_eq!(card.state, CardState::Exiting);
        assert!((card.progress - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn display_opacity_values() {
        let hidden = AnimatedReasoningCard::new("c5", "h", 0.5);
        assert!((hidden.display_opacity() - 0.0).abs() < f32::EPSILON);

        let entering = hidden.clone().advance(0.4);
        assert!((entering.display_opacity() - 0.4).abs() < 1e-5);

        let visible = entering.clone().advance(1.0);
        assert!((visible.display_opacity() - 1.0).abs() < f32::EPSILON);

        let exiting = visible.start_exit().advance(0.3);
        assert!((exiting.display_opacity() - 0.7).abs() < 1e-5);
    }
}
