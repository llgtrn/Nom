use crate::types::*;

/// Mouse button identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Mouse events
#[derive(Debug, Clone)]
pub enum MouseEvent {
    Down { button: MouseButton, position: Vec2, modifiers: Modifiers },
    Up { button: MouseButton, position: Vec2, modifiers: Modifiers },
    Move { position: Vec2, modifiers: Modifiers },
    Enter { position: Vec2 },
    Leave,
}

/// Scroll event
#[derive(Debug, Clone)]
pub struct ScrollEvent {
    pub position: Vec2,
    pub delta: Vec2, // pixels scrolled
    pub modifiers: Modifiers,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on Mac, Win on Windows
}

/// Key identifiers — subset of winit VirtualKeyCode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    // Navigation
    Up, Down, Left, Right, Home, End, PageUp, PageDown,
    // Editing
    Backspace, Delete, Return, Tab, Space, Escape,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Characters (a-z, 0-9)
    Char(char),
    // Unknown
    Unknown,
}

/// Keyboard events
#[derive(Debug, Clone)]
pub enum KeyEvent {
    Pressed { key: Key, modifiers: Modifiers },
    Released { key: Key, modifiers: Modifiers },
    Input { text: String }, // IME composed text
}

impl Modifiers {
    pub fn is_shortcut(&self) -> bool { self.ctrl || self.meta }
}

/// Action dispatch (registered handlers for keyboard shortcuts)
pub trait Action: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
}

/// Common editor actions
#[derive(Debug)]
pub struct Undo;
impl Action for Undo { fn name(&self) -> &'static str { "undo" } }

#[derive(Debug)]
pub struct Redo;
impl Action for Redo { fn name(&self) -> &'static str { "redo" } }

#[derive(Debug)]
pub struct OpenCommandPalette;
impl Action for OpenCommandPalette { fn name(&self) -> &'static str { "open_command_palette" } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_default_all_false() {
        let m = Modifiers::default();
        assert!(!m.shift);
        assert!(!m.ctrl);
        assert!(!m.alt);
        assert!(!m.meta);
    }

    #[test]
    fn is_shortcut_true_for_ctrl() {
        let m = Modifiers { ctrl: true, ..Modifiers::default() };
        assert!(m.is_shortcut());
    }

    #[test]
    fn is_shortcut_true_for_meta() {
        let m = Modifiers { meta: true, ..Modifiers::default() };
        assert!(m.is_shortcut());
    }

    #[test]
    fn is_shortcut_false_when_neither() {
        let m = Modifiers { shift: true, alt: true, ..Modifiers::default() };
        assert!(!m.is_shortcut());
    }

    #[test]
    fn key_event_pressed_matches_key_and_modifiers() {
        let ev = KeyEvent::Pressed {
            key: Key::Char('a'),
            modifiers: Modifiers { ctrl: true, ..Modifiers::default() },
        };
        if let KeyEvent::Pressed { key, modifiers } = ev {
            assert_eq!(key, Key::Char('a'));
            assert!(modifiers.ctrl);
        } else {
            panic!("expected Pressed variant");
        }
    }

    #[test]
    fn key_event_input_carries_text() {
        let ev = KeyEvent::Input { text: "hello".to_string() };
        if let KeyEvent::Input { text } = ev {
            assert_eq!(text, "hello");
        } else {
            panic!("expected Input variant");
        }
    }

    #[test]
    fn mouse_event_down_carries_position_and_button() {
        let pos = Vec2::new(10.0, 20.0);
        let ev = MouseEvent::Down {
            button: MouseButton::Left,
            position: pos,
            modifiers: Modifiers::default(),
        };
        if let MouseEvent::Down { button, position, .. } = ev {
            assert_eq!(button, MouseButton::Left);
            assert_eq!(position, pos);
        } else {
            panic!("expected Down variant");
        }
    }

    #[test]
    fn scroll_event_stores_delta() {
        let ev = ScrollEvent {
            position: Vec2::new(0.0, 0.0),
            delta: Vec2::new(0.0, -120.0),
            modifiers: Modifiers::default(),
        };
        assert_eq!(ev.delta, Vec2::new(0.0, -120.0));
    }

    #[test]
    fn action_names_are_correct() {
        assert_eq!(Undo.name(), "undo");
        assert_eq!(Redo.name(), "redo");
        assert_eq!(OpenCommandPalette.name(), "open_command_palette");
    }
}
