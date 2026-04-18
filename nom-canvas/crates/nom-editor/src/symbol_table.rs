//! Symbol table primitives for the Nom editor.

/// The syntactic category of a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Type,
    Constant,
    Module,
}

impl SymbolKind {
    /// Returns `true` only for callable symbols (functions).
    pub fn is_callable(&self) -> bool {
        matches!(self, SymbolKind::Function)
    }

    /// Returns a short icon string for display purposes.
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Variable => "var",
            SymbolKind::Type => "type",
            SymbolKind::Constant => "const",
            SymbolKind::Module => "mod",
        }
    }
}

/// Visibility level of a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolVisibility {
    Public,
    Private,
    Internal,
}

impl SymbolVisibility {
    /// Returns `true` only for publicly exported symbols.
    pub fn is_exported(&self) -> bool {
        matches!(self, SymbolVisibility::Public)
    }

    /// Numeric code for the visibility level: Public=0, Internal=1, Private=2.
    pub fn visibility_code(&self) -> u8 {
        match self {
            SymbolVisibility::Public => 0,
            SymbolVisibility::Internal => 1,
            SymbolVisibility::Private => 2,
        }
    }
}

/// A single entry in the symbol table.
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: SymbolVisibility,
    pub line: u32,
}

impl SymbolEntry {
    /// Formats the entry as `"<icon> <name> [<vis_code>:<line>]"`.
    pub fn display(&self) -> String {
        format!(
            "{} {} [{}:{}]",
            self.kind.icon(),
            self.name,
            self.visibility.visibility_code(),
            self.line
        )
    }

    /// Returns `true` when the symbol is both callable and publicly exported.
    pub fn is_public_callable(&self) -> bool {
        self.kind.is_callable() && self.visibility.is_exported()
    }
}

/// A collection of `SymbolEntry` values indexed for lookup.
#[derive(Debug, Default)]
pub struct SymbolTable {
    pub entries: Vec<SymbolEntry>,
}

impl SymbolTable {
    /// Creates an empty symbol table.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Appends an entry to the table.
    pub fn insert(&mut self, entry: SymbolEntry) {
        self.entries.push(entry);
    }

    /// Returns the first entry whose name matches exactly, or `None`.
    pub fn find_by_name(&self, name: &str) -> Option<&SymbolEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Returns all entries that are publicly exported.
    pub fn public_entries(&self) -> Vec<&SymbolEntry> {
        self.entries
            .iter()
            .filter(|e| e.visibility.is_exported())
            .collect()
    }

    /// Returns the number of callable entries in the table.
    pub fn callable_count(&self) -> usize {
        self.entries.iter().filter(|e| e.kind.is_callable()).count()
    }
}

/// Utility for resolving a slice of names against a `SymbolTable`.
pub struct SymbolResolver;

impl SymbolResolver {
    /// Maps each name to the first matching entry, returning `None` when absent.
    pub fn resolve_all<'a>(
        table: &'a SymbolTable,
        names: &[&str],
    ) -> Vec<Option<&'a SymbolEntry>> {
        names.iter().map(|n| table.find_by_name(n)).collect()
    }

    /// Counts how many names could not be resolved in the table.
    pub fn unresolved_count(table: &SymbolTable, names: &[&str]) -> usize {
        Self::resolve_all(table, names)
            .into_iter()
            .filter(|r| r.is_none())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. SymbolKind::is_callable — Function is callable, others are not.
    #[test]
    fn test_symbol_kind_is_callable() {
        assert!(SymbolKind::Function.is_callable());
        assert!(!SymbolKind::Variable.is_callable());
        assert!(!SymbolKind::Type.is_callable());
        assert!(!SymbolKind::Constant.is_callable());
        assert!(!SymbolKind::Module.is_callable());
    }

    // 2. SymbolKind::icon returns the correct short string for each variant.
    #[test]
    fn test_symbol_kind_icon() {
        assert_eq!(SymbolKind::Function.icon(), "fn");
        assert_eq!(SymbolKind::Variable.icon(), "var");
        assert_eq!(SymbolKind::Type.icon(), "type");
        assert_eq!(SymbolKind::Constant.icon(), "const");
        assert_eq!(SymbolKind::Module.icon(), "mod");
    }

    // 3. SymbolVisibility::is_exported — only Public returns true.
    #[test]
    fn test_symbol_visibility_is_exported() {
        assert!(SymbolVisibility::Public.is_exported());
        assert!(!SymbolVisibility::Private.is_exported());
        assert!(!SymbolVisibility::Internal.is_exported());
    }

    // 4. SymbolVisibility::visibility_code returns the correct numeric codes.
    #[test]
    fn test_symbol_visibility_code() {
        assert_eq!(SymbolVisibility::Public.visibility_code(), 0);
        assert_eq!(SymbolVisibility::Internal.visibility_code(), 1);
        assert_eq!(SymbolVisibility::Private.visibility_code(), 2);
    }

    // 5. SymbolEntry::display produces the expected formatted string.
    #[test]
    fn test_symbol_entry_display() {
        let entry = SymbolEntry {
            name: "process".to_string(),
            kind: SymbolKind::Function,
            visibility: SymbolVisibility::Public,
            line: 42,
        };
        assert_eq!(entry.display(), "fn process [0:42]");
    }

    // 6. SymbolEntry::is_public_callable is true only when callable AND exported.
    #[test]
    fn test_symbol_entry_is_public_callable() {
        let pub_fn = SymbolEntry {
            name: "open".to_string(),
            kind: SymbolKind::Function,
            visibility: SymbolVisibility::Public,
            line: 1,
        };
        let priv_fn = SymbolEntry {
            name: "helper".to_string(),
            kind: SymbolKind::Function,
            visibility: SymbolVisibility::Private,
            line: 2,
        };
        let pub_var = SymbolEntry {
            name: "COUNT".to_string(),
            kind: SymbolKind::Variable,
            visibility: SymbolVisibility::Public,
            line: 3,
        };
        assert!(pub_fn.is_public_callable());
        assert!(!priv_fn.is_public_callable());
        assert!(!pub_var.is_public_callable());
    }

    // 7. SymbolTable::find_by_name returns Some for present names, None for absent.
    #[test]
    fn test_symbol_table_find_by_name() {
        let mut table = SymbolTable::new();
        table.insert(SymbolEntry {
            name: "alpha".to_string(),
            kind: SymbolKind::Variable,
            visibility: SymbolVisibility::Public,
            line: 10,
        });
        assert!(table.find_by_name("alpha").is_some());
        assert!(table.find_by_name("beta").is_none());
    }

    // 8. SymbolTable::public_entries filters to only exported entries.
    #[test]
    fn test_symbol_table_public_entries() {
        let mut table = SymbolTable::new();
        table.insert(SymbolEntry {
            name: "pub_fn".to_string(),
            kind: SymbolKind::Function,
            visibility: SymbolVisibility::Public,
            line: 1,
        });
        table.insert(SymbolEntry {
            name: "priv_fn".to_string(),
            kind: SymbolKind::Function,
            visibility: SymbolVisibility::Private,
            line: 2,
        });
        table.insert(SymbolEntry {
            name: "int_fn".to_string(),
            kind: SymbolKind::Function,
            visibility: SymbolVisibility::Internal,
            line: 3,
        });
        let public = table.public_entries();
        assert_eq!(public.len(), 1);
        assert_eq!(public[0].name, "pub_fn");
    }

    // 9. SymbolResolver::unresolved_count counts names not found in the table.
    #[test]
    fn test_symbol_resolver_unresolved_count() {
        let mut table = SymbolTable::new();
        table.insert(SymbolEntry {
            name: "known".to_string(),
            kind: SymbolKind::Type,
            visibility: SymbolVisibility::Public,
            line: 5,
        });
        let names = ["known", "missing_a", "missing_b"];
        assert_eq!(SymbolResolver::unresolved_count(&table, &names), 2);
        assert_eq!(SymbolResolver::unresolved_count(&table, &["known"]), 0);
        assert_eq!(SymbolResolver::unresolved_count(&table, &[]), 0);
    }
}
