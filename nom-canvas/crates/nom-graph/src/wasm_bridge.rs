/// Target environment for WASM compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmTarget {
    Web,
    NodeJs,
    Wasi,
}

impl WasmTarget {
    pub fn target_name(&self) -> &str {
        match self {
            WasmTarget::Web => "web",
            WasmTarget::NodeJs => "nodejs",
            WasmTarget::Wasi => "wasi",
        }
    }
}

/// Describes a WASM module: its name, exported symbols, and memory layout.
#[derive(Debug, Clone)]
pub struct WasmModule {
    pub name: String,
    pub exports: Vec<String>,
    pub memory_pages: u32,
}

impl WasmModule {
    pub fn new(name: impl Into<String>, memory_pages: u32) -> Self {
        Self {
            name: name.into(),
            exports: Vec::new(),
            memory_pages,
        }
    }

    pub fn add_export(&mut self, name: impl Into<String>) {
        self.exports.push(name.into());
    }

    pub fn export_count(&self) -> usize {
        self.exports.len()
    }

    /// Returns total linear-memory size in bytes (each WASM page = 65 536 bytes).
    pub fn memory_bytes(&self) -> u64 {
        self.memory_pages as u64 * 65_536
    }

    pub fn has_export(&self, name: &str) -> bool {
        self.exports.iter().any(|e| e == name)
    }
}

/// Controls which optional WASM features are active for a build.
#[derive(Debug, Clone)]
pub struct WasmFeatureGate {
    pub features: std::collections::HashSet<String>,
}

impl WasmFeatureGate {
    pub fn new() -> Self {
        Self {
            features: std::collections::HashSet::new(),
        }
    }

    pub fn enable(&mut self, feature: impl Into<String>) {
        self.features.insert(feature.into());
    }

    pub fn disable(&mut self, feature: &str) {
        self.features.remove(feature);
    }

    pub fn is_enabled(&self, feature: &str) -> bool {
        self.features.contains(feature)
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
    }
}

impl Default for WasmFeatureGate {
    fn default() -> Self {
        Self::new()
    }
}

/// Bridges Rust types to WASM-compatible representations and manages build configuration.
#[derive(Debug, Clone)]
pub struct WasmBridge {
    pub module: WasmModule,
    pub target: WasmTarget,
    pub gate: WasmFeatureGate,
}

impl WasmBridge {
    pub fn new(module: WasmModule, target: WasmTarget) -> Self {
        Self {
            module,
            target,
            gate: WasmFeatureGate::new(),
        }
    }

    pub fn enable_feature(&mut self, f: impl Into<String>) {
        self.gate.enable(f);
    }

    /// Returns `true` when the module has at least one export (ready to bind).
    pub fn is_ready(&self) -> bool {
        self.module.export_count() > 0
    }

    /// Produces a human-readable build config string for the wasm32 target.
    pub fn build_config(&self) -> String {
        format!(
            "wasm32-{} features={}",
            self.target.target_name(),
            self.gate.feature_count()
        )
    }
}

#[cfg(test)]
mod wasm_bridge_tests {
    use super::*;

    #[test]
    fn wasm_target_target_name() {
        assert_eq!(WasmTarget::Web.target_name(), "web");
        assert_eq!(WasmTarget::NodeJs.target_name(), "nodejs");
        assert_eq!(WasmTarget::Wasi.target_name(), "wasi");
    }

    #[test]
    fn wasm_module_add_export() {
        let mut m = WasmModule::new("my_module", 4);
        assert_eq!(m.export_count(), 0);
        m.add_export("greet");
        m.add_export("run");
        assert_eq!(m.export_count(), 2);
    }

    #[test]
    fn wasm_module_memory_bytes() {
        let m = WasmModule::new("mem_test", 2);
        assert_eq!(m.memory_bytes(), 2 * 65_536);
    }

    #[test]
    fn wasm_module_has_export() {
        let mut m = WasmModule::new("exports_test", 1);
        m.add_export("init");
        assert!(m.has_export("init"));
        assert!(!m.has_export("destroy"));
    }

    #[test]
    fn wasm_feature_gate_enable_disable() {
        let mut gate = WasmFeatureGate::new();
        gate.enable("simd");
        gate.enable("threads");
        assert_eq!(gate.feature_count(), 2);
        gate.disable("simd");
        assert_eq!(gate.feature_count(), 1);
    }

    #[test]
    fn wasm_feature_gate_is_enabled() {
        let mut gate = WasmFeatureGate::new();
        assert!(!gate.is_enabled("bulk-memory"));
        gate.enable("bulk-memory");
        assert!(gate.is_enabled("bulk-memory"));
    }

    #[test]
    fn wasm_bridge_is_ready_false() {
        let module = WasmModule::new("empty", 1);
        let bridge = WasmBridge::new(module, WasmTarget::Web);
        assert!(!bridge.is_ready());
    }

    #[test]
    fn wasm_bridge_is_ready_true() {
        let mut module = WasmModule::new("ready", 1);
        module.add_export("start");
        let bridge = WasmBridge::new(module, WasmTarget::NodeJs);
        assert!(bridge.is_ready());
    }

    #[test]
    fn wasm_bridge_build_config() {
        let mut module = WasmModule::new("cfg_test", 4);
        module.add_export("run");
        let mut bridge = WasmBridge::new(module, WasmTarget::Wasi);
        bridge.enable_feature("bulk-memory");
        bridge.enable_feature("simd");
        let cfg = bridge.build_config();
        assert_eq!(cfg, "wasm32-wasi features=2");
    }
}
