//! Editor scroll controller.
#![deny(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollPosition {
    /// Top visible line (0-indexed).
    pub top_line: u32,
    /// Pixels scrolled horizontally.
    pub left_px: f32,
    /// Sub-line fractional offset (0.0..1.0) for smooth vertical scrolling.
    pub sub_line: f32,
}

impl ScrollPosition {
    pub fn new() -> Self { Self { top_line: 0, left_px: 0.0, sub_line: 0.0 } }
    pub fn top(self, line: u32) -> Self { Self { top_line: line, ..self } }
    pub fn horizontal(self, px: f32) -> Self { Self { left_px: px.max(0.0), ..self } }
}

impl Default for ScrollPosition { fn default() -> Self { Self::new() } }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width_lines: u32,  // how many lines fit vertically
    pub width_cols: u32,   // how many columns fit horizontally (tab-expanded)
}

/// Scroll the top_line down by `lines` lines, saturating at document length.
pub fn scroll_down(pos: ScrollPosition, lines: u32, total_lines: u32) -> ScrollPosition {
    let new_top = pos.top_line.saturating_add(lines).min(total_lines.saturating_sub(1));
    ScrollPosition { top_line: new_top, ..pos }
}

pub fn scroll_up(pos: ScrollPosition, lines: u32) -> ScrollPosition {
    ScrollPosition { top_line: pos.top_line.saturating_sub(lines), ..pos }
}

/// Scroll a page (i.e. by viewport height).
pub fn scroll_page_down(pos: ScrollPosition, viewport: Viewport, total_lines: u32) -> ScrollPosition {
    scroll_down(pos, viewport.width_lines, total_lines)
}

pub fn scroll_page_up(pos: ScrollPosition, viewport: Viewport) -> ScrollPosition {
    scroll_up(pos, viewport.width_lines)
}

/// Adjust `pos` so that `cursor_line` is visible inside the viewport.
/// Returns the minimally-adjusted position.
pub fn scroll_to_line(pos: ScrollPosition, cursor_line: u32, viewport: Viewport) -> ScrollPosition {
    let top = pos.top_line;
    let bottom = top + viewport.width_lines.saturating_sub(1);
    if cursor_line < top {
        ScrollPosition { top_line: cursor_line, ..pos }
    } else if cursor_line > bottom {
        let new_top = cursor_line.saturating_sub(viewport.width_lines.saturating_sub(1));
        ScrollPosition { top_line: new_top, ..pos }
    } else {
        pos
    }
}

/// Adjust horizontal scroll so that `cursor_col` is visible.
/// `char_width_px` is the average character width; used to compute pixel bounds.
pub fn scroll_to_column(pos: ScrollPosition, cursor_col: u32, viewport: Viewport, char_width_px: f32) -> ScrollPosition {
    let cursor_px = cursor_col as f32 * char_width_px;
    let viewport_px = viewport.width_cols as f32 * char_width_px;
    let left = pos.left_px;
    let right = left + viewport_px;
    if cursor_px < left {
        ScrollPosition { left_px: cursor_px, ..pos }
    } else if cursor_px > right {
        let new_left = (cursor_px - viewport_px).max(0.0);
        ScrollPosition { left_px: new_left, ..pos }
    } else {
        pos
    }
}

/// Is a given line currently visible?
pub fn is_line_visible(pos: ScrollPosition, line: u32, viewport: Viewport) -> bool {
    let top = pos.top_line;
    let bottom = top + viewport.width_lines.saturating_sub(1);
    line >= top && line <= bottom
}

/// Clamp position against total document dimensions.
pub fn clamp_to_document(pos: ScrollPosition, total_lines: u32) -> ScrollPosition {
    let max_top = total_lines.saturating_sub(1);
    ScrollPosition {
        top_line: pos.top_line.min(max_top),
        left_px: pos.left_px.max(0.0),
        sub_line: pos.sub_line.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp(lines: u32, cols: u32) -> Viewport { Viewport { width_lines: lines, width_cols: cols } }

    #[test]
    fn scroll_position_new_defaults() {
        let p = ScrollPosition::new();
        assert_eq!(p.top_line, 0);
        assert_eq!(p.left_px, 0.0);
        assert_eq!(p.sub_line, 0.0);
    }

    #[test]
    fn builder_top_and_horizontal_chain() {
        let p = ScrollPosition::new().top(5).horizontal(120.0);
        assert_eq!(p.top_line, 5);
        assert_eq!(p.left_px, 120.0);
    }

    #[test]
    fn horizontal_clamps_negative_to_zero() {
        let p = ScrollPosition::new().horizontal(-50.0);
        assert_eq!(p.left_px, 0.0);
    }

    #[test]
    fn scroll_down_saturates_at_total_minus_one() {
        let pos = ScrollPosition::new().top(90);
        let result = scroll_down(pos, 20, 100);
        assert_eq!(result.top_line, 99);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let pos = ScrollPosition::new().top(3);
        let result = scroll_up(pos, 10);
        assert_eq!(result.top_line, 0);
    }

    #[test]
    fn scroll_page_down_uses_viewport_width_lines() {
        let pos = ScrollPosition::new().top(0);
        let result = scroll_page_down(pos, vp(10, 80), 100);
        assert_eq!(result.top_line, 10);
    }

    #[test]
    fn scroll_page_up_uses_viewport_width_lines() {
        let pos = ScrollPosition::new().top(20);
        let result = scroll_page_up(pos, vp(10, 80));
        assert_eq!(result.top_line, 10);
    }

    #[test]
    fn scroll_to_line_cursor_above_scrolls_up_to_cursor() {
        let pos = ScrollPosition::new().top(10);
        let result = scroll_to_line(pos, 5, vp(10, 80));
        assert_eq!(result.top_line, 5);
    }

    #[test]
    fn scroll_to_line_cursor_below_scrolls_to_make_visible() {
        let pos = ScrollPosition::new().top(0);
        let result = scroll_to_line(pos, 15, vp(10, 80));
        // cursor_line=15, viewport=10: new_top = 15 - (10-1) = 6
        assert_eq!(result.top_line, 6);
    }

    #[test]
    fn scroll_to_line_cursor_inside_unchanged() {
        let pos = ScrollPosition::new().top(5);
        let result = scroll_to_line(pos, 8, vp(10, 80));
        assert_eq!(result.top_line, 5);
    }

    #[test]
    fn scroll_to_column_cursor_left_scrolls_left() {
        let pos = ScrollPosition::new().horizontal(100.0);
        // cursor at col 3, char_width=10 => cursor_px=30, left=100 => 30 < 100
        let result = scroll_to_column(pos, 3, vp(10, 80), 10.0);
        assert_eq!(result.left_px, 30.0);
    }

    #[test]
    fn scroll_to_column_cursor_right_scrolls_right() {
        let pos = ScrollPosition::new().horizontal(0.0);
        // cursor at col 100, char_width=10 => cursor_px=1000, viewport_px=80*10=800
        // right = 0+800=800 => 1000 > 800 => new_left = 1000-800 = 200
        let result = scroll_to_column(pos, 100, vp(10, 80), 10.0);
        assert_eq!(result.left_px, 200.0);
    }

    #[test]
    fn is_line_visible_true_and_false_boundaries() {
        let pos = ScrollPosition::new().top(10);
        let viewport = vp(10, 80);
        // top=10, bottom=19
        assert!(is_line_visible(pos, 10, viewport));
        assert!(is_line_visible(pos, 19, viewport));
        assert!(!is_line_visible(pos, 9, viewport));
        assert!(!is_line_visible(pos, 20, viewport));
    }

    #[test]
    fn clamp_to_document_clamps_top_line() {
        let pos = ScrollPosition::new().top(200);
        let result = clamp_to_document(pos, 50);
        assert_eq!(result.top_line, 49);
    }

    #[test]
    fn clamp_to_document_clamps_sub_line() {
        let pos = ScrollPosition { top_line: 0, left_px: 0.0, sub_line: 1.5 };
        let result = clamp_to_document(pos, 100);
        assert_eq!(result.sub_line, 1.0);

        let pos2 = ScrollPosition { top_line: 0, left_px: 0.0, sub_line: -0.5 };
        let result2 = clamp_to_document(pos2, 100);
        assert_eq!(result2.sub_line, 0.0);
    }
}
