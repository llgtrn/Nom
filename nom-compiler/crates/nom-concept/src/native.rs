//! Native binary output — target description and stub code-generation.
//!
//! `NativeTarget` describes a (arch, OS) pair and produces a triple string.
//! `NativeBinary` holds the raw byte output for that target.
//! `NativeCodegen::lower_to_native` is a stub that emits one RET byte per
//! function — sufficient for fixpoint bootstrap smoke tests.

// ── TargetArch ───────────────────────────────────────────────────────────────

/// CPU architecture for native code emission.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Wasm32,
}

// ── TargetOs ─────────────────────────────────────────────────────────────────

/// Operating system / environment for native code emission.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetOs {
    Linux,
    Windows,
    Macos,
    Wasi,
}

// ── NativeTarget ─────────────────────────────────────────────────────────────

/// A (arch, OS) pair that fully identifies a native compilation target.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeTarget {
    pub arch: TargetArch,
    pub os: TargetOs,
}

impl NativeTarget {
    /// Create a new target from an arch/OS pair.
    pub fn new(arch: TargetArch, os: TargetOs) -> Self {
        Self { arch, os }
    }

    /// Return the LLVM-style target triple string for this target.
    ///
    /// Examples: `"x86_64-unknown-linux-gnu"`, `"aarch64-apple-darwin"`,
    /// `"wasm32-unknown-wasi"`.
    pub fn triple(&self) -> String {
        match (&self.arch, &self.os) {
            (TargetArch::X86_64, TargetOs::Linux) => "x86_64-unknown-linux-gnu".to_string(),
            (TargetArch::X86_64, TargetOs::Windows) => "x86_64-pc-windows-msvc".to_string(),
            (TargetArch::X86_64, TargetOs::Macos) => "x86_64-apple-darwin".to_string(),
            (TargetArch::X86_64, TargetOs::Wasi) => "x86_64-unknown-wasi".to_string(),
            (TargetArch::Aarch64, TargetOs::Linux) => "aarch64-unknown-linux-gnu".to_string(),
            (TargetArch::Aarch64, TargetOs::Windows) => "aarch64-pc-windows-msvc".to_string(),
            (TargetArch::Aarch64, TargetOs::Macos) => "aarch64-apple-darwin".to_string(),
            (TargetArch::Aarch64, TargetOs::Wasi) => "aarch64-unknown-wasi".to_string(),
            (TargetArch::Wasm32, TargetOs::Linux) => "wasm32-unknown-unknown".to_string(),
            (TargetArch::Wasm32, TargetOs::Windows) => "wasm32-unknown-unknown".to_string(),
            (TargetArch::Wasm32, TargetOs::Macos) => "wasm32-unknown-unknown".to_string(),
            (TargetArch::Wasm32, TargetOs::Wasi) => "wasm32-unknown-wasi".to_string(),
        }
    }

    /// Returns `true` when the architecture is WebAssembly.
    pub fn is_wasm(&self) -> bool {
        matches!(self.arch, TargetArch::Wasm32)
    }
}

// ── NativeBinary ─────────────────────────────────────────────────────────────

/// Raw binary output produced for a specific native target.
#[derive(Debug, Clone)]
pub struct NativeBinary {
    /// The target this binary was compiled for.
    pub target: NativeTarget,
    /// Raw bytes of the output object / executable.
    pub bytes: Vec<u8>,
    /// Name of the entry-point symbol, if one was identified.
    pub entry_symbol: Option<String>,
}

impl NativeBinary {
    /// Create an empty binary for `target` with no bytes and no entry symbol.
    pub fn new(target: NativeTarget) -> Self {
        Self {
            target,
            bytes: Vec::new(),
            entry_symbol: None,
        }
    }

    /// Builder: set the raw bytes.
    pub fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.bytes = bytes;
        self
    }

    /// Number of bytes in the binary.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` when an entry-point symbol has been recorded.
    pub fn has_entry(&self) -> bool {
        self.entry_symbol.is_some()
    }
}

// ── NativeCodegen ─────────────────────────────────────────────────────────────

/// Stub native code-generator — lowers an `IrModule` to a `NativeBinary`.
pub struct NativeCodegen;

impl NativeCodegen {
    /// Lower `module` to a native binary for `target`.
    ///
    /// **Stub implementation**: emits one `0xC3` byte (x86 RET) per function
    /// and sets `entry_symbol` to the name of the first function, if any.
    pub fn lower_to_native(module: &crate::ir::IrModule, target: NativeTarget) -> NativeBinary {
        let mut binary = NativeBinary::new(target);

        // Emit one RET byte per function as a stand-in for real machine code.
        let func_count = module.function_count();
        binary.bytes = vec![0xC3u8; func_count];

        // Record the first function name as the entry symbol.
        if func_count > 0 {
            // Retrieve the first function name via the module's function list.
            let first_name = module
                .functions
                .first()
                .map(|f| f.name.clone());
            binary.entry_symbol = first_name;
        }

        binary
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrFunction, IrModule, IrType};

    #[test]
    fn target_new() {
        let t = NativeTarget::new(TargetArch::X86_64, TargetOs::Linux);
        assert_eq!(t.arch, TargetArch::X86_64);
        assert_eq!(t.os, TargetOs::Linux);
    }

    #[test]
    fn target_triple_x86() {
        let t = NativeTarget::new(TargetArch::X86_64, TargetOs::Linux);
        assert_eq!(t.triple(), "x86_64-unknown-linux-gnu");

        let t2 = NativeTarget::new(TargetArch::Aarch64, TargetOs::Macos);
        assert_eq!(t2.triple(), "aarch64-apple-darwin");
    }

    #[test]
    fn target_is_wasm() {
        let wasm = NativeTarget::new(TargetArch::Wasm32, TargetOs::Wasi);
        assert!(wasm.is_wasm());

        let native = NativeTarget::new(TargetArch::X86_64, TargetOs::Windows);
        assert!(!native.is_wasm());
    }

    #[test]
    fn binary_new() {
        let t = NativeTarget::new(TargetArch::Aarch64, TargetOs::Linux);
        let b = NativeBinary::new(t.clone());
        assert!(b.bytes.is_empty());
        assert!(b.entry_symbol.is_none());
        assert_eq!(b.target, t);
    }

    #[test]
    fn binary_size() {
        let t = NativeTarget::new(TargetArch::X86_64, TargetOs::Macos);
        let b = NativeBinary::new(t).with_bytes(vec![0xC3, 0xC3, 0xC3]);
        assert_eq!(b.size(), 3);
        assert!(!b.has_entry());
    }

    #[test]
    fn lower_to_native_stub() {
        let module = IrModule::new("test_mod")
            .push_function(IrFunction::new("main", IrType::Unit))
            .push_function(IrFunction::new("helper", IrType::Int(64)));

        let target = NativeTarget::new(TargetArch::X86_64, TargetOs::Linux);
        let binary = NativeCodegen::lower_to_native(&module, target);

        // One RET byte per function.
        assert_eq!(binary.size(), 2);
        assert_eq!(binary.bytes, vec![0xC3u8, 0xC3u8]);
        // Entry symbol is the first function.
        assert!(binary.has_entry());
        assert_eq!(binary.entry_symbol.as_deref(), Some("main"));
    }
}
