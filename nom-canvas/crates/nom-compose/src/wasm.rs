//! WASM sandbox for polyglot plugins.
//!
//! Inspired by Wasmtime's `Store<T>` + `Linker::func_wrap()` pattern.
//! MVP: in-memory Wasm module loading with a minimal host function interface.
//! Future: full Wasmtime integration with fuel metering and WASI.

/// Error type for WASM sandbox operations.
#[derive(Debug)]
pub enum WasmError {
    /// Module binary is invalid.
    InvalidModule(String),
    /// A required export is missing.
    MissingExport(String),
    /// Host function call failed.
    HostError(String),
    /// Execution trap (out of bounds, unreachable, etc.).
    Trap(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::InvalidModule(s) => write!(f, "invalid module: {s}"),
            WasmError::MissingExport(s) => write!(f, "missing export: {s}"),
            WasmError::HostError(s) => write!(f, "host error: {s}"),
            WasmError::Trap(s) => write!(f, "trap: {s}"),
        }
    }
}

impl std::error::Error for WasmError {}

/// A host function callable from guest WASM code.
pub struct HostFunc {
    pub name: String,
    pub handler: Box<dyn Fn(&[WasmValue]) -> Result<Vec<WasmValue>, WasmError> + Send + Sync>,
}

impl HostFunc {
    pub fn new(
        name: impl Into<String>,
        handler: impl Fn(&[WasmValue]) -> Result<Vec<WasmValue>, WasmError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            handler: Box::new(handler),
        }
    }
}

/// Values that can be passed between host and guest.
#[derive(Debug, Clone, PartialEq)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

/// In-memory representation of a loaded WASM module.
pub struct WasmModule {
    bytes: Vec<u8>,
    exports: Vec<String>,
}

impl WasmModule {
    /// Load from raw bytes. Validates the WASM magic number.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, WasmError> {
        if bytes.len() < 8 || &bytes[0..4] != &[0x00, 0x61, 0x73, 0x6d] {
            return Err(WasmError::InvalidModule(
                "missing WASM magic number".into(),
            ));
        }
        // MVP: parse only the export section to know available exports.
        let exports = parse_export_names(&bytes)?;
        Ok(Self { bytes, exports })
    }

    pub fn exports(&self) -> &[String] {
        &self.exports
    }

    pub fn has_export(&self, name: &str) -> bool {
        self.exports.iter().any(|e| e == name)
    }
}

/// A sandboxed WASM execution environment.
pub struct WasmSandbox {
    host_funcs: Vec<HostFunc>,
}

impl WasmSandbox {
    pub fn new() -> Self {
        Self {
            host_funcs: Vec::new(),
        }
    }

    /// Register a host function available to guest modules.
    pub fn register_host_func(&mut self, func: HostFunc) {
        self.host_funcs.push(func);
    }

    /// Wrap a simple closure as a host function.
    pub fn func_wrap<F>(
        &mut self,
        name: impl Into<String>,
        f: F,
    ) where
        F: Fn(&[WasmValue]) -> Result<Vec<WasmValue>, WasmError> + Send + Sync + 'static,
    {
        self.register_host_func(HostFunc::new(name, f));
    }

    /// Instantiate `module` and run its `main` export if present.
    pub fn instantiate_and_run(&self, module: &WasmModule) -> Result<Vec<WasmValue>, WasmError> {
        if !module.has_export("main") {
            return Err(WasmError::MissingExport("main".into()));
        }
        // MVP: simulate execution by invoking host functions registered with "main".
        for func in &self.host_funcs {
            if func.name == "main" {
                return (func.handler)(&[]);
            }
        }
        Ok(vec![WasmValue::I32(0)])
    }

    /// Lookup a host function by name.
    pub fn get_host_func(&self, name: &str) -> Option<&HostFunc> {
        self.host_funcs.iter().find(|f| f.name == name)
    }
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Minimal WASM parser: extract export names from the export section.
// ---------------------------------------------------------------------------

fn parse_export_names(bytes: &[u8]) -> Result<Vec<String>, WasmError> {
    // WASM binary format:
    // 0..4  magic
    // 4..8  version
    // sections...
    //
    // Each section: id (1 byte) | size (leb128) | payload
    // Export section id = 7
    if bytes.len() < 8 {
        return Err(WasmError::InvalidModule("too short".into()));
    }
    let mut pos = 8;
    while pos < bytes.len() {
        let section_id = bytes[pos];
        pos += 1;
        let (section_size, bytes_read) = read_leb128_u32(&bytes[pos..])?;
        pos += bytes_read;
        if section_id == 7 {
            return parse_export_section(&bytes[pos..pos + section_size as usize]);
        }
        pos += section_size as usize;
    }
    Ok(Vec::new())
}

fn parse_export_section(payload: &[u8]) -> Result<Vec<String>, WasmError> {
    let mut pos = 0;
    let (count, bytes_read) = read_leb128_u32(payload)?;
    pos += bytes_read;
    let mut names = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let (name, bytes_read) = read_name(&payload[pos..])?;
        pos += bytes_read;
        // Skip kind (1 byte) and index (leb128)
        if pos >= payload.len() {
            break;
        }
        pos += 1; // kind
        let (_, idx_bytes) = read_leb128_u32(&payload[pos..])?;
        pos += idx_bytes;
        names.push(name);
    }
    Ok(names)
}

fn read_leb128_u32(bytes: &[u8]) -> Result<(u32, usize), WasmError> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        let byte = bytes.get(pos).ok_or_else(|| {
            WasmError::InvalidModule("truncated leb128".into())
        })?;
        pos += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            return Err(WasmError::InvalidModule("leb128 overflow".into()));
        }
    }
    Ok((result, pos))
}

fn read_name(bytes: &[u8]) -> Result<(String, usize), WasmError> {
    let (len, bytes_read) = read_leb128_u32(bytes)?;
    let start = bytes_read;
    let end = start + len as usize;
    if end > bytes.len() {
        return Err(WasmError::InvalidModule("name out of bounds".into()));
    }
    let name = String::from_utf8(bytes[start..end].to_vec())
        .map_err(|_| WasmError::InvalidModule("invalid utf8 name".into()))?;
    Ok((name, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid WASM module with one export named "main".
    /// Generated from wat: (module (func (export "main")))
    const MINIMAL_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // magic
        0x01, 0x00, 0x00, 0x00, // version
        // Type section (id=1)
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
        // Function section (id=3)
        0x03, 0x02, 0x01, 0x00,
        // Export section (id=7)
        0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00,
        // Code section (id=10)
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ];

    #[test]
    fn wasm_module_from_bytes_parses_exports() {
        let module = WasmModule::from_bytes(MINIMAL_WASM.to_vec()).unwrap();
        assert!(module.has_export("main"));
        assert!(!module.has_export("missing"));
    }

    #[test]
    fn wasm_module_rejects_invalid_magic() {
        let result = WasmModule::from_bytes(vec![0x00, 0x00, 0x00, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_func_wrap_and_lookup() {
        let mut sandbox = WasmSandbox::new();
        sandbox.func_wrap("add", |args| {
            if let [WasmValue::I32(a), WasmValue::I32(b)] = args {
                Ok(vec![WasmValue::I32(a + b)])
            } else {
                Err(WasmError::HostError("bad args".into()))
            }
        });
        let func = sandbox.get_host_func("add").unwrap();
        let result = (func.handler)(&[WasmValue::I32(2), WasmValue::I32(3)]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(5)]);
    }

    #[test]
    fn sandbox_run_main_export() {
        let mut sandbox = WasmSandbox::new();
        sandbox.func_wrap("main", |_args| Ok(vec![WasmValue::I32(42)]));
        let module = WasmModule::from_bytes(MINIMAL_WASM.to_vec()).unwrap();
        let result = sandbox.instantiate_and_run(&module).unwrap();
        assert_eq!(result, vec![WasmValue::I32(42)]);
    }

    #[test]
    fn sandbox_missing_main_error() {
        let sandbox = WasmSandbox::new();
        let bad = WasmModule::from_bytes(
            vec![
                0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00,
                0x00, 0x03, 0x02, 0x01, 0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
            ]
        )
        .unwrap();
        let result = sandbox.instantiate_and_run(&bad);
        assert!(result.is_err());
    }

    #[test]
    fn wasm_value_equality() {
        assert_eq!(WasmValue::I32(1), WasmValue::I32(1));
        assert_ne!(WasmValue::I32(1), WasmValue::I32(2));
    }
}
