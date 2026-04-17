#![deny(unsafe_code)]
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char), Enter, Escape, Backspace, Delete,
    ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    Home, End, PageUp, PageDown, Tab,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl KeyBinding {
    pub fn key(key: KeyCode) -> Self { Self { key, ctrl: false, alt: false, shift: false, meta: false } }
    pub fn ctrl(key: KeyCode) -> Self { Self { key, ctrl: true, alt: false, shift: false, meta: false } }
    pub fn ctrl_shift(key: KeyCode) -> Self { Self { key, ctrl: true, alt: false, shift: true, meta: false } }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyAction {
    InsertChar(char),
    DeleteBackward, DeleteForward,
    MoveLeft, MoveRight, MoveUp, MoveDown,
    MoveWordLeft, MoveWordRight,
    MoveToLineStart, MoveToLineEnd,
    SelectLeft, SelectRight, SelectAll,
    SelectWordLeft, SelectWordRight,
    Undo, Redo,
    DuplicateCursor,
    ToggleComment,
    Newline,
    Indent, Dedent,
    Copy, Cut, Paste,
    TriggerCompletion,
    Escape,
}

pub struct ActionRegistry {
    bindings: HashMap<KeyBinding, KeyAction>,
}

impl ActionRegistry {
    pub fn default_bindings() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('z')), KeyAction::Undo);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('y')), KeyAction::Redo);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('d')), KeyAction::DuplicateCursor);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('/')), KeyAction::ToggleComment);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('c')), KeyAction::Copy);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('x')), KeyAction::Cut);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('v')), KeyAction::Paste);
        bindings.insert(KeyBinding::ctrl(KeyCode::Char('a')), KeyAction::SelectAll);
        bindings.insert(KeyBinding::key(KeyCode::Backspace), KeyAction::DeleteBackward);
        bindings.insert(KeyBinding::key(KeyCode::Delete), KeyAction::DeleteForward);
        bindings.insert(KeyBinding::key(KeyCode::ArrowLeft), KeyAction::MoveLeft);
        bindings.insert(KeyBinding::key(KeyCode::ArrowRight), KeyAction::MoveRight);
        bindings.insert(KeyBinding::key(KeyCode::ArrowUp), KeyAction::MoveUp);
        bindings.insert(KeyBinding::key(KeyCode::ArrowDown), KeyAction::MoveDown);
        bindings.insert(KeyBinding::key(KeyCode::Home), KeyAction::MoveToLineStart);
        bindings.insert(KeyBinding::key(KeyCode::End), KeyAction::MoveToLineEnd);
        bindings.insert(KeyBinding::key(KeyCode::Enter), KeyAction::Newline);
        bindings.insert(KeyBinding::key(KeyCode::Tab), KeyAction::Indent);
        bindings.insert(KeyBinding::key(KeyCode::Escape), KeyAction::Escape);
        bindings.insert(KeyBinding { key: KeyCode::Char(' '), ctrl: true, alt: false, shift: false, meta: false }, KeyAction::TriggerCompletion);
        Self { bindings }
    }

    pub fn resolve(&self, binding: &KeyBinding) -> Option<&KeyAction> {
        self.bindings.get(binding)
    }
}

/// IME composition state
#[derive(Clone, Debug, Default)]
pub struct ImeState {
    pub composing: bool,
    pub composition_text: String,
    pub cursor_in_composition: usize,
}

impl ImeState {
    pub fn start(&mut self) { self.composing = true; self.composition_text.clear(); }
    pub fn update(&mut self, text: impl Into<String>) { self.composition_text = text.into(); }
    pub fn commit(&mut self) -> String {
        self.composing = false;
        std::mem::take(&mut self.composition_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_registry_undo_redo() {
        let reg = ActionRegistry::default_bindings();
        assert_eq!(reg.resolve(&KeyBinding::ctrl(KeyCode::Char('z'))), Some(&KeyAction::Undo));
        assert_eq!(reg.resolve(&KeyBinding::ctrl(KeyCode::Char('y'))), Some(&KeyAction::Redo));
    }

    #[test]
    fn ime_state_lifecycle() {
        let mut ime = ImeState::default();
        ime.start();
        assert!(ime.composing);
        ime.update("にほ");
        let text = ime.commit();
        assert_eq!(text, "にほ");
        assert!(!ime.composing);
    }
}
