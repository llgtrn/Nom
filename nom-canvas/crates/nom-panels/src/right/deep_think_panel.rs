#![deny(unsafe_code)]

use super::hypothesis_nav::HypothesisTreeNav;
use super::reasoning_card::{AnimatedReasoningCard, CardState};

/// Renderer that aggregates animated reasoning cards with hypothesis tree navigation.
#[derive(Debug)]
pub struct DeepThinkRenderer {
    pub cards: Vec<AnimatedReasoningCard>,
    pub hypothesis_nav: HypothesisTreeNav,
    pub active_card_idx: usize,
}

impl DeepThinkRenderer {
    /// Create an empty renderer.
    pub fn new() -> Self {
        Self {
            cards: Vec::new(),
            hypothesis_nav: HypothesisTreeNav::new(),
            active_card_idx: 0,
        }
    }

    /// Append a card to the renderer.
    pub fn push_card(&mut self, card: AnimatedReasoningCard) {
        self.cards.push(card);
    }

    /// Advance the animation FSM on all cards by a fixed delta of `0.3`.
    pub fn advance_all(&mut self) {
        let cards = std::mem::take(&mut self.cards);
        self.cards = cards.into_iter().map(|c| c.advance(0.3)).collect();
    }

    /// Return a reference to the active card, if one exists.
    pub fn active_card(&self) -> Option<&AnimatedReasoningCard> {
        self.cards.get(self.active_card_idx)
    }

    /// Return references to all cards that are not in the `Hidden` state.
    pub fn visible_cards(&self) -> Vec<&AnimatedReasoningCard> {
        self.cards
            .iter()
            .filter(|c| c.state != CardState::Hidden)
            .collect()
    }

    /// Total number of cards held by this renderer.
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }
}

impl Default for DeepThinkRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card(id: &str) -> AnimatedReasoningCard {
        AnimatedReasoningCard::new(id, "hypothesis", 0.7)
    }

    #[test]
    fn new_empty() {
        let r = DeepThinkRenderer::new();
        assert_eq!(r.card_count(), 0);
        assert_eq!(r.active_card_idx, 0);
        assert!(r.active_card().is_none());
    }

    #[test]
    fn push_and_count() {
        let mut r = DeepThinkRenderer::new();
        r.push_card(make_card("c1"));
        r.push_card(make_card("c2"));
        assert_eq!(r.card_count(), 2);
    }

    #[test]
    fn advance_all_advances() {
        let mut r = DeepThinkRenderer::new();
        r.push_card(make_card("a"));
        // Before advance: all cards are Hidden.
        assert_eq!(r.cards[0].state, CardState::Hidden);
        r.advance_all();
        // After one advance(0.3): Hidden -> Entering.
        assert_eq!(r.cards[0].state, CardState::Entering);
    }

    #[test]
    fn visible_cards_excludes_hidden() {
        let mut r = DeepThinkRenderer::new();
        r.push_card(make_card("hidden")); // stays Hidden
        let entering = make_card("entering").advance(0.3); // -> Entering
        r.push_card(entering);
        let visible = make_card("visible").advance(0.6).advance(0.6); // -> Visible
        r.push_card(visible);

        let visible_refs = r.visible_cards();
        assert_eq!(visible_refs.len(), 2);
        // The hidden card must not appear.
        assert!(visible_refs.iter().all(|c| c.state != CardState::Hidden));
    }
}
