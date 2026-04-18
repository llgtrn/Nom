//! Bitcode layer — stub binary encoding of an `IrModule`.
//!
//! `BitcodeModule` holds a flat symbol table plus raw bytes.
//! `IrToBitcode::lower` converts an `IrModule` into a `BitcodeModule`
//! using a trivial stub encoding (one symbol per function, one byte per
//! function whose value equals `name.len() as u8`).

use crate::ir::IrModule;

// ─── Section ─────────────────────────────────────────────────────────────────

/// ELF-style section classification for a bitcode symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitcodeSection {
    /// Executable code.
    Text,
    /// Initialized mutable data.
    Data,
    /// Zero-initialised data (no file bytes).
    Bss,
    /// Read-only data.
    Rodata,
}

// ─── Symbol ──────────────────────────────────────────────────────────────────

/// A single named symbol within a `BitcodeModule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitcodeSymbol {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub section: BitcodeSection,
}

impl BitcodeSymbol {
    /// Construct a new symbol.
    pub fn new(name: impl Into<String>, offset: u32, size: u32, section: BitcodeSection) -> Self {
        Self {
            name: name.into(),
            offset,
            size,
            section,
        }
    }
}

// ─── Module ──────────────────────────────────────────────────────────────────

/// A flat binary module with a symbol table and raw byte payload.
#[derive(Debug, Default)]
pub struct BitcodeModule {
    pub symbols: Vec<BitcodeSymbol>,
    pub raw_bytes: Vec<u8>,
    pub entry_point: Option<String>,
}

impl BitcodeModule {
    /// Create an empty module.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a symbol and return `&mut Self` for chaining.
    pub fn add_symbol(
        &mut self,
        name: impl Into<String>,
        offset: u32,
        size: u32,
        section: BitcodeSection,
    ) -> &mut Self {
        self.symbols.push(BitcodeSymbol::new(name, offset, size, section));
        self
    }

    /// Set the entry-point symbol name.
    pub fn set_entry(&mut self, name: &str) -> &mut Self {
        self.entry_point = Some(name.to_owned());
        self
    }

    /// Number of symbols currently in the module.
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// `true` if an entry point has been set.
    pub fn has_entry(&self) -> bool {
        self.entry_point.is_some()
    }
}

// ─── Lowering ────────────────────────────────────────────────────────────────

/// Lowers an `IrModule` into a stub `BitcodeModule`.
///
/// Stub encoding rules:
/// - One `Text`-section symbol per `IrFunction`.
/// - `raw_bytes` has one byte per function; value = `name.len() as u8`.
/// - `entry_point` = name of the first function, if any.
pub struct IrToBitcode;

impl IrToBitcode {
    /// Produce a `BitcodeModule` from the given `IrModule`.
    pub fn lower(module: &IrModule) -> BitcodeModule {
        let mut bc = BitcodeModule::new();
        let mut offset: u32 = 0;

        for func in &module.functions {
            let size = 1u32; // stub: one byte per function
            bc.add_symbol(func.name.clone(), offset, size, BitcodeSection::Text);
            bc.raw_bytes.push(func.name.len() as u8);
            offset += size;
        }

        if let Some(first) = module.functions.first() {
            bc.set_entry(&first.name);
        }

        bc
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrFunction, IrModule, IrType};

    #[test]
    fn new_empty() {
        let bc = BitcodeModule::new();
        assert_eq!(bc.symbol_count(), 0);
        assert!(bc.raw_bytes.is_empty());
        assert!(bc.entry_point.is_none());
    }

    #[test]
    fn add_symbol() {
        let mut bc = BitcodeModule::new();
        bc.add_symbol("main", 0, 16, BitcodeSection::Text);
        assert_eq!(bc.symbol_count(), 1);
        assert_eq!(bc.symbols[0].name, "main");
        assert_eq!(bc.symbols[0].offset, 0);
        assert_eq!(bc.symbols[0].size, 16);
        assert_eq!(bc.symbols[0].section, BitcodeSection::Text);
    }

    #[test]
    fn set_entry() {
        let mut bc = BitcodeModule::new();
        bc.set_entry("start");
        assert_eq!(bc.entry_point.as_deref(), Some("start"));
    }

    #[test]
    fn has_entry_false() {
        let bc = BitcodeModule::new();
        assert!(!bc.has_entry());
    }

    #[test]
    fn has_entry_true() {
        let mut bc = BitcodeModule::new();
        bc.set_entry("_start");
        assert!(bc.has_entry());
    }

    #[test]
    fn lower_empty_module() {
        let m = IrModule::new("empty");
        let bc = IrToBitcode::lower(&m);
        assert_eq!(bc.symbol_count(), 0);
        assert!(bc.raw_bytes.is_empty());
        assert!(!bc.has_entry());
    }

    #[test]
    fn lower_single_function() {
        let m = IrModule::new("pkg")
            .push_function(IrFunction::new("run", IrType::Unit));
        let bc = IrToBitcode::lower(&m);
        assert_eq!(bc.symbol_count(), 1);
        assert_eq!(bc.symbols[0].name, "run");
        assert_eq!(bc.symbols[0].section, BitcodeSection::Text);
        // stub byte = "run".len() = 3
        assert_eq!(bc.raw_bytes, vec![3u8]);
        assert_eq!(bc.entry_point.as_deref(), Some("run"));
    }

    #[test]
    fn symbol_count() {
        let mut bc = BitcodeModule::new();
        bc.add_symbol("a", 0, 4, BitcodeSection::Data);
        bc.add_symbol("b", 4, 8, BitcodeSection::Rodata);
        bc.add_symbol("c", 12, 0, BitcodeSection::Bss);
        assert_eq!(bc.symbol_count(), 3);
    }
}
