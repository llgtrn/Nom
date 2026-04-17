//! Icon metadata registry — viewbox + style policy only.
//!
//! Actual SVG→path tessellation or raster atlas upload happens in nom-gpui.
//! This module holds the canonical icon list + metadata so that UI code can
//! reference icons by stable ids (e.g. "chevron-down") without embedding SVG.
#![deny(unsafe_code)]

pub type IconId = &'static str;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconStyle {
    Stroke,
    Fill,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ViewBox {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        ViewBox { x, y, width, height }
    }
    pub const LUCIDE_24: ViewBox = ViewBox::new(0, 0, 24, 24);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconMeta {
    pub id: IconId,
    pub viewbox: ViewBox,
    pub style: IconStyle,
    /// Default stroke width for Stroke/Both; ignored for Fill.
    pub stroke_width_px: f32,
    pub category: IconCategory,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconCategory {
    Arrow,
    Navigation,
    Action,
    File,
    Text,
    Layout,
    Media,
    Status,
    Communication,
    Misc,
}

impl IconMeta {
    pub const fn stroke_24(id: IconId, category: IconCategory) -> Self {
        IconMeta {
            id,
            viewbox: ViewBox::LUCIDE_24,
            style: IconStyle::Stroke,
            stroke_width_px: 2.0,
            category,
        }
    }

    pub const fn fill_24(id: IconId, category: IconCategory) -> Self {
        IconMeta {
            id,
            viewbox: ViewBox::LUCIDE_24,
            style: IconStyle::Fill,
            stroke_width_px: 0.0,
            category,
        }
    }
}

/// Canonical core icon list — ~40 ids covering the UI essentials from the
/// Lucide 24-pixel stroke family. Full 1400-icon catalog is lazy-loaded
/// from disk (future work).
pub const CORE_ICONS: &[IconMeta] = &[
    // Arrow
    IconMeta::stroke_24("chevron-up", IconCategory::Arrow),
    IconMeta::stroke_24("chevron-down", IconCategory::Arrow),
    IconMeta::stroke_24("chevron-left", IconCategory::Arrow),
    IconMeta::stroke_24("chevron-right", IconCategory::Arrow),
    IconMeta::stroke_24("arrow-up", IconCategory::Arrow),
    IconMeta::stroke_24("arrow-down", IconCategory::Arrow),
    IconMeta::stroke_24("arrow-left", IconCategory::Arrow),
    IconMeta::stroke_24("arrow-right", IconCategory::Arrow),
    // Navigation
    IconMeta::stroke_24("menu", IconCategory::Navigation),
    IconMeta::stroke_24("home", IconCategory::Navigation),
    IconMeta::stroke_24("search", IconCategory::Navigation),
    IconMeta::stroke_24("settings", IconCategory::Navigation),
    // Action
    IconMeta::stroke_24("plus", IconCategory::Action),
    IconMeta::stroke_24("minus", IconCategory::Action),
    IconMeta::stroke_24("x", IconCategory::Action),
    IconMeta::stroke_24("check", IconCategory::Action),
    IconMeta::stroke_24("edit", IconCategory::Action),
    IconMeta::stroke_24("trash", IconCategory::Action),
    IconMeta::stroke_24("copy", IconCategory::Action),
    IconMeta::stroke_24("save", IconCategory::Action),
    // File
    IconMeta::stroke_24("file", IconCategory::File),
    IconMeta::stroke_24("folder", IconCategory::File),
    IconMeta::stroke_24("file-plus", IconCategory::File),
    IconMeta::stroke_24("download", IconCategory::File),
    IconMeta::stroke_24("upload", IconCategory::File),
    // Text
    IconMeta::stroke_24("bold", IconCategory::Text),
    IconMeta::stroke_24("italic", IconCategory::Text),
    IconMeta::stroke_24("underline", IconCategory::Text),
    IconMeta::stroke_24("list", IconCategory::Text),
    IconMeta::stroke_24("heading-1", IconCategory::Text),
    IconMeta::stroke_24("heading-2", IconCategory::Text),
    // Layout
    IconMeta::stroke_24("sidebar", IconCategory::Layout),
    IconMeta::stroke_24("columns", IconCategory::Layout),
    IconMeta::stroke_24("grid", IconCategory::Layout),
    // Media
    IconMeta::stroke_24("image", IconCategory::Media),
    IconMeta::stroke_24("video", IconCategory::Media),
    IconMeta::stroke_24("music", IconCategory::Media),
    // Status
    IconMeta::stroke_24("alert-circle", IconCategory::Status),
    IconMeta::stroke_24("info", IconCategory::Status),
    IconMeta::stroke_24("check-circle", IconCategory::Status),
    // Communication
    IconMeta::stroke_24("message", IconCategory::Communication),
    IconMeta::stroke_24("bell", IconCategory::Communication),
];

pub struct IconRegistry {
    items: Vec<IconMeta>,
}

impl IconRegistry {
    pub fn new() -> Self {
        Self { items: CORE_ICONS.to_vec() }
    }

    pub fn get(&self, id: IconId) -> Option<&IconMeta> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn by_category(&self, category: IconCategory) -> Vec<&IconMeta> {
        self.items.iter().filter(|i| i.category == category).collect()
    }

    /// Inserts or replaces an icon by id.
    pub fn register(&mut self, meta: IconMeta) {
        self.items.retain(|i| i.id != meta.id);
        self.items.push(meta);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for IconRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewbox_lucide_24_values() {
        assert_eq!(ViewBox::LUCIDE_24.x, 0);
        assert_eq!(ViewBox::LUCIDE_24.y, 0);
        assert_eq!(ViewBox::LUCIDE_24.width, 24);
        assert_eq!(ViewBox::LUCIDE_24.height, 24);
    }

    #[test]
    fn stroke_24_has_correct_stroke_width() {
        let m = IconMeta::stroke_24("test", IconCategory::Misc);
        assert_eq!(m.stroke_width_px, 2.0);
        assert_eq!(m.style, IconStyle::Stroke);
    }

    #[test]
    fn fill_24_has_fill_style_and_zero_stroke() {
        let m = IconMeta::fill_24("test-fill", IconCategory::Misc);
        assert_eq!(m.style, IconStyle::Fill);
        assert_eq!(m.stroke_width_px, 0.0);
    }

    #[test]
    fn core_icons_has_all_8_arrow_ids() {
        let arrow_ids = ["chevron-up", "chevron-down", "chevron-left", "chevron-right",
                         "arrow-up", "arrow-down", "arrow-left", "arrow-right"];
        for id in &arrow_ids {
            assert!(
                CORE_ICONS.iter().any(|i| i.id == *id && i.category == IconCategory::Arrow),
                "missing arrow icon: {id}"
            );
        }
    }

    #[test]
    fn registry_new_has_at_least_40_icons() {
        let reg = IconRegistry::new();
        assert!(reg.len() >= 40, "expected >= 40 icons, got {}", reg.len());
    }

    #[test]
    fn get_returns_some_for_chevron_down() {
        let reg = IconRegistry::new();
        assert!(reg.get("chevron-down").is_some());
    }

    #[test]
    fn get_returns_none_for_nonexistent() {
        let reg = IconRegistry::new();
        assert!(reg.get("nonexistent-icon-xyz").is_none());
    }

    #[test]
    fn by_category_arrow_returns_8_icons() {
        let reg = IconRegistry::new();
        assert_eq!(reg.by_category(IconCategory::Arrow).len(), 8);
    }

    #[test]
    fn by_category_status_returns_3_icons() {
        let reg = IconRegistry::new();
        assert_eq!(reg.by_category(IconCategory::Status).len(), 3);
    }

    #[test]
    fn register_replaces_existing_id() {
        let mut reg = IconRegistry::new();
        let before_len = reg.len();
        let replacement = IconMeta::fill_24("chevron-down", IconCategory::Arrow);
        reg.register(replacement);
        // Length unchanged (replaced, not added)
        assert_eq!(reg.len(), before_len);
        let got = reg.get("chevron-down").unwrap();
        assert_eq!(got.style, IconStyle::Fill);
    }

    #[test]
    fn register_new_id_increases_len() {
        let mut reg = IconRegistry::new();
        let before = reg.len();
        reg.register(IconMeta::stroke_24("brand-new-icon", IconCategory::Misc));
        assert_eq!(reg.len(), before + 1);
    }

    #[test]
    fn is_empty_false_for_new_registry() {
        assert!(!IconRegistry::new().is_empty());
    }
}
