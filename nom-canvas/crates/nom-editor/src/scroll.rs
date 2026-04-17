#![deny(unsafe_code)]
#[derive(Clone, Debug)]
pub struct ScrollPosition { pub top_row: usize, pub vertical_offset: f32, pub horizontal_offset: f32 }
impl Default for ScrollPosition { fn default() -> Self { Self { top_row: 0, vertical_offset: 0.0, horizontal_offset: 0.0 } } }
impl ScrollPosition {
    pub fn scroll_by(&mut self, dy: f32, line_height: f32) {
        self.vertical_offset += dy;
        while self.vertical_offset >= line_height { self.top_row += 1; self.vertical_offset -= line_height; }
        while self.vertical_offset < 0.0 && self.top_row > 0 { self.top_row -= 1; self.vertical_offset += line_height; }
        self.vertical_offset = self.vertical_offset.max(0.0);
    }
    pub fn ensure_visible(&mut self, row: usize, visible_rows: usize) {
        if row < self.top_row { self.top_row = row; }
        else if row >= self.top_row + visible_rows { self.top_row = row + 1 - visible_rows; }
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
        let mut pos = ScrollPosition { top_row: 5, vertical_offset: 0.0, horizontal_offset: 0.0 };
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
        let mut pos = ScrollPosition { top_row: 3, vertical_offset: 0.0, horizontal_offset: 0.0 };
        pos.ensure_visible(5, 10);
        assert_eq!(pos.top_row, 3);
    }

    #[test]
    fn scroll_to_pixel_offset() {
        let pos = ScrollPosition { top_row: 3, vertical_offset: 4.0, horizontal_offset: 0.0 };
        let offset = pos.to_pixel_offset(16.0);
        assert!((offset - 52.0).abs() < f32::EPSILON);
    }
}
