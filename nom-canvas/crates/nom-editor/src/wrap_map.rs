#![deny(unsafe_code)]

pub struct WrapPoint { pub display_row: usize, pub buffer_row: usize, pub wrap_column: usize }

pub struct WrapMap {
    pub column_width: usize,
    pub wrap_points: Vec<WrapPoint>,
}

impl WrapMap {
    pub fn new(column_width: usize) -> Self {
        Self { column_width, wrap_points: Vec::new() }
    }
    /// Rebuild wrap points for a set of display rows given their visual widths
    pub fn rebuild(&mut self, rows: &[(usize, usize)]) {
        self.wrap_points.clear();
        for (buffer_row, visual_width) in rows {
            if *visual_width > self.column_width {
                let chunks = visual_width / self.column_width;
                for i in 0..chunks {
                    self.wrap_points.push(WrapPoint {
                        display_row: *buffer_row + i,
                        buffer_row: *buffer_row,
                        wrap_column: (i + 1) * self.column_width,
                    });
                }
            }
        }
    }
    pub fn wrap_count(&self) -> usize { self.wrap_points.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_map_no_wraps_for_short_lines() {
        let mut wm = WrapMap::new(80);
        wm.rebuild(&[(0, 40), (1, 30), (2, 20)]);
        assert_eq!(wm.wrap_count(), 0);
    }

    #[test]
    fn wrap_map_wraps_long_line() {
        let mut wm = WrapMap::new(80);
        // Line 0 is 200 chars wide → 200/80 = 2 wrap points
        wm.rebuild(&[(0, 200)]);
        assert_eq!(wm.wrap_count(), 2);
        assert_eq!(wm.wrap_points[0].wrap_column, 80);
        assert_eq!(wm.wrap_points[1].wrap_column, 160);
    }

    #[test]
    fn wrap_map_rebuild_clears_previous_points() {
        let mut wm = WrapMap::new(80);
        wm.rebuild(&[(0, 200)]);
        assert!(wm.wrap_count() > 0);
        wm.rebuild(&[(0, 10)]);
        assert_eq!(wm.wrap_count(), 0);
    }
}
