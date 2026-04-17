//! Keyboard shortcut registry: single-key, modified-key, and chord bindings.
#![deny(unsafe_code)]

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    Cmd,
    Ctrl,
    Alt,
    Shift,
}

/// Convenience alias — `Super` on Windows/Linux is `Cmd` on macOS.
impl ModifierKey {
    pub fn is_primary(self) -> bool {
        matches!(self, Self::Cmd | Self::Ctrl)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyStroke {
    pub modifiers: Vec<ModifierKey>,
    pub key: String, // lowercase; e.g. "a", "enter", "f1"
}

impl KeyStroke {
    pub fn new(key: impl Into<String>) -> Self {
        Self { modifiers: Vec::new(), key: key.into().to_lowercase() }
    }

    pub fn with_modifier(mut self, m: ModifierKey) -> Self {
        if !self.modifiers.contains(&m) {
            self.modifiers.push(m);
        }
        self
    }

    /// Canonical textual representation. Modifiers sorted in a stable order.
    pub fn as_canonical(&self) -> String {
        let mut mods = self.modifiers.clone();
        mods.sort_by_key(|m| match m {
            ModifierKey::Ctrl => 0,
            ModifierKey::Cmd => 1,
            ModifierKey::Alt => 2,
            ModifierKey::Shift => 3,
        });
        let mut s = String::new();
        for m in &mods {
            s.push_str(match m {
                ModifierKey::Ctrl => "Ctrl+",
                ModifierKey::Cmd => "Cmd+",
                ModifierKey::Alt => "Alt+",
                ModifierKey::Shift => "Shift+",
            });
        }
        s.push_str(&self.key);
        s
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShortcutBinding {
    pub command_id: String,
    pub chord: Vec<KeyStroke>, // 1 = single keystroke, 2+ = chord
    pub when: Option<String>,  // context filter, e.g. "editor_focused"
}

impl ShortcutBinding {
    pub fn single(command_id: impl Into<String>, stroke: KeyStroke) -> Self {
        Self { command_id: command_id.into(), chord: vec![stroke], when: None }
    }

    pub fn chord(command_id: impl Into<String>, strokes: Vec<KeyStroke>) -> Self {
        Self { command_id: command_id.into(), chord: strokes, when: None }
    }

    pub fn when(mut self, context: impl Into<String>) -> Self {
        self.when = Some(context.into());
        self
    }
}

#[derive(Default)]
pub struct ShortcutRegistry {
    bindings: Vec<ShortcutBinding>,
    by_first_stroke: HashMap<String, Vec<usize>>,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a binding. Returns the index of the binding in the registry.
    /// Conflicts (same first-stroke + same when context) are permitted — the
    /// caller decides precedence (usually last-wins on dispatch).
    pub fn register(&mut self, binding: ShortcutBinding) -> usize {
        let idx = self.bindings.len();
        let first = binding.chord.first().map(|s| s.as_canonical()).unwrap_or_default();
        self.by_first_stroke.entry(first).or_default().push(idx);
        self.bindings.push(binding);
        idx
    }

    /// Find every binding whose first stroke matches `stroke` AND whose
    /// `when` context is either None or equal to `active_context`.
    pub fn find_for(&self, stroke: &KeyStroke, active_context: Option<&str>) -> Vec<&ShortcutBinding> {
        let canonical = stroke.as_canonical();
        let Some(indices) = self.by_first_stroke.get(&canonical) else {
            return Vec::new();
        };
        indices
            .iter()
            .filter_map(|&i| {
                let b = &self.bindings[i];
                match (&b.when, active_context) {
                    (None, _) => Some(b),
                    (Some(w), Some(ctx)) if w == ctx => Some(b),
                    _ => None,
                }
            })
            .collect()
    }

    /// True when two or more bindings share a first-stroke + when-context
    /// combination (potential conflict).
    pub fn conflicts_for(&self, stroke: &KeyStroke, context: Option<&str>) -> Vec<&ShortcutBinding> {
        let matches = self.find_for(stroke, context);
        if matches.len() >= 2 { matches } else { Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn bindings(&self) -> &[ShortcutBinding] {
        &self.bindings
    }
}

/// Normalise a platform-specific binding. On macOS, `Ctrl+` -> `Cmd+` (and
/// vice-versa) so cross-platform keymaps only need to express the intent.
pub fn normalize_for_platform(binding: &ShortcutBinding, is_macos: bool) -> ShortcutBinding {
    let chord = binding
        .chord
        .iter()
        .map(|stroke| {
            let modifiers = stroke
                .modifiers
                .iter()
                .map(|m| match (m, is_macos) {
                    (ModifierKey::Ctrl, true) => ModifierKey::Cmd,
                    (ModifierKey::Cmd, false) => ModifierKey::Ctrl,
                    (other, _) => *other,
                })
                .collect();
            KeyStroke { modifiers, key: stroke.key.clone() }
        })
        .collect();
    ShortcutBinding { command_id: binding.command_id.clone(), chord, when: binding.when.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_new_lowercases() {
        let k = KeyStroke::new("A");
        assert_eq!(k.key, "a");
    }

    #[test]
    fn with_modifier_dedupes() {
        let k = KeyStroke::new("s")
            .with_modifier(ModifierKey::Ctrl)
            .with_modifier(ModifierKey::Ctrl);
        assert_eq!(k.modifiers.len(), 1);
    }

    #[test]
    fn canonical_sort_order_ctrl_before_shift() {
        let k = KeyStroke::new("a")
            .with_modifier(ModifierKey::Shift)
            .with_modifier(ModifierKey::Ctrl);
        assert_eq!(k.as_canonical(), "Ctrl+Shift+a");
    }

    #[test]
    fn single_binding_has_one_chord_element() {
        let b = ShortcutBinding::single("save", KeyStroke::new("s").with_modifier(ModifierKey::Ctrl));
        assert_eq!(b.chord.len(), 1);
    }

    #[test]
    fn chord_binding_accepts_multi_stroke() {
        let strokes = vec![
            KeyStroke::new("k").with_modifier(ModifierKey::Ctrl),
            KeyStroke::new("s").with_modifier(ModifierKey::Ctrl),
        ];
        let b = ShortcutBinding::chord("save.all", strokes);
        assert_eq!(b.chord.len(), 2);
    }

    #[test]
    fn when_attaches_context() {
        let b = ShortcutBinding::single("copy", KeyStroke::new("c").with_modifier(ModifierKey::Ctrl))
            .when("editor_focused");
        assert_eq!(b.when.as_deref(), Some("editor_focused"));
    }

    #[test]
    fn register_len_is_empty() {
        let mut reg = ShortcutRegistry::new();
        assert!(reg.is_empty());
        reg.register(ShortcutBinding::single(
            "save",
            KeyStroke::new("s").with_modifier(ModifierKey::Ctrl),
        ));
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
    }

    #[test]
    fn find_for_matches_no_when() {
        let mut reg = ShortcutRegistry::new();
        let stroke = KeyStroke::new("s").with_modifier(ModifierKey::Ctrl);
        reg.register(ShortcutBinding::single("save", stroke.clone()));
        let found = reg.find_for(&stroke, None);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn find_for_matches_correct_when() {
        let mut reg = ShortcutRegistry::new();
        let stroke = KeyStroke::new("s").with_modifier(ModifierKey::Ctrl);
        reg.register(ShortcutBinding::single("save", stroke.clone()).when("editor_focused"));
        let found = reg.find_for(&stroke, Some("editor_focused"));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn find_for_skips_wrong_when() {
        let mut reg = ShortcutRegistry::new();
        let stroke = KeyStroke::new("s").with_modifier(ModifierKey::Ctrl);
        reg.register(ShortcutBinding::single("save", stroke.clone()).when("editor_focused"));
        let found = reg.find_for(&stroke, Some("tree_focused"));
        assert!(found.is_empty());
    }

    #[test]
    fn conflicts_for_returns_only_if_two_or_more() {
        let mut reg = ShortcutRegistry::new();
        let stroke = KeyStroke::new("s").with_modifier(ModifierKey::Ctrl);
        reg.register(ShortcutBinding::single("save", stroke.clone()));
        assert!(reg.conflicts_for(&stroke, None).is_empty());
        reg.register(ShortcutBinding::single("save.as", stroke.clone()));
        assert_eq!(reg.conflicts_for(&stroke, None).len(), 2);
    }

    #[test]
    fn normalize_macos_ctrl_becomes_cmd() {
        let b = ShortcutBinding::single(
            "save",
            KeyStroke::new("s").with_modifier(ModifierKey::Ctrl),
        );
        let norm = normalize_for_platform(&b, true);
        assert!(norm.chord[0].modifiers.contains(&ModifierKey::Cmd));
        assert!(!norm.chord[0].modifiers.contains(&ModifierKey::Ctrl));
    }

    #[test]
    fn normalize_non_macos_cmd_becomes_ctrl() {
        let b = ShortcutBinding::single(
            "save",
            KeyStroke::new("s").with_modifier(ModifierKey::Cmd),
        );
        let norm = normalize_for_platform(&b, false);
        assert!(norm.chord[0].modifiers.contains(&ModifierKey::Ctrl));
        assert!(!norm.chord[0].modifiers.contains(&ModifierKey::Cmd));
    }

    #[test]
    fn normalize_preserves_alt_and_shift() {
        let b = ShortcutBinding::single(
            "redo",
            KeyStroke::new("z")
                .with_modifier(ModifierKey::Ctrl)
                .with_modifier(ModifierKey::Alt)
                .with_modifier(ModifierKey::Shift),
        );
        let norm = normalize_for_platform(&b, true);
        let mods = &norm.chord[0].modifiers;
        assert!(mods.contains(&ModifierKey::Alt));
        assert!(mods.contains(&ModifierKey::Shift));
    }

    #[test]
    fn empty_chord_binding_uses_empty_string_key() {
        let mut reg = ShortcutRegistry::new();
        let b = ShortcutBinding { command_id: "noop".into(), chord: vec![], when: None };
        reg.register(b);
        assert_eq!(reg.len(), 1);
        assert!(reg.by_first_stroke.contains_key(""));
    }
}
