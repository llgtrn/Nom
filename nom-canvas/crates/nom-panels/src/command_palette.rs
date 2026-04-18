#![deny(unsafe_code)]
use crate::dock::{fill_quad, focus_ring_quad};
use nom_gpui::scene::Scene;
use nom_theme::tokens;

#[derive(Debug, Clone)]
pub struct CommandPaletteItem {
    pub label: String,
    pub description: String,
    pub shortcut: Option<String>,
}

impl CommandPaletteItem {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            shortcut: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

pub struct CommandPalette {
    pub items: Vec<CommandPaletteItem>,
    pub query: String,
    pub selected_idx: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            items: vec![],
            query: String::new(),
            selected_idx: 0,
        }
    }

    pub fn set_query(&mut self, q: &str) {
        self.query = q.to_string();
        self.selected_idx = 0;
    }

    pub fn filtered_items(&self) -> Vec<&CommandPaletteItem> {
        if self.query.is_empty() {
            self.items.iter().collect()
        } else {
            let q = self.query.to_lowercase();
            self.items
                .iter()
                .filter(|item| {
                    item.label.to_lowercase().contains(&q)
                        || item.description.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_items().len();
        if count == 0 {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % count;
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered_items().len();
        if count == 0 {
            return;
        }
        self.selected_idx = if self.selected_idx == 0 {
            count - 1
        } else {
            self.selected_idx - 1
        };
    }

    /// Paint the command palette overlay into the shared GPU scene.
    ///
    /// Layout: centered modal with a background quad, a border quad (EDGE_MED),
    /// one row quad per filtered item (BG), and a focus ring on the selected item.
    pub fn paint_scene(&self, width: f32, height: f32, scene: &mut Scene) {
        let modal_w = (width * 0.5).clamp(320.0, 640.0);
        let row_h = 32.0;
        let filtered = self.filtered_items();
        let modal_h = 48.0 + row_h * filtered.len() as f32;
        let modal_x = (width - modal_w) / 2.0;
        let modal_y = (height - modal_h) / 2.0;

        // Background fill
        scene.push_quad(fill_quad(modal_x, modal_y, modal_w, modal_h, tokens::BG));

        // Border quad using EDGE_MED as background (1px border simulation)
        scene.push_quad(fill_quad(modal_x, modal_y, modal_w, 1.0, tokens::EDGE_MED));
        scene.push_quad(fill_quad(
            modal_x,
            modal_y + modal_h - 1.0,
            modal_w,
            1.0,
            tokens::EDGE_MED,
        ));
        scene.push_quad(fill_quad(modal_x, modal_y, 1.0, modal_h, tokens::EDGE_MED));
        scene.push_quad(fill_quad(
            modal_x + modal_w - 1.0,
            modal_y,
            1.0,
            modal_h,
            tokens::EDGE_MED,
        ));

        // Input row
        let input_y = modal_y + 8.0;
        scene.push_quad(fill_quad(
            modal_x + 8.0,
            input_y,
            modal_w - 16.0,
            32.0,
            tokens::BG,
        ));

        // Item rows
        for (i, _item) in filtered.iter().enumerate() {
            let row_y = modal_y + 48.0 + i as f32 * row_h;
            scene.push_quad(fill_quad(modal_x, row_y, modal_w, row_h, tokens::BG));

            if i == self.selected_idx {
                scene.push_quad(focus_ring_quad(
                    modal_x + 2.0,
                    row_y + 2.0,
                    modal_w - 4.0,
                    row_h - 4.0,
                ));
            }
        }
    }
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
    fn command_palette_filter_by_query() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new(
            "Open File",
            "Open a file in the editor",
        ));
        palette
            .items
            .push(CommandPaletteItem::new("Save File", "Save current file"));
        palette
            .items
            .push(CommandPaletteItem::new("Run Tests", "Execute test suite"));

        assert_eq!(palette.filtered_items().len(), 3);

        palette.set_query("file");
        let filtered = palette.filtered_items();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].label, "Open File");
        assert_eq!(filtered[1].label, "Save File");

        palette.set_query("xyz");
        assert_eq!(palette.filtered_items().len(), 0);
    }

    #[test]
    fn command_palette_select_next_wraps() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("A", ""));
        palette.items.push(CommandPaletteItem::new("B", ""));
        palette.items.push(CommandPaletteItem::new("C", ""));

        assert_eq!(palette.selected_idx, 0);
        palette.select_next();
        assert_eq!(palette.selected_idx, 1);
        palette.select_next();
        assert_eq!(palette.selected_idx, 2);
        // Wraps around
        palette.select_next();
        assert_eq!(palette.selected_idx, 0);

        // select_prev wraps backward
        palette.select_prev();
        assert_eq!(palette.selected_idx, 2);
    }

    #[test]
    fn palette_filter_case_insensitive() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new(
            "graph layout",
            "Arrange graph nodes",
        ));
        palette
            .items
            .push(CommandPaletteItem::new("Open File", "Open a file"));
        palette.set_query("GRA");
        let filtered = palette.filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "graph layout");
    }

    #[test]
    fn palette_filtered_empty_when_no_match() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("Open File", ""));
        palette.items.push(CommandPaletteItem::new("Save File", ""));
        palette.set_query("zzz");
        assert_eq!(palette.filtered_items().len(), 0);
    }

    #[test]
    fn palette_select_next_wraps_past_end() {
        let mut palette = CommandPalette::new();
        palette.items.push(CommandPaletteItem::new("A", ""));
        palette.items.push(CommandPaletteItem::new("B", ""));
        // Move to last index (1)
        palette.select_next();
        assert_eq!(palette.selected_idx, 1);
        // Wrap back to first
        palette.select_next();
        assert_eq!(palette.selected_idx, 0);
    }

    #[test]
    fn palette_item_description() {
        let item = CommandPaletteItem::new("Open File", "Opens a file in the editor");
        assert_eq!(item.description, "Opens a file in the editor");
    }

    #[test]
    fn palette_item_shortcut() {
        let item = CommandPaletteItem::new("Save", "Save file").with_shortcut("Ctrl+S");
        assert_eq!(item.shortcut, Some("Ctrl+S".to_string()));
        let item_no_shortcut = CommandPaletteItem::new("Close", "Close");
        assert_eq!(item_no_shortcut.shortcut, None);
    }

    #[test]
    fn palette_execute_first_item() {
        let mut palette = CommandPalette::new();
        palette.items.push(
            CommandPaletteItem::new("Run Tests", "Execute test suite").with_shortcut("Ctrl+T"),
        );
        palette
            .items
            .push(CommandPaletteItem::new("Open File", "Open"));
        // Verify we can access first item by selected_idx
        let filtered = palette.filtered_items();
        assert!(!filtered.is_empty());
        let first = &filtered[palette.selected_idx];
        assert_eq!(first.label, "Run Tests");
    }

    #[test]
    fn command_palette_paint_scene_emits_quads() {
        let mut palette = CommandPalette::new();
        palette
            .items
            .push(CommandPaletteItem::new("Open File", "").with_shortcut("Cmd+O"));
        palette.items.push(CommandPaletteItem::new("Save File", ""));
        palette.items.push(CommandPaletteItem::new("Run Tests", ""));

        let mut scene = Scene::new();
        palette.paint_scene(1440.0, 900.0, &mut scene);

        // background + 4 border edges + input row + 3 item rows + 1 focus ring = 10 quads minimum
        assert!(
            scene.quads.len() >= 10,
            "expected >=10 quads, got {}",
            scene.quads.len()
        );

        // Background quad is the first one pushed
        let bg = &scene.quads[0];
        assert!(bg.background.is_some());
    }
}

// ---------------------------------------------------------------------------
// Typed command palette — category, shortcut, command, registry, filter
// ---------------------------------------------------------------------------

/// High-level categories for palette commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandCategory {
    File,
    Edit,
    View,
    Run,
    Navigate,
}

impl CommandCategory {
    /// Short lowercase prefix string for display in labels.
    pub fn prefix(&self) -> &'static str {
        match self {
            CommandCategory::File => "file",
            CommandCategory::Edit => "edit",
            CommandCategory::View => "view",
            CommandCategory::Run => "run",
            CommandCategory::Navigate => "nav",
        }
    }

    /// Returns `true` for categories whose commands may have side effects.
    pub fn is_destructive(&self) -> bool {
        matches!(self, CommandCategory::Run)
    }
}

/// A keyboard shortcut, optionally with a modifier key.
#[derive(Debug, Clone)]
pub struct CommandShortcut {
    pub key: String,
    pub modifier: Option<String>,
}

impl CommandShortcut {
    /// Format as "Modifier+Key" when a modifier is present, otherwise just "Key".
    pub fn display(&self) -> String {
        match &self.modifier {
            Some(m) => format!("{}+{}", m, self.key),
            None => self.key.clone(),
        }
    }

    /// Returns `true` when a modifier key is set.
    pub fn has_modifier(&self) -> bool {
        self.modifier.is_some()
    }
}

/// A single command registered in the palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    pub id: u32,
    pub label: String,
    pub category: CommandCategory,
    pub shortcut: Option<CommandShortcut>,
}

impl PaletteCommand {
    /// Full display label prefixed with the category string, e.g. "[file] Open".
    pub fn full_label(&self) -> String {
        format!("[{}] {}", self.category.prefix(), self.label)
    }

    /// Returns `true` when a shortcut is attached to this command.
    pub fn has_shortcut(&self) -> bool {
        self.shortcut.is_some()
    }

    /// Returns the shortcut display string, or "—" when no shortcut is set.
    pub fn shortcut_display(&self) -> String {
        match &self.shortcut {
            Some(s) => s.display(),
            None => "\u{2014}".to_string(), // em dash —
        }
    }
}

/// Registry-style command palette that holds and searches typed `PaletteCommand`s.
///
/// Named `CommandRegistry` in this module to avoid collision with the existing
/// overlay-style `CommandPalette` struct above.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    pub commands: Vec<PaletteCommand>,
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    /// Add a command to the registry.
    pub fn register(&mut self, cmd: PaletteCommand) {
        self.commands.push(cmd);
    }

    /// Case-insensitive search across command labels.
    pub fn search(&self, query: &str) -> Vec<&PaletteCommand> {
        let q = query.to_lowercase();
        self.commands
            .iter()
            .filter(|c| c.label.to_lowercase().contains(&q))
            .collect()
    }

    /// Return all commands whose category prefix matches `cat`.
    pub fn by_category(&self, cat: &CommandCategory) -> Vec<&PaletteCommand> {
        let prefix = cat.prefix();
        self.commands
            .iter()
            .filter(|c| c.category.prefix() == prefix)
            .collect()
    }
}

/// A filter that can be toggled active/inactive.
///
/// When active it filters a slice of commands by case-insensitive label match;
/// when inactive it passes every command through unchanged.
#[derive(Debug, Clone)]
pub struct PaletteFilter {
    pub active: bool,
}

impl PaletteFilter {
    /// Create a new, inactive filter.
    pub fn new() -> Self {
        Self { active: false }
    }

    /// Enable filtering.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Disable filtering.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Apply the filter. When inactive every command is returned unchanged.
    pub fn apply<'a>(
        &self,
        commands: Vec<&'a PaletteCommand>,
        query: &str,
    ) -> Vec<&'a PaletteCommand> {
        if self.active {
            let q = query.to_lowercase();
            commands
                .into_iter()
                .filter(|c| c.label.to_lowercase().contains(&q))
                .collect()
        } else {
            commands
        }
    }
}

impl Default for PaletteFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod typed_tests {
    use super::*;

    fn make_cmd(
        id: u32,
        label: &str,
        cat: CommandCategory,
        shortcut: Option<CommandShortcut>,
    ) -> PaletteCommand {
        PaletteCommand {
            id,
            label: label.to_string(),
            category: cat,
            shortcut,
        }
    }

    #[test]
    fn category_prefix_values() {
        assert_eq!(CommandCategory::File.prefix(), "file");
        assert_eq!(CommandCategory::Edit.prefix(), "edit");
        assert_eq!(CommandCategory::View.prefix(), "view");
        assert_eq!(CommandCategory::Run.prefix(), "run");
        assert_eq!(CommandCategory::Navigate.prefix(), "nav");
    }

    #[test]
    fn category_is_destructive_only_run() {
        assert!(CommandCategory::Run.is_destructive());
        assert!(!CommandCategory::File.is_destructive());
        assert!(!CommandCategory::Edit.is_destructive());
        assert!(!CommandCategory::View.is_destructive());
        assert!(!CommandCategory::Navigate.is_destructive());
    }

    #[test]
    fn shortcut_display_with_modifier() {
        let s = CommandShortcut {
            key: "S".to_string(),
            modifier: Some("Ctrl".to_string()),
        };
        assert_eq!(s.display(), "Ctrl+S");
        assert!(s.has_modifier());
    }

    #[test]
    fn shortcut_display_without_modifier() {
        let s = CommandShortcut {
            key: "F5".to_string(),
            modifier: None,
        };
        assert_eq!(s.display(), "F5");
        assert!(!s.has_modifier());
    }

    #[test]
    fn palette_command_full_label() {
        let cmd = make_cmd(1, "Open File", CommandCategory::File, None);
        assert_eq!(cmd.full_label(), "[file] Open File");
    }

    #[test]
    fn palette_command_has_shortcut() {
        let with_sc = make_cmd(
            2,
            "Save",
            CommandCategory::File,
            Some(CommandShortcut {
                key: "S".to_string(),
                modifier: Some("Ctrl".to_string()),
            }),
        );
        assert!(with_sc.has_shortcut());

        let without_sc = make_cmd(3, "Close", CommandCategory::File, None);
        assert!(!without_sc.has_shortcut());
    }

    #[test]
    fn palette_command_shortcut_display_none_is_em_dash() {
        let cmd = make_cmd(4, "Undo", CommandCategory::Edit, None);
        assert_eq!(cmd.shortcut_display(), "\u{2014}");
    }

    #[test]
    fn command_registry_search_case_insensitive() {
        let mut reg = CommandRegistry::new();
        reg.register(make_cmd(1, "Open File", CommandCategory::File, None));
        reg.register(make_cmd(2, "open terminal", CommandCategory::View, None));
        reg.register(make_cmd(3, "Run Tests", CommandCategory::Run, None));

        let results = reg.search("OPEN");
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|c| c.label == "Open File"));
        assert!(results.iter().any(|c| c.label == "open terminal"));
    }

    #[test]
    fn command_registry_by_category() {
        let mut reg = CommandRegistry::new();
        reg.register(make_cmd(1, "Open", CommandCategory::File, None));
        reg.register(make_cmd(2, "Save", CommandCategory::File, None));
        reg.register(make_cmd(3, "Run All", CommandCategory::Run, None));

        let file_cmds = reg.by_category(&CommandCategory::File);
        assert_eq!(file_cmds.len(), 2);

        let run_cmds = reg.by_category(&CommandCategory::Run);
        assert_eq!(run_cmds.len(), 1);
        assert_eq!(run_cmds[0].label, "Run All");
    }

    #[test]
    fn palette_filter_inactive_returns_all() {
        let mut reg = CommandRegistry::new();
        reg.register(make_cmd(1, "Open", CommandCategory::File, None));
        reg.register(make_cmd(2, "Save", CommandCategory::File, None));
        reg.register(make_cmd(3, "Run", CommandCategory::Run, None));

        let filter = PaletteFilter::new();
        assert!(!filter.active);

        let all: Vec<&PaletteCommand> = reg.commands.iter().collect();
        let result = filter.apply(all.clone(), "open");
        // Inactive filter must pass everything through regardless of query
        assert_eq!(result.len(), 3);
    }
}
