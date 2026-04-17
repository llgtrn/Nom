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
}
