/// Easing function variants for animation tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum EasingKind {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Spring,
}

/// A named animation design token.
#[derive(Debug, Clone)]
pub struct AnimationToken {
    pub name: String,
    pub duration_ms: u32,
    pub easing: EasingKind,
}

impl AnimationToken {
    /// Create a new token with `name`, `duration_ms`, and `easing`.
    pub fn new(name: &str, duration_ms: u32, easing: EasingKind) -> Self {
        Self {
            name: name.to_string(),
            duration_ms,
            easing,
        }
    }

    /// Returns `true` when the easing is [`EasingKind::Spring`].
    pub fn is_spring(&self) -> bool {
        self.easing == EasingKind::Spring
    }
}

/// Runtime registry of [`AnimationToken`]s.
pub struct AnimationRegistry {
    pub tokens: Vec<AnimationToken>,
}

impl AnimationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }

    /// Add a token to the registry.
    pub fn register(&mut self, token: AnimationToken) {
        self.tokens.push(token);
    }

    /// Find a token by name, returning `None` if not found.
    pub fn find_by_name(&self, name: &str) -> Option<&AnimationToken> {
        self.tokens.iter().find(|t| t.name == name)
    }

    /// Count of tokens whose easing is [`EasingKind::Spring`].
    pub fn spring_count(&self) -> usize {
        self.tokens.iter().filter(|t| t.is_spring()).count()
    }
}

impl Default for AnimationRegistry {
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

    #[test]
    fn animation_token_new() {
        let t = AnimationToken::new("fade-in", 300, EasingKind::EaseIn);
        assert_eq!(t.name, "fade-in");
        assert_eq!(t.duration_ms, 300);
        assert_eq!(t.easing, EasingKind::EaseIn);
    }

    #[test]
    fn animation_is_spring() {
        let spring = AnimationToken::new("bounce", 500, EasingKind::Spring);
        let linear = AnimationToken::new("slide", 200, EasingKind::Linear);
        assert!(spring.is_spring());
        assert!(!linear.is_spring());
    }

    #[test]
    fn registry_register() {
        let mut reg = AnimationRegistry::new();
        assert_eq!(reg.tokens.len(), 0);
        reg.register(AnimationToken::new("a", 100, EasingKind::Linear));
        reg.register(AnimationToken::new("b", 200, EasingKind::EaseOut));
        assert_eq!(reg.tokens.len(), 2);
    }

    #[test]
    fn registry_find() {
        let mut reg = AnimationRegistry::new();
        reg.register(AnimationToken::new("pop", 150, EasingKind::EaseInOut));
        let found = reg.find_by_name("pop");
        assert!(found.is_some());
        assert_eq!(found.unwrap().duration_ms, 150);
        assert!(reg.find_by_name("missing").is_none());
    }

    #[test]
    fn registry_spring_count() {
        let mut reg = AnimationRegistry::new();
        reg.register(AnimationToken::new("s1", 300, EasingKind::Spring));
        reg.register(AnimationToken::new("s2", 400, EasingKind::Spring));
        reg.register(AnimationToken::new("n1", 100, EasingKind::Linear));
        assert_eq!(reg.spring_count(), 2);
    }

    #[test]
    fn easing_variants() {
        let variants = [
            EasingKind::Linear,
            EasingKind::EaseIn,
            EasingKind::EaseOut,
            EasingKind::EaseInOut,
            EasingKind::Spring,
        ];
        assert_eq!(variants.len(), 5);
        assert_ne!(EasingKind::Linear, EasingKind::Spring);
    }
}
