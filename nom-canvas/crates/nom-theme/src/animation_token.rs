/// CSS easing curve variants for animation tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimCurve {
    Linear,
    EaseOut,
    EaseIn,
    Spring,
}

impl AnimCurve {
    /// Returns the CSS `transition-timing-function` value for this curve.
    pub fn css_value(&self) -> &'static str {
        match self {
            AnimCurve::Linear => "linear",
            AnimCurve::EaseOut => "ease-out",
            AnimCurve::EaseIn => "ease-in",
            AnimCurve::Spring => "cubic-bezier(0.34,1.56,0.64,1)",
        }
    }

    /// Returns `true` only for `Spring`, which overshoots its target.
    pub fn is_bouncy(&self) -> bool {
        matches!(self, AnimCurve::Spring)
    }
}

// ---------------------------------------------------------------------------

/// Duration of an animation expressed in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimDuration {
    pub ms: u32,
}

impl AnimDuration {
    /// Returns the duration in seconds.
    pub fn secs(&self) -> f32 {
        self.ms as f32 / 1000.0
    }

    /// Returns `true` when the duration is zero (instant transition).
    pub fn is_instant(&self) -> bool {
        self.ms == 0
    }

    /// Returns a new `AnimDuration` scaled by `factor`.
    pub fn scale(&self, factor: f32) -> AnimDuration {
        AnimDuration {
            ms: (self.ms as f32 * factor) as u32,
        }
    }
}

// ---------------------------------------------------------------------------

/// A named animation token combining a duration and an easing curve.
#[derive(Debug, Clone)]
pub struct AnimToken {
    pub name: String,
    pub duration: AnimDuration,
    pub curve: AnimCurve,
}

impl AnimToken {
    /// Generates a CSS `transition` shorthand for `property`.
    ///
    /// Format: `"<property> <ms>ms <css-timing-function>"`
    pub fn css_transition(&self, property: &str) -> String {
        format!("{} {}ms {}", property, self.duration.ms, self.curve.css_value())
    }

    /// Returns `true` for tokens whose duration is ≤ 100 ms (micro-interactions).
    pub fn is_micro(&self) -> bool {
        self.duration.ms <= 100
    }
}

// ---------------------------------------------------------------------------

/// Motion intensity scale — maps to user preference and platform policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationScale {
    /// Zero-motion mode: satisfies `prefers-reduced-motion: reduce`.
    Reduced,
    /// Standard motion.
    Normal,
    /// Expressive / theatrical motion (1.5× longer durations).
    Expressive,
}

impl AnimationScale {
    /// Returns the duration multiplier for this scale level.
    pub fn duration_multiplier(&self) -> f32 {
        match self {
            AnimationScale::Reduced => 0.0,
            AnimationScale::Normal => 1.0,
            AnimationScale::Expressive => 1.5,
        }
    }

    /// Returns `true` for `Reduced`, which satisfies `prefers-reduced-motion`.
    pub fn is_accessible(&self) -> bool {
        matches!(self, AnimationScale::Reduced)
    }
}

// ---------------------------------------------------------------------------

/// Registry that owns a collection of [`AnimToken`] values.
#[derive(Debug, Default)]
pub struct AnimTokenRegistry {
    pub tokens: Vec<AnimToken>,
}

impl AnimTokenRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    /// Adds `token` to the registry.
    pub fn register(&mut self, token: AnimToken) {
        self.tokens.push(token);
    }

    /// Returns the first token whose `name` matches exactly, or `None`.
    pub fn find(&self, name: &str) -> Option<&AnimToken> {
        self.tokens.iter().find(|t| t.name == name)
    }

    /// Returns all tokens whose duration qualifies as a micro-interaction (≤ 100 ms).
    pub fn micro_tokens(&self) -> Vec<&AnimToken> {
        self.tokens.iter().filter(|t| t.is_micro()).collect()
    }

    /// Returns the scaled duration for every registered token under `scale`.
    pub fn scale_all(&self, scale: &AnimationScale) -> Vec<AnimDuration> {
        let multiplier = scale.duration_multiplier();
        self.tokens
            .iter()
            .map(|t| t.duration.scale(multiplier))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anim_curve_css_value() {
        assert_eq!(AnimCurve::Linear.css_value(), "linear");
        assert_eq!(AnimCurve::EaseOut.css_value(), "ease-out");
        assert_eq!(AnimCurve::EaseIn.css_value(), "ease-in");
        assert_eq!(
            AnimCurve::Spring.css_value(),
            "cubic-bezier(0.34,1.56,0.64,1)"
        );
    }

    #[test]
    fn anim_curve_is_bouncy() {
        assert!(AnimCurve::Spring.is_bouncy());
        assert!(!AnimCurve::Linear.is_bouncy());
        assert!(!AnimCurve::EaseOut.is_bouncy());
        assert!(!AnimCurve::EaseIn.is_bouncy());
    }

    #[test]
    fn anim_duration_secs() {
        let d = AnimDuration { ms: 250 };
        assert!((d.secs() - 0.25).abs() < f32::EPSILON);
        let d2 = AnimDuration { ms: 1000 };
        assert!((d2.secs() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn anim_duration_is_instant() {
        assert!(AnimDuration { ms: 0 }.is_instant());
        assert!(!AnimDuration { ms: 1 }.is_instant());
    }

    #[test]
    fn anim_duration_scale() {
        let d = AnimDuration { ms: 200 };
        assert_eq!(d.scale(2.0).ms, 400);
        assert_eq!(d.scale(0.5).ms, 100);
        assert_eq!(d.scale(0.0).ms, 0);
    }

    #[test]
    fn anim_token_css_transition_format() {
        let token = AnimToken {
            name: "fade".to_string(),
            duration: AnimDuration { ms: 300 },
            curve: AnimCurve::EaseOut,
        };
        assert_eq!(token.css_transition("opacity"), "opacity 300ms ease-out");
    }

    #[test]
    fn anim_token_is_micro() {
        let micro = AnimToken {
            name: "tap".to_string(),
            duration: AnimDuration { ms: 100 },
            curve: AnimCurve::Linear,
        };
        let normal = AnimToken {
            name: "slide".to_string(),
            duration: AnimDuration { ms: 200 },
            curve: AnimCurve::EaseOut,
        };
        assert!(micro.is_micro());
        assert!(!normal.is_micro());
    }

    #[test]
    fn animation_scale_is_accessible() {
        assert!(AnimationScale::Reduced.is_accessible());
        assert!(!AnimationScale::Normal.is_accessible());
        assert!(!AnimationScale::Expressive.is_accessible());
    }

    #[test]
    fn anim_token_registry_micro_tokens_filter() {
        let mut reg = AnimTokenRegistry::new();
        reg.register(AnimToken {
            name: "instant".to_string(),
            duration: AnimDuration { ms: 0 },
            curve: AnimCurve::Linear,
        });
        reg.register(AnimToken {
            name: "quick".to_string(),
            duration: AnimDuration { ms: 80 },
            curve: AnimCurve::EaseOut,
        });
        reg.register(AnimToken {
            name: "slow".to_string(),
            duration: AnimDuration { ms: 400 },
            curve: AnimCurve::Spring,
        });
        let micros = reg.micro_tokens();
        assert_eq!(micros.len(), 2);
        assert!(micros.iter().any(|t| t.name == "instant"));
        assert!(micros.iter().any(|t| t.name == "quick"));
        assert!(!micros.iter().any(|t| t.name == "slow"));
    }
}
