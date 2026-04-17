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
    Down {
        button: MouseButton,
        position: Vec2,
        modifiers: Modifiers,
    },
    Up {
        button: MouseButton,
        position: Vec2,
        modifiers: Modifiers,
    },
    Move {
        position: Vec2,
        modifiers: Modifiers,
    },
    Enter {
        position: Vec2,
    },
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
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    // Editing
    Backspace,
    Delete,
    Return,
    Tab,
    Space,
    Escape,
    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
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
    pub fn is_shortcut(&self) -> bool {
        self.ctrl || self.meta
    }
}

/// Action dispatch (registered handlers for keyboard shortcuts)
pub trait Action: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
}

/// Common editor actions
#[derive(Debug)]
pub struct Undo;
impl Action for Undo {
    fn name(&self) -> &'static str {
        "undo"
    }
}

#[derive(Debug)]
pub struct Redo;
impl Action for Redo {
    fn name(&self) -> &'static str {
        "redo"
    }
}

#[derive(Debug)]
pub struct OpenCommandPalette;
impl Action for OpenCommandPalette {
    fn name(&self) -> &'static str {
        "open_command_palette"
    }
}

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
        let m = Modifiers {
            ctrl: true,
            ..Modifiers::default()
        };
        assert!(m.is_shortcut());
    }

    #[test]
    fn is_shortcut_true_for_meta() {
        let m = Modifiers {
            meta: true,
            ..Modifiers::default()
        };
        assert!(m.is_shortcut());
    }

    #[test]
    fn is_shortcut_false_when_neither() {
        let m = Modifiers {
            shift: true,
            alt: true,
            ..Modifiers::default()
        };
        assert!(!m.is_shortcut());
    }

    #[test]
    fn key_event_pressed_matches_key_and_modifiers() {
        let ev = KeyEvent::Pressed {
            key: Key::Char('a'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
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
        let ev = KeyEvent::Input {
            text: "hello".to_string(),
        };
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
        if let MouseEvent::Down {
            button, position, ..
        } = ev
        {
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

    #[test]
    fn mouse_event_position_preserved() {
        let pos = Vec2::new(42.5, 99.0);
        let ev = MouseEvent::Move {
            position: pos,
            modifiers: Modifiers::default(),
        };
        if let MouseEvent::Move { position, .. } = ev {
            assert_eq!(position, pos);
        } else {
            panic!("expected Move variant");
        }
    }

    #[test]
    fn key_event_code_preserved() {
        let ev = KeyEvent::Pressed {
            key: Key::Return,
            modifiers: Modifiers::default(),
        };
        if let KeyEvent::Pressed { key, .. } = ev {
            assert_eq!(key, Key::Return);
        } else {
            panic!("expected Pressed variant");
        }
    }

    #[test]
    fn scroll_event_delta_preserved() {
        let delta = Vec2::new(-15.0, 30.0);
        let ev = ScrollEvent {
            position: Vec2::new(0.0, 0.0),
            delta,
            modifiers: Modifiers::default(),
        };
        assert_eq!(ev.delta, delta);
    }

    #[test]
    fn mouse_button_other_variant_carries_value() {
        let btn = MouseButton::Other(5);
        assert_eq!(btn, MouseButton::Other(5));
        assert_ne!(btn, MouseButton::Other(6));
    }

    #[test]
    fn mouse_event_up_position_preserved() {
        let pos = Vec2::new(7.0, 14.0);
        let ev = MouseEvent::Up {
            button: MouseButton::Right,
            position: pos,
            modifiers: Modifiers::default(),
        };
        if let MouseEvent::Up {
            button, position, ..
        } = ev
        {
            assert_eq!(button, MouseButton::Right);
            assert_eq!(position, pos);
        } else {
            panic!("expected Up variant");
        }
    }

    #[test]
    fn mouse_event_enter_position_preserved() {
        let pos = Vec2::new(3.0, 5.0);
        let ev = MouseEvent::Enter { position: pos };
        if let MouseEvent::Enter { position } = ev {
            assert_eq!(position, pos);
        } else {
            panic!("expected Enter variant");
        }
    }

    #[test]
    fn key_event_released_key_preserved() {
        let ev = KeyEvent::Released {
            key: Key::Escape,
            modifiers: Modifiers::default(),
        };
        if let KeyEvent::Released { key, .. } = ev {
            assert_eq!(key, Key::Escape);
        } else {
            panic!("expected Released variant");
        }
    }

    #[test]
    fn key_char_variant_preserves_char() {
        let k = Key::Char('z');
        assert_eq!(k, Key::Char('z'));
        assert_ne!(k, Key::Char('a'));
    }

    #[test]
    fn modifiers_shortcut_false_for_shift_only() {
        let m = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        assert!(!m.is_shortcut());
    }

    #[test]
    fn scroll_event_position_preserved() {
        let pos = Vec2::new(100.0, 200.0);
        let ev = ScrollEvent {
            position: pos,
            delta: Vec2::new(0.0, 0.0),
            modifiers: Modifiers::default(),
        };
        assert_eq!(ev.position, pos);
    }
}
