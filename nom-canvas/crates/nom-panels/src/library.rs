//! Library panel — shared asset/component library.

use smallvec::SmallVec;

/// A single item in the shared library.
#[derive(Debug, Clone)]
pub struct LibraryItem {
    pub id: String,
    pub name: String,
    pub preview_bytes: Option<Vec<u8>>,
}

impl LibraryItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            preview_bytes: None,
        }
    }
}

/// Library panel state.
#[derive(Debug)]
pub struct Library {
    pub items: SmallVec<[LibraryItem; 8]>,
}

impl Library {
    pub fn new() -> Self {
        Self {
            items: SmallVec::new(),
        }
    }

    /// Add an item to the library.
    pub fn add(&mut self, item: LibraryItem) {
        self.items.push(item);
    }

    /// Find items whose name starts with `prefix` (case-insensitive).
    pub fn find(&self, prefix: &str) -> Vec<&LibraryItem> {
        let p = prefix.to_lowercase();
        self.items
            .iter()
            .filter(|i| i.name.to_lowercase().starts_with(&p))
            .collect()
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_increases_len() {
        let mut lib = Library::new();
        assert_eq!(lib.items.len(), 0);
        lib.add(LibraryItem::new("a", "Alpha"));
        assert_eq!(lib.items.len(), 1);
    }

    #[test]
    fn find_prefix_case_insensitive() {
        let mut lib = Library::new();
        lib.add(LibraryItem::new("a", "Alpha"));
        lib.add(LibraryItem::new("b", "Beta"));
        lib.add(LibraryItem::new("c", "Almond"));
        let results = lib.find("AL");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_no_match_returns_empty() {
        let mut lib = Library::new();
        lib.add(LibraryItem::new("a", "Alpha"));
        assert!(lib.find("Z").is_empty());
    }
}
