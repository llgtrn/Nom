#[derive(Debug, Clone)]
pub enum VideoCodec {
    H264,
    Vp9,
    ProRes,
    Hevc,
}

#[derive(Debug, Clone)]
pub struct CompositionConfig {
    pub fps: u32,
    pub duration_frames: u32,
    pub width: u32,
    pub height: u32,
    pub default_codec: Option<VideoCodec>,
}

impl Default for CompositionConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            duration_frames: 90,
            width: 1920,
            height: 1080,
            default_codec: Some(VideoCodec::H264),
        }
    }
}

pub type ConfigFn = Box<dyn Fn() -> CompositionConfig + Send + Sync>;

pub struct CompositionEntry {
    pub id: String,
    pub config_fn: ConfigFn,
}

pub struct CompositionRegistry {
    entries: std::sync::Mutex<Vec<CompositionEntry>>,
}

impl CompositionRegistry {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn register(&self, id: impl Into<String>, config_fn: ConfigFn) -> Result<(), String> {
        let id = id.into();
        let mut entries = self.entries.lock().unwrap();
        if entries.iter().any(|e| e.id == id) {
            return Err(format!("composition id already registered: {}", id));
        }
        entries.push(CompositionEntry { id, config_fn });
        Ok(())
    }

    pub fn get_config(&self, id: &str) -> Option<CompositionConfig> {
        let entries = self.entries.lock().unwrap();
        entries.iter().find(|e| e.id == id).map(|e| (e.config_fn)())
    }

    pub fn list_ids(&self) -> Vec<String> {
        let entries = self.entries.lock().unwrap();
        entries.iter().map(|e| e.id.clone()).collect()
    }
}

impl Default for CompositionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_register_and_get() {
        let registry = CompositionRegistry::new();
        registry
            .register("scene-a", Box::new(|| CompositionConfig { fps: 24, ..Default::default() }))
            .unwrap();
        let cfg = registry.get_config("scene-a").expect("must find registered id");
        assert_eq!(cfg.fps, 24);
    }

    #[test]
    fn test_registry_duplicate_id_errors() {
        let registry = CompositionRegistry::new();
        registry
            .register("dup", Box::new(|| CompositionConfig::default()))
            .unwrap();
        let result = registry.register("dup", Box::new(|| CompositionConfig::default()));
        assert!(result.is_err(), "duplicate id must return Err");
        assert!(result.unwrap_err().contains("dup"));
    }

    #[test]
    fn test_registry_list_ids() {
        let registry = CompositionRegistry::new();
        registry
            .register("alpha", Box::new(|| CompositionConfig::default()))
            .unwrap();
        registry
            .register("beta", Box::new(|| CompositionConfig::default()))
            .unwrap();
        let ids = registry.list_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"alpha".to_string()));
        assert!(ids.contains(&"beta".to_string()));
    }

    #[test]
    fn test_composition_config_default_fps_30() {
        let cfg = CompositionConfig::default();
        assert_eq!(cfg.fps, 30);
        assert_eq!(cfg.duration_frames, 90);
        assert_eq!(cfg.width, 1920);
        assert_eq!(cfg.height, 1080);
        assert!(cfg.default_codec.is_some());
    }
}
