//! Animated reasoning-card progression on canvas.
//!
//! Provides [`AnimationCard`], [`CardTimeline`], and [`CardAnimator`] for
//! driving state-transition animations on canvas reasoning cards.

/// The lifecycle state of an animated card.
#[derive(Debug, Clone, PartialEq)]
pub enum CardState {
    /// Card is fully hidden and takes no visual space.
    Hidden,
    /// Card is transitioning into view.
    Entering,
    /// Card is fully visible.
    Visible,
    /// Card is transitioning out of view.
    Exiting,
}

impl CardState {
    /// Returns a static string label for the state.
    pub fn state_name(&self) -> &str {
        match self {
            CardState::Hidden => "hidden",
            CardState::Entering => "entering",
            CardState::Visible => "visible",
            CardState::Exiting => "exiting",
        }
    }

    /// Returns `true` when the card occupies visual space (Visible, Entering, or Exiting).
    pub fn is_visible(&self) -> bool {
        matches!(self, CardState::Visible | CardState::Entering | CardState::Exiting)
    }
}

/// A canvas card that can be animated through state transitions.
#[derive(Debug)]
pub struct AnimationCard {
    /// Unique identifier for the card.
    pub id: u64,
    /// Display title of the card.
    pub title: String,
    /// Current lifecycle state.
    pub state: CardState,
    /// Current opacity in the range `[0.0, 1.0]`.
    pub opacity: f32,
}

impl AnimationCard {
    /// Creates a new card in the `Hidden` state with zero opacity.
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            state: CardState::Hidden,
            opacity: 0.0,
        }
    }

    /// Transitions the card to `Visible` with full opacity.
    pub fn show(&mut self) {
        self.state = CardState::Visible;
        self.opacity = 1.0;
    }

    /// Transitions the card to `Hidden` with zero opacity.
    pub fn hide(&mut self) {
        self.state = CardState::Hidden;
        self.opacity = 0.0;
    }

    /// Sets the card opacity, clamped to `[0.0, 1.0]`.
    pub fn set_opacity(&mut self, o: f32) {
        self.opacity = o.clamp(0.0, 1.0);
    }

    /// Returns `true` when the card is in a state that produces visual output.
    pub fn is_visible(&self) -> bool {
        self.state.is_visible()
    }
}

/// A single keyframe in a card animation timeline.
#[derive(Debug, Clone)]
pub struct CardKeyframe {
    /// Time offset from the start of the timeline in milliseconds.
    pub time_ms: u64,
    /// Target opacity at this keyframe.
    pub opacity: f32,
    /// Target state at this keyframe.
    pub state: CardState,
}

impl CardKeyframe {
    /// Creates a new keyframe.
    pub fn new(time_ms: u64, opacity: f32, state: CardState) -> Self {
        Self { time_ms, opacity, state }
    }
}

/// An ordered sequence of [`CardKeyframe`]s with a fixed duration.
#[derive(Debug)]
pub struct CardTimeline {
    /// The keyframes, maintained in insertion order (not necessarily sorted).
    pub keyframes: Vec<CardKeyframe>,
    /// Total duration of the timeline in milliseconds.
    pub duration_ms: u64,
}

impl CardTimeline {
    /// Creates an empty timeline with the given duration.
    pub fn new(duration_ms: u64) -> Self {
        Self { keyframes: Vec::new(), duration_ms }
    }

    /// Appends a keyframe to the timeline.
    pub fn add_keyframe(&mut self, kf: CardKeyframe) {
        self.keyframes.push(kf);
    }

    /// Returns the number of keyframes in the timeline.
    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    /// Returns the last keyframe whose `time_ms` is ≤ `time_ms`, or `None` if no
    /// keyframe has been reached yet.
    pub fn keyframe_at(&self, time_ms: u64) -> Option<&CardKeyframe> {
        self.keyframes
            .iter()
            .filter(|kf| kf.time_ms <= time_ms)
            .last()
    }
}

/// Drives a [`CardTimeline`] forward in time and applies the current keyframe
/// to an [`AnimationCard`].
#[derive(Debug)]
pub struct CardAnimator {
    /// The timeline being driven.
    pub timeline: CardTimeline,
    /// Current playhead position in milliseconds.
    pub current_time_ms: u64,
}

impl CardAnimator {
    /// Creates a new animator starting at time zero.
    pub fn new(timeline: CardTimeline) -> Self {
        Self { timeline, current_time_ms: 0 }
    }

    /// Advances the playhead by `delta_ms`, clamped to the timeline duration.
    pub fn advance(&mut self, delta_ms: u64) {
        self.current_time_ms = self
            .current_time_ms
            .saturating_add(delta_ms)
            .min(self.timeline.duration_ms);
    }

    /// Applies the keyframe at the current playhead position to `card`.
    /// If no keyframe has been reached yet, the card is unchanged.
    pub fn apply_to(&self, card: &mut AnimationCard) {
        if let Some(kf) = self.timeline.keyframe_at(self.current_time_ms) {
            card.state = kf.state.clone();
            card.set_opacity(kf.opacity);
        }
    }

    /// Returns `true` when the playhead has reached or passed the timeline duration.
    pub fn is_finished(&self) -> bool {
        self.current_time_ms >= self.timeline.duration_ms
    }
}

#[cfg(test)]
mod animation_card_tests {
    use super::*;

    #[test]
    fn card_state_is_visible() {
        assert!(!CardState::Hidden.is_visible());
        assert!(CardState::Entering.is_visible());
        assert!(CardState::Visible.is_visible());
        assert!(CardState::Exiting.is_visible());
    }

    #[test]
    fn card_state_state_name() {
        assert_eq!(CardState::Hidden.state_name(), "hidden");
        assert_eq!(CardState::Entering.state_name(), "entering");
        assert_eq!(CardState::Visible.state_name(), "visible");
        assert_eq!(CardState::Exiting.state_name(), "exiting");
    }

    #[test]
    fn animation_card_show_sets_visible() {
        let mut card = AnimationCard::new(1, "Reasoning Step");
        assert_eq!(card.state, CardState::Hidden);
        card.show();
        assert_eq!(card.state, CardState::Visible);
        assert!((card.opacity - 1.0).abs() < f32::EPSILON);
        assert!(card.is_visible());
    }

    #[test]
    fn animation_card_hide_sets_hidden() {
        let mut card = AnimationCard::new(2, "Hidden Card");
        card.show();
        card.hide();
        assert_eq!(card.state, CardState::Hidden);
        assert!(card.opacity.abs() < f32::EPSILON);
        assert!(!card.is_visible());
    }

    #[test]
    fn animation_card_set_opacity_clamps() {
        let mut card = AnimationCard::new(3, "Clamped");
        card.set_opacity(2.5);
        assert!((card.opacity - 1.0).abs() < f32::EPSILON, "expected clamped to 1.0");
        card.set_opacity(-0.5);
        assert!(card.opacity.abs() < f32::EPSILON, "expected clamped to 0.0");
        card.set_opacity(0.7);
        assert!((card.opacity - 0.7).abs() < 1e-6, "expected 0.7");
    }

    #[test]
    fn card_timeline_add_keyframe() {
        let mut tl = CardTimeline::new(1000);
        assert_eq!(tl.keyframe_count(), 0);
        tl.add_keyframe(CardKeyframe::new(0, 0.0, CardState::Hidden));
        tl.add_keyframe(CardKeyframe::new(500, 1.0, CardState::Visible));
        assert_eq!(tl.keyframe_count(), 2);
    }

    #[test]
    fn card_timeline_keyframe_at() {
        let mut tl = CardTimeline::new(1000);
        tl.add_keyframe(CardKeyframe::new(0, 0.0, CardState::Hidden));
        tl.add_keyframe(CardKeyframe::new(300, 0.5, CardState::Entering));
        tl.add_keyframe(CardKeyframe::new(700, 1.0, CardState::Visible));

        // Before first keyframe — but 0 == first keyframe time
        let kf0 = tl.keyframe_at(0).expect("should find keyframe at t=0");
        assert_eq!(kf0.state, CardState::Hidden);

        // Between first and second
        let kf1 = tl.keyframe_at(150).expect("should find keyframe at t=150");
        assert_eq!(kf1.state, CardState::Hidden);

        // Exactly on second keyframe
        let kf2 = tl.keyframe_at(300).expect("should find keyframe at t=300");
        assert_eq!(kf2.state, CardState::Entering);

        // Past all keyframes
        let kf3 = tl.keyframe_at(1000).expect("should find keyframe at t=1000");
        assert_eq!(kf3.state, CardState::Visible);
    }

    #[test]
    fn card_animator_advance_clamps() {
        let mut tl = CardTimeline::new(500);
        tl.add_keyframe(CardKeyframe::new(0, 0.0, CardState::Hidden));
        let mut animator = CardAnimator::new(tl);

        animator.advance(300);
        assert_eq!(animator.current_time_ms, 300);

        // Advancing past duration should clamp to duration_ms
        animator.advance(400);
        assert_eq!(animator.current_time_ms, 500, "should clamp to duration");
    }

    #[test]
    fn card_animator_is_finished() {
        let mut tl = CardTimeline::new(200);
        tl.add_keyframe(CardKeyframe::new(0, 0.0, CardState::Hidden));
        tl.add_keyframe(CardKeyframe::new(200, 1.0, CardState::Visible));
        let mut animator = CardAnimator::new(tl);

        assert!(!animator.is_finished());
        animator.advance(100);
        assert!(!animator.is_finished());
        animator.advance(100);
        assert!(animator.is_finished());

        // Verify apply_to works when finished
        let mut card = AnimationCard::new(10, "Done");
        animator.apply_to(&mut card);
        assert_eq!(card.state, CardState::Visible);
        assert!((card.opacity - 1.0).abs() < f32::EPSILON);
    }
}
