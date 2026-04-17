//! Keyboard and IME input dispatch.
//!
//! `translate` converts a `KeyEvent` into an `EditorCommand`. IME composition
//! state (pre-edit strings) is tracked as a TODO for a later winit integration
//! pass; the types are already stubbed here so callers can handle that path.

#![deny(unsafe_code)]

/// Logical key identity, independent of physical layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Return,
    Tab,
    Escape,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
}

/// Active modifier keys at the moment of the event.
///
/// Design note: each modifier is an independent `bool` field rather than a
/// bitfield so that pattern matching is exhaustive and field names remain
/// readable. The overhead is negligible at four bytes per event.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// Cmd on macOS, Win key on Windows.
    pub meta: bool,
}

/// A complete keyboard event: the logical key plus any held modifiers.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub mods: Modifiers,
}

/// High-level editor operations derived from raw key events.
///
/// Variants are intentionally granular — each maps to exactly one logical
/// editing intent — so that the buffer layer does not need to re-interpret
/// modifier state.
#[derive(Clone, Debug)]
pub enum EditorCommand {
    InsertChar(char),
    InsertString(String),
    Backspace,
    Delete,
    NewLine,
    Tab,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveLineStart,
    MoveLineEnd,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectLineStart,
    SelectLineEnd,
    /// Key combination that has no bound action in the current context.
    Noop,
}

/// Translate a raw `KeyEvent` into an `EditorCommand`.
///
/// Modifier-sensitive bindings (shift + arrow = select) are resolved here so
/// callers receive a fully-resolved intent without inspecting `Modifiers`.
pub fn translate(ev: &KeyEvent) -> EditorCommand {
    match ev.key {
        Key::Char(c) => EditorCommand::InsertChar(c),
        Key::Backspace => EditorCommand::Backspace,
        Key::Delete    => EditorCommand::Delete,
        Key::Return    => EditorCommand::NewLine,
        Key::Tab       => EditorCommand::Tab,

        Key::ArrowLeft  if ev.mods.shift => EditorCommand::SelectLeft,
        Key::ArrowLeft                   => EditorCommand::MoveLeft,
        Key::ArrowRight if ev.mods.shift => EditorCommand::SelectRight,
        Key::ArrowRight                  => EditorCommand::MoveRight,
        Key::ArrowUp    if ev.mods.shift => EditorCommand::SelectUp,
        Key::ArrowUp                     => EditorCommand::MoveUp,
        Key::ArrowDown  if ev.mods.shift => EditorCommand::SelectDown,
        Key::ArrowDown                   => EditorCommand::MoveDown,

        Key::Home if ev.mods.shift => EditorCommand::SelectLineStart,
        Key::Home                  => EditorCommand::MoveLineStart,
        Key::End  if ev.mods.shift => EditorCommand::SelectLineEnd,
        Key::End                   => EditorCommand::MoveLineEnd,

        _ => EditorCommand::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(key: Key) -> KeyEvent {
        KeyEvent { key, mods: Modifiers::default() }
    }

    fn ev_shift(key: Key) -> KeyEvent {
        KeyEvent { key, mods: Modifiers { shift: true, ..Modifiers::default() } }
    }

    #[test]
    fn char_becomes_insert_char() {
        assert!(matches!(translate(&ev(Key::Char('a'))), EditorCommand::InsertChar('a')));
    }

    #[test]
    fn backspace_and_delete() {
        assert!(matches!(translate(&ev(Key::Backspace)), EditorCommand::Backspace));
        assert!(matches!(translate(&ev(Key::Delete)),    EditorCommand::Delete));
    }

    #[test]
    fn return_becomes_new_line() {
        assert!(matches!(translate(&ev(Key::Return)), EditorCommand::NewLine));
    }

    #[test]
    fn tab_becomes_tab() {
        assert!(matches!(translate(&ev(Key::Tab)), EditorCommand::Tab));
    }

    #[test]
    fn shift_arrow_left_is_select_left() {
        assert!(matches!(translate(&ev_shift(Key::ArrowLeft)), EditorCommand::SelectLeft));
    }

    #[test]
    fn unshift_arrow_left_is_move_left() {
        assert!(matches!(translate(&ev(Key::ArrowLeft)), EditorCommand::MoveLeft));
    }

    #[test]
    fn shift_arrow_right_is_select_right() {
        assert!(matches!(translate(&ev_shift(Key::ArrowRight)), EditorCommand::SelectRight));
    }

    #[test]
    fn shift_home_is_select_line_start() {
        assert!(matches!(translate(&ev_shift(Key::Home)), EditorCommand::SelectLineStart));
    }

    #[test]
    fn home_is_move_line_start() {
        assert!(matches!(translate(&ev(Key::Home)), EditorCommand::MoveLineStart));
    }

    #[test]
    fn shift_end_is_select_line_end() {
        assert!(matches!(translate(&ev_shift(Key::End)), EditorCommand::SelectLineEnd));
    }

    #[test]
    fn end_is_move_line_end() {
        assert!(matches!(translate(&ev(Key::End)), EditorCommand::MoveLineEnd));
    }

    #[test]
    fn escape_is_noop() {
        assert!(matches!(translate(&ev(Key::Escape)), EditorCommand::Noop));
    }

    #[test]
    fn page_up_down_are_noop() {
        assert!(matches!(translate(&ev(Key::PageUp)),   EditorCommand::Noop));
        assert!(matches!(translate(&ev(Key::PageDown)), EditorCommand::Noop));
    }

    #[test]
    fn modifiers_default_is_all_false() {
        let m = Modifiers::default();
        assert!(!m.shift && !m.ctrl && !m.alt && !m.meta);
    }
}
