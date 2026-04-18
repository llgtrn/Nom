// EvictionPolicy — cache eviction strategies

/// Describes how cache entries are evicted when capacity is exceeded.
#[derive(Debug, Clone, PartialEq)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
    NoEviction,
}

impl EvictionPolicy {
    /// Human-readable name for the policy.
    pub fn display_name(&self) -> &str {
        match self {
            EvictionPolicy::Lru => "LRU",
            EvictionPolicy::Lfu => "LFU",
            EvictionPolicy::Fifo => "FIFO",
            EvictionPolicy::NoEviction => "NoEviction",
        }
    }

    /// Returns `true` when the policy enforces a maximum cache size.
    pub fn is_size_bounded(&self) -> bool {
        matches!(self, EvictionPolicy::Lru | EvictionPolicy::Lfu | EvictionPolicy::Fifo)
    }
}

/// Configuration pairing a policy with an optional size cap.
pub struct PolicyConfig {
    pub policy: EvictionPolicy,
    pub max_size: Option<usize>,
}

impl PolicyConfig {
    /// Creates a size-bounded LRU config.
    pub fn lru(max_size: usize) -> Self {
        Self {
            policy: EvictionPolicy::Lru,
            max_size: Some(max_size),
        }
    }

    /// Creates an unbounded no-eviction config.
    pub fn no_eviction() -> Self {
        Self {
            policy: EvictionPolicy::NoEviction,
            max_size: None,
        }
    }

    /// Returns the effective maximum size (`usize::MAX` when unbounded).
    pub fn effective_max(&self) -> usize {
        self.max_size.unwrap_or(usize::MAX)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_policy_is_size_bounded() {
        assert!(EvictionPolicy::Lru.is_size_bounded());
    }

    #[test]
    fn no_eviction_unbounded() {
        assert!(!EvictionPolicy::NoEviction.is_size_bounded());
    }

    #[test]
    fn policy_display_names() {
        assert_eq!(EvictionPolicy::Lru.display_name(), "LRU");
        assert_eq!(EvictionPolicy::Lfu.display_name(), "LFU");
        assert_eq!(EvictionPolicy::Fifo.display_name(), "FIFO");
        assert_eq!(EvictionPolicy::NoEviction.display_name(), "NoEviction");
    }

    #[test]
    fn config_lru_max() {
        let cfg = PolicyConfig::lru(128);
        assert_eq!(cfg.max_size, Some(128));
        assert_eq!(cfg.policy, EvictionPolicy::Lru);
    }

    #[test]
    fn config_effective_max_none() {
        let cfg = PolicyConfig::no_eviction();
        assert_eq!(cfg.effective_max(), usize::MAX);
    }

    #[test]
    fn policy_all_variants_have_names() {
        let variants = [
            EvictionPolicy::Lru,
            EvictionPolicy::Lfu,
            EvictionPolicy::Fifo,
            EvictionPolicy::NoEviction,
        ];
        for v in &variants {
            assert!(!v.display_name().is_empty(), "{v:?} must have a non-empty display name");
        }
    }
}
