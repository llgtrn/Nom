#![deny(unsafe_code)]
#[derive(Clone, Debug)]
pub struct ScrollPosition {
    pub top_row: usize,
    pub anchor_row: usize,
    pub anchor_col: usize,
    pub vertical_offset: f32,
    pub horizontal_offset: f32,
}
impl Default for ScrollPosition {
    fn default() -> Self {
        Self {
            top_row: 0,
            anchor_row: 0,
            anchor_col: 0,
            vertical_offset: 0.0,
            horizontal_offset: 0.0,
        }
    }
}
impl ScrollPosition {
    pub fn with_anchor(row: usize, col: usize) -> Self {
        Self {
            top_row: row,
            anchor_row: row,
            anchor_col: col,
            vertical_offset: 0.0,
            horizontal_offset: 0.0,
        }
    }

    /// Adjust `top_row` so that `line` is visible within `viewport_lines` rows.
    /// The anchor is updated to track the target line.
    pub fn scroll_to_line(&mut self, line: usize, viewport_lines: usize) {
        self.anchor_row = line;
        if line < self.top_row {
            self.top_row = line;
        } else if viewport_lines > 0 && line >= self.top_row + viewport_lines {
            self.top_row = line + 1 - viewport_lines;
        }
    }

    pub fn scroll_by(&mut self, dy: f32, line_height: f32) {
        self.vertical_offset += dy;
        while self.vertical_offset >= line_height {
            self.top_row += 1;
            self.vertical_offset -= line_height;
        }
        while self.vertical_offset < 0.0 && self.top_row > 0 {
            self.top_row -= 1;
            self.vertical_offset += line_height;
        }
        self.vertical_offset = self.vertical_offset.max(0.0);
    }
    pub fn ensure_visible(&mut self, row: usize, visible_rows: usize) {
        if row < self.top_row {
            self.top_row = row;
        } else if row >= self.top_row + visible_rows {
            self.top_row = row + 1 - visible_rows;
        }
    }
    pub fn to_pixel_offset(&self, line_height: f32) -> f32 {
        self.top_row as f32 * line_height + self.vertical_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_by_increases_top_row() {
        let mut pos = ScrollPosition::default();
        pos.scroll_by(20.0, 16.0);
        assert_eq!(pos.top_row, 1);
    }

    #[test]
    fn scroll_by_negative_decreases_top_row() {
        let mut pos = ScrollPosition {
            top_row: 5,
            anchor_row: 5,
            anchor_col: 0,
            vertical_offset: 0.0,
            horizontal_offset: 0.0,
        };
        pos.scroll_by(-16.0, 16.0);
        assert_eq!(pos.top_row, 4);
    }

    #[test]
    fn scroll_ensure_visible_below_viewport() {
        let mut pos = ScrollPosition::default();
        pos.ensure_visible(10, 5);
        assert_eq!(pos.top_row, 6);
    }

    #[test]
    fn scroll_ensure_visible_already_visible() {
        let mut pos = ScrollPosition {
            top_row: 3,
            anchor_row: 3,
            anchor_col: 0,
            vertical_offset: 0.0,
            horizontal_offset: 0.0,
        };
        pos.ensure_visible(5, 10);
        assert_eq!(pos.top_row, 3);
    }

    #[test]
    fn scroll_to_pixel_offset() {
        let pos = ScrollPosition {
            top_row: 3,
            anchor_row: 3,
            anchor_col: 0,
            vertical_offset: 4.0,
            horizontal_offset: 0.0,
        };
        let offset = pos.to_pixel_offset(16.0);
        assert!((offset - 52.0).abs() < f32::EPSILON);
    }

    #[test]
    fn with_anchor_sets_fields() {
        let pos = ScrollPosition::with_anchor(7, 3);
        assert_eq!(pos.top_row, 7);
        assert_eq!(pos.anchor_row, 7);
        assert_eq!(pos.anchor_col, 3);
    }

    #[test]
    fn scroll_to_line_scrolls_down_when_below_viewport() {
        let mut pos = ScrollPosition::default();
        pos.scroll_to_line(12, 5);
        // line 12 should be the last visible line: top_row = 12 + 1 - 5 = 8
        assert_eq!(pos.top_row, 8);
        assert_eq!(pos.anchor_row, 12);
    }

    #[test]
    fn scroll_to_line_scrolls_up_when_above_viewport() {
        let mut pos = ScrollPosition::with_anchor(10, 0);
        pos.scroll_to_line(2, 5);
        assert_eq!(pos.top_row, 2);
        assert_eq!(pos.anchor_row, 2);
    }

    #[test]
    fn scroll_to_line_no_change_when_visible() {
        let mut pos = ScrollPosition::with_anchor(5, 0);
        pos.scroll_to_line(7, 10);
        // line 7 is within [5, 15), so top_row stays 5
        assert_eq!(pos.top_row, 5);
        assert_eq!(pos.anchor_row, 7);
    }

    #[test]
    fn scroll_default_top_row_is_zero() {
        let pos = ScrollPosition::default();
        assert_eq!(pos.top_row, 0);
        assert_eq!(pos.vertical_offset, 0.0);
    }

    #[test]
    fn scroll_by_multiple_lines() {
        let mut pos = ScrollPosition::default();
        pos.scroll_by(48.0, 16.0); // 3 full lines
        assert_eq!(pos.top_row, 3);
    }

    #[test]
    fn scroll_by_fractional_stays_in_offset() {
        let mut pos = ScrollPosition::default();
        pos.scroll_by(10.0, 16.0); // less than one line
        assert_eq!(pos.top_row, 0);
        assert!((pos.vertical_offset - 10.0).abs() < 0.001);
    }

    #[test]
    fn scroll_by_negative_at_top_clamps_to_zero() {
        let mut pos = ScrollPosition::default(); // top_row=0
        pos.scroll_by(-50.0, 16.0);
        assert_eq!(pos.top_row, 0);
        assert!(pos.vertical_offset >= 0.0);
    }

    #[test]
    fn ensure_visible_row_above_scrolls_up() {
        let mut pos = ScrollPosition::with_anchor(10, 0);
        pos.ensure_visible(5, 5);
        assert_eq!(pos.top_row, 5);
    }

    #[test]
    fn scroll_to_line_at_top_row_exactly() {
        let mut pos = ScrollPosition::with_anchor(5, 0);
        pos.scroll_to_line(5, 5);
        // line 5 == top_row, so it's visible; no change
        assert_eq!(pos.top_row, 5);
    }

    #[test]
    fn scroll_to_pixel_offset_zero() {
        let pos = ScrollPosition::default();
        assert_eq!(pos.to_pixel_offset(16.0), 0.0);
    }

    #[test]
    fn scroll_horizontal_offset_defaults_zero() {
        let pos = ScrollPosition::default();
        assert_eq!(pos.horizontal_offset, 0.0);
    }

    #[test]
    fn scroll_with_anchor_col_stored() {
        let pos = ScrollPosition::with_anchor(3, 7);
        assert_eq!(pos.anchor_col, 7);
    }

    // ── scroll to reveal cursor below viewport ────────────────────────────────

    #[test]
    fn scroll_to_line_reveals_cursor_below_viewport() {
        // Viewport shows 5 lines starting at row 0. Cursor moves to row 10.
        let mut pos = ScrollPosition::default();
        pos.scroll_to_line(10, 5);
        // top_row must be set so that row 10 is the last visible line.
        // top_row = 10 + 1 - 5 = 6
        assert_eq!(pos.top_row, 6);
        // Cursor row is visible: top_row <= 10 < top_row + 5
        assert!(pos.top_row <= 10 && 10 < pos.top_row + 5);
    }

    #[test]
    fn scroll_to_line_cursor_just_below_viewport_edge() {
        // Viewport [0, 5). Cursor at line 5 (first invisible line).
        let mut pos = ScrollPosition::default();
        pos.scroll_to_line(5, 5);
        // top_row = 5 + 1 - 5 = 1
        assert_eq!(pos.top_row, 1);
    }

    #[test]
    fn ensure_visible_cursor_below_viewport_scrolls_down() {
        let mut pos = ScrollPosition::default(); // top_row = 0
        pos.ensure_visible(20, 5); // cursor at row 20, viewport 5 rows
        // top_row = 20 + 1 - 5 = 16
        assert_eq!(pos.top_row, 16);
    }

    // ── scroll_by large value clamps ──────────────────────────────────────────

    #[test]
    fn scroll_by_large_value_increases_top_row_proportionally() {
        let mut pos = ScrollPosition::default();
        // Scroll by 1000 pixels with 16-pixel line height → 62 full lines + 8px offset
        pos.scroll_by(1000.0, 16.0);
        assert_eq!(pos.top_row, 62);
        assert!((pos.vertical_offset - 8.0).abs() < 0.01);
    }

    #[test]
    fn scroll_by_large_negative_clamps_top_row_to_zero() {
        let mut pos = ScrollPosition {
            top_row: 3,
            anchor_row: 3,
            anchor_col: 0,
            vertical_offset: 0.0,
            horizontal_offset: 0.0,
        };
        // Scrolling up by a huge amount should not go below row 0.
        pos.scroll_by(-10_000.0, 16.0);
        assert_eq!(pos.top_row, 0);
        assert!(pos.vertical_offset >= 0.0);
    }

    #[test]
    fn scroll_by_exactly_one_line_height() {
        let mut pos = ScrollPosition::default();
        pos.scroll_by(16.0, 16.0);
        assert_eq!(pos.top_row, 1);
        assert!((pos.vertical_offset).abs() < 0.001);
    }

    #[test]
    fn scroll_by_zero_changes_nothing() {
        let mut pos = ScrollPosition {
            top_row: 5,
            anchor_row: 5,
            anchor_col: 0,
            vertical_offset: 4.0,
            horizontal_offset: 0.0,
        };
        pos.scroll_by(0.0, 16.0);
        assert_eq!(pos.top_row, 5);
        assert!((pos.vertical_offset - 4.0).abs() < 0.001);
    }

    #[test]
    fn scroll_to_line_cursor_far_below_large_viewport() {
        // Large viewport (50 lines), cursor at line 200.
        let mut pos = ScrollPosition::default();
        pos.scroll_to_line(200, 50);
        // top_row = 200 + 1 - 50 = 151
        assert_eq!(pos.top_row, 151);
    }

    #[test]
    fn scroll_to_line_cursor_above_top_row_scrolls_up() {
        let mut pos = ScrollPosition::with_anchor(100, 0);
        pos.scroll_to_line(50, 10);
        assert_eq!(pos.top_row, 50);
        assert_eq!(pos.anchor_row, 50);
    }
}
