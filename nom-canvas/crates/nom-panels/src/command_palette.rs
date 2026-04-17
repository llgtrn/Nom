//! Command palette panel — fuzzy command search overlay.

use smallvec::SmallVec;

/// A single command entry in the palette.
#[derive(Debug, Clone)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub keybind: Option<String>,
    pub category: String,
}

impl Command {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            keybind: None,
            category: category.into(),
        }
    }
}

/// Command palette panel state.
#[derive(Debug)]
pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub commands: SmallVec<[Command; 8]>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            commands: SmallVec::new(),
        }
    }

    /// Open the command palette.
    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
    }

    /// Close the command palette.
    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
    }

    /// Fuzzy search: case-insensitive substring match OR first-letter acronym match.
    pub fn fuzzy_search(&self, query: &str) -> Vec<&Command> {
        if query.is_empty() {
            return self.commands.iter().collect();
        }
        let q = query.to_lowercase();
        self.commands
            .iter()
            .filter(|c| {
                let name_lower = c.name.to_lowercase();
                // Substring match
                if name_lower.contains(&q) {
                    return true;
                }
                // First-letter acronym match: "fo" matches "File Open"
                let initials: String = name_lower
                    .split_whitespace()
                    .filter_map(|w| w.chars().next())
                    .collect();
                initials.contains(&q)
            })
            .collect()
    }

    /// Stub paint method — rendering lives in the GPU layer.
    pub fn paint(&self) {}
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_close_toggle() {
        let mut cp = CommandPalette::new();
        assert!(!cp.is_open);
        cp.open();
        assert!(cp.is_open);
        cp.close();
        assert!(!cp.is_open);
    }

    #[test]
    fn fuzzy_substring_match() {
        let mut cp = CommandPalette::new();
        cp.commands.push(Command::new("fo", "File Open", "file"));
        cp.commands.push(Command::new("fc", "File Close", "file"));
        cp.commands.push(Command::new("gs", "Git Status", "git"));
        let results = cp.fuzzy_search("file");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn fuzzy_acronym_match() {
        let mut cp = CommandPalette::new();
        cp.commands.push(Command::new("fo", "File Open", "file"));
        cp.commands.push(Command::new("gs", "Git Status", "git"));
        // "fo" matches first letters of "File Open"
        let results = cp.fuzzy_search("fo");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.id == "fo"));
    }

    #[test]
    fn empty_query_returns_all() {
        let mut cp = CommandPalette::new();
        cp.commands.push(Command::new("a", "Alpha", "x"));
        cp.commands.push(Command::new("b", "Beta", "x"));
        assert_eq!(cp.fuzzy_search("").len(), 2);
    }
}
