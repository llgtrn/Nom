#![deny(unsafe_code)]
pub mod fonts;
pub mod icons;
pub mod tokens;
pub use fonts::{FontRegistry, TypeStyle};
pub use icons::{icon_path, Icon, IconPath};
pub use tokens::*;

// ---------------------------------------------------------------------------
// Theme mode toggle
// ---------------------------------------------------------------------------

/// The three supported display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
    Oled,
}

impl ThemeMode {
    /// Cycle Dark → Light → Oled → Dark.
    pub fn toggle(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Oled,
            ThemeMode::Oled => ThemeMode::Dark,
        }
    }

    /// Returns `true` for modes that use a dark background (Dark and Oled).
    pub fn is_dark_family(self) -> bool {
        matches!(self, ThemeMode::Dark | ThemeMode::Oled)
    }
}

// ---------------------------------------------------------------------------
// Theme registry
// ---------------------------------------------------------------------------

/// Runtime registry that tracks the active theme mode and its visual tokens.
pub struct ThemeRegistry {
    pub current: ThemeMode,
    pub frosted: FrostedGlassToken,
}

impl ThemeRegistry {
    /// Create a registry initialised for `mode`.
    pub fn new(mode: ThemeMode) -> Self {
        Self {
            frosted: Self::frosted_for_mode(mode),
            current: mode,
        }
    }

    /// Return a new registry with the mode switched to `mode`.
    pub fn switch(self, mode: ThemeMode) -> Self {
        Self::new(mode)
    }

    /// Return the correct [`FrostedGlassToken`] for the given mode.
    pub fn frosted_for_mode(mode: ThemeMode) -> FrostedGlassToken {
        match mode {
            ThemeMode::Dark | ThemeMode::Oled => FrostedGlassToken::default_dark(),
            ThemeMode::Light => FrostedGlassToken::default_light(),
        }
    }
}
