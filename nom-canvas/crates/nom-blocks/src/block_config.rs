use std::collections::HashMap;

use crate::flavour::Flavour;

/// Trait implemented by per-flavour configuration objects.
pub trait BlockConfig: std::any::Any + Send + Sync {
    fn flavour(&self) -> Flavour;
}

/// Central registry that maps flavours to their configuration objects.
pub struct ConfigRegistry {
    configs: HashMap<Flavour, Box<dyn BlockConfig>>,
}

impl ConfigRegistry {
    pub fn new() -> Self {
        Self { configs: HashMap::new() }
    }

    /// Register a config. Overwrites any previous entry for the same flavour.
    pub fn register(&mut self, config: Box<dyn BlockConfig>) {
        self.configs.insert(config.flavour(), config);
    }

    /// Retrieve the config for `flavour`, or `None` if not registered.
    pub fn get(&self, flavour: Flavour) -> Option<&dyn BlockConfig> {
        self.configs.get(flavour).map(|b| b.as_ref())
    }
}

impl Default for ConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavour::PROSE;

    struct ProseConfig;

    impl BlockConfig for ProseConfig {
        fn flavour(&self) -> Flavour {
            PROSE
        }
    }

    #[test]
    fn register_and_get_round_trip() {
        let mut reg = ConfigRegistry::new();
        reg.register(Box::new(ProseConfig));
        let cfg = reg.get(PROSE);
        assert!(cfg.is_some());
        assert_eq!(cfg.unwrap().flavour(), PROSE);
    }

    #[test]
    fn missing_flavour_returns_none() {
        let reg = ConfigRegistry::new();
        assert!(reg.get("nom:does-not-exist").is_none());
    }
}
