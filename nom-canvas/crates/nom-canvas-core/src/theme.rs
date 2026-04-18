//! Dark and light theme toggle for NomCanvas.
//!
//! Provides [`ThemeMode`], [`ThemeEntry`], [`ThemeTokenMap`], and [`ThemeToggle`]
//! for managing design tokens across dark and light modes.

/// Whether the canvas is rendered in dark or light mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeMode {
    /// Light mode.
    Light,
    /// Dark mode.
    Dark,
}

impl ThemeMode {
    /// Returns the canonical name of this mode.
    pub fn mode_name(&self) -> &str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    /// Returns the opposite mode.
    pub fn opposite(&self) -> ThemeMode {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }

    /// Returns `true` when this is [`ThemeMode::Dark`].
    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeMode::Dark)
    }
}

/// A single design-token entry with separate light and dark values.
#[derive(Debug, Clone)]
pub struct ThemeEntry {
    /// Token identifier (e.g. `"bg"`, `"text"`).
    pub token_name: String,
    /// CSS / hex value used in light mode.
    pub light_value: String,
    /// CSS / hex value used in dark mode.
    pub dark_value: String,
}

impl ThemeEntry {
    /// Creates a new `ThemeEntry`.
    pub fn new(
        token_name: impl Into<String>,
        light_value: impl Into<String>,
        dark_value: impl Into<String>,
    ) -> Self {
        Self {
            token_name: token_name.into(),
            light_value: light_value.into(),
            dark_value: dark_value.into(),
        }
    }

    /// Resolves the token value for the given mode.
    ///
    /// Returns `light_value` for [`ThemeMode::Light`] and `dark_value` for
    /// [`ThemeMode::Dark`].
    pub fn resolve(&self, mode: &ThemeMode) -> &str {
        match mode {
            ThemeMode::Light => &self.light_value,
            ThemeMode::Dark => &self.dark_value,
        }
    }
}

/// A collection of [`ThemeEntry`] values that can be looked up by token name.
#[derive(Debug, Default)]
pub struct ThemeTokenMap {
    entries: Vec<ThemeEntry>,
}

impl ThemeTokenMap {
    /// Creates an empty `ThemeTokenMap`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an entry.
    pub fn add(&mut self, entry: ThemeEntry) {
        self.entries.push(entry);
    }

    /// Resolves a token by name for the given mode.
    ///
    /// Returns `None` if no entry with the given name exists.
    pub fn resolve(&self, token_name: &str, mode: &ThemeMode) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.token_name == token_name)
            .map(|e| e.resolve(mode))
    }

    /// Returns the number of entries in the map.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Seeds the map with the four standard dark/light tokens:
    ///
    /// | token    | light     | dark      |
    /// |----------|-----------|-----------|
    /// | `bg`     | `#FFFFFF` | `#1A1A1A` |
    /// | `text`   | `#121212` | `#F0F0F0` |
    /// | `border` | `#E0E0E0` | `#333333` |
    /// | `surface`| `#F5F5F5` | `#252525` |
    pub fn seed_dark_light(&mut self) {
        self.add(ThemeEntry::new("bg", "#FFFFFF", "#1A1A1A"));
        self.add(ThemeEntry::new("text", "#121212", "#F0F0F0"));
        self.add(ThemeEntry::new("border", "#E0E0E0", "#333333"));
        self.add(ThemeEntry::new("surface", "#F5F5F5", "#252525"));
    }
}

/// Manages the current [`ThemeMode`] with undo history.
#[derive(Debug)]
pub struct ThemeToggle {
    current: ThemeMode,
    history: Vec<ThemeMode>,
}

impl ThemeToggle {
    /// Creates a new `ThemeToggle` with the given initial mode and empty history.
    pub fn new(initial: ThemeMode) -> Self {
        Self {
            current: initial,
            history: Vec::new(),
        }
    }

    /// Toggles between dark and light, pushing the previous mode onto the history stack.
    pub fn toggle(&mut self) {
        let previous = self.current.clone();
        self.current = previous.opposite();
        self.history.push(previous);
    }

    /// Returns a reference to the current mode.
    pub fn current(&self) -> &ThemeMode {
        &self.current
    }

    /// Returns the number of recorded history entries.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Restores the previous mode by popping the history stack.
    ///
    /// No-op when history is empty.
    pub fn undo(&mut self) {
        if let Some(previous) = self.history.pop() {
            self.current = previous;
        }
    }
}

#[cfg(test)]
mod theme_tests {
    use super::{ThemeEntry, ThemeMode, ThemeToggle, ThemeTokenMap};

    #[test]
    fn theme_mode_is_dark() {
        assert!(ThemeMode::Dark.is_dark());
        assert!(!ThemeMode::Light.is_dark());
    }

    #[test]
    fn theme_mode_opposite() {
        assert_eq!(ThemeMode::Light.opposite(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.opposite(), ThemeMode::Light);
    }

    #[test]
    fn theme_entry_resolve_light() {
        let entry = ThemeEntry::new("bg", "#FFFFFF", "#1A1A1A");
        assert_eq!(entry.resolve(&ThemeMode::Light), "#FFFFFF");
    }

    #[test]
    fn theme_entry_resolve_dark() {
        let entry = ThemeEntry::new("bg", "#FFFFFF", "#1A1A1A");
        assert_eq!(entry.resolve(&ThemeMode::Dark), "#1A1A1A");
    }

    #[test]
    fn theme_token_map_seed_and_count() {
        let mut map = ThemeTokenMap::new();
        map.seed_dark_light();
        assert_eq!(map.entry_count(), 4);
    }

    #[test]
    fn theme_token_map_resolve_bg_dark() {
        let mut map = ThemeTokenMap::new();
        map.seed_dark_light();
        let value = map.resolve("bg", &ThemeMode::Dark);
        assert_eq!(value, Some("#1A1A1A"));
    }

    #[test]
    fn theme_toggle_toggle_changes_mode() {
        let mut toggle = ThemeToggle::new(ThemeMode::Light);
        toggle.toggle();
        assert_eq!(toggle.current(), &ThemeMode::Dark);
    }

    #[test]
    fn theme_toggle_history_tracks() {
        let mut toggle = ThemeToggle::new(ThemeMode::Light);
        toggle.toggle();
        toggle.toggle();
        assert_eq!(toggle.history_len(), 2);
    }

    #[test]
    fn theme_toggle_undo_restores() {
        let mut toggle = ThemeToggle::new(ThemeMode::Light);
        toggle.toggle(); // now Dark, history=[Light]
        toggle.undo();   // back to Light, history=[]
        assert_eq!(toggle.current(), &ThemeMode::Light);
        assert_eq!(toggle.history_len(), 0);
        // undo on empty history is a no-op
        toggle.undo();
        assert_eq!(toggle.current(), &ThemeMode::Light);
    }
}
