//! CSS-grid-inspired layout primitives: tracks, cells, and placement.

/// Size specification for a single grid track (row or column).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackSize {
    /// Fixed pixel size.
    Fixed(f32),
    /// Fractional share of available space.
    Fraction(f32),
    /// Auto-sized: fills all available space.
    Auto,
}

impl TrackSize {
    /// Returns `true` for Fraction and Auto tracks (size depends on context).
    pub fn is_flexible(&self) -> bool {
        matches!(self, TrackSize::Fraction(_) | TrackSize::Auto)
    }

    /// Minimum intrinsic size: Fixed tracks contribute their value; flexible tracks contribute 0.
    pub fn min_size(&self) -> f32 {
        match self {
            TrackSize::Fixed(v) => *v,
            TrackSize::Fraction(_) | TrackSize::Auto => 0.0,
        }
    }
}

/// A single row or column track in the grid.
#[derive(Debug, Clone)]
pub struct GridTrack {
    /// Zero-based index of this track.
    pub index: u32,
    /// How the track is sized.
    pub size: TrackSize,
    /// Gap (gutter) appended after this track.
    pub gap: f32,
}

impl GridTrack {
    /// Resolved size of this track given `available` space.
    ///
    /// - Fixed → the fixed value (ignores `available`)
    /// - Fraction(f) → `available * f`
    /// - Auto → `available`
    pub fn total_size(&self, available: f32) -> f32 {
        match &self.size {
            TrackSize::Fixed(v) => *v,
            TrackSize::Fraction(f) => available * f,
            TrackSize::Auto => available,
        }
    }

    /// Convenience: `total_size(100.0) + gap`.
    pub fn with_gap(&self) -> f32 {
        self.total_size(100.0) + self.gap
    }
}

/// Rectangular region within the grid, expressed in track indices.
#[derive(Debug, Clone)]
pub struct GridCell {
    /// Starting row index (0-based).
    pub row: u32,
    /// Starting column index (0-based).
    pub col: u32,
    /// Number of rows spanned.
    pub row_span: u32,
    /// Number of columns spanned.
    pub col_span: u32,
}

impl GridCell {
    /// Exclusive end row: `row + row_span`.
    pub fn end_row(&self) -> u32 {
        self.row + self.row_span
    }

    /// Exclusive end column: `col + col_span`.
    pub fn end_col(&self) -> u32 {
        self.col + self.col_span
    }

    /// Number of track cells covered: `row_span * col_span`.
    pub fn area(&self) -> u32 {
        self.row_span * self.col_span
    }
}

/// A grid composed of row and column tracks.
#[derive(Debug, Default, Clone)]
pub struct LayoutGrid {
    /// Row tracks in order.
    pub rows: Vec<GridTrack>,
    /// Column tracks in order.
    pub cols: Vec<GridTrack>,
}

impl LayoutGrid {
    /// Append a row track.
    pub fn add_row(&mut self, t: GridTrack) {
        self.rows.push(t);
    }

    /// Append a column track.
    pub fn add_col(&mut self, t: GridTrack) {
        self.cols.push(t);
    }

    /// Total number of cells: `rows.len() * cols.len()`.
    pub fn cell_count(&self) -> u32 {
        self.rows.len() as u32 * self.cols.len() as u32
    }

    /// Rows whose size is flexible (Fraction or Auto).
    pub fn flexible_rows(&self) -> Vec<&GridTrack> {
        self.rows.iter().filter(|r| r.size.is_flexible()).collect()
    }
}

/// An element placed inside a grid.
#[derive(Debug, Clone)]
pub struct GridPlacement {
    /// The cell region the element occupies.
    pub cell: GridCell,
    /// Identifier of the canvas element being placed.
    pub element_id: u64,
}

impl GridPlacement {
    /// Returns `true` when the cell's end row and end column are within the grid bounds.
    pub fn fits_in_grid(&self, grid: &LayoutGrid) -> bool {
        self.cell.end_row() <= grid.rows.len() as u32
            && self.cell.end_col() <= grid.cols.len() as u32
    }
}

#[cfg(test)]
mod layout_grid_tests {
    use super::*;

    #[test]
    fn track_size_is_flexible() {
        assert!(!TrackSize::Fixed(50.0).is_flexible());
        assert!(TrackSize::Fraction(0.5).is_flexible());
        assert!(TrackSize::Auto.is_flexible());
    }

    #[test]
    fn track_size_min_size() {
        assert_eq!(TrackSize::Fixed(80.0).min_size(), 80.0);
        assert_eq!(TrackSize::Fraction(0.3).min_size(), 0.0);
        assert_eq!(TrackSize::Auto.min_size(), 0.0);
    }

    #[test]
    fn grid_track_total_size_fixed() {
        let t = GridTrack { index: 0, size: TrackSize::Fixed(120.0), gap: 8.0 };
        assert_eq!(t.total_size(999.0), 120.0);
    }

    #[test]
    fn grid_track_total_size_fraction() {
        let t = GridTrack { index: 1, size: TrackSize::Fraction(0.25), gap: 4.0 };
        assert!((t.total_size(200.0) - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cell_end_row_end_col() {
        let c = GridCell { row: 2, col: 3, row_span: 2, col_span: 4 };
        assert_eq!(c.end_row(), 4);
        assert_eq!(c.end_col(), 7);
    }

    #[test]
    fn cell_area() {
        let c = GridCell { row: 0, col: 0, row_span: 3, col_span: 5 };
        assert_eq!(c.area(), 15);
    }

    #[test]
    fn grid_cell_count() {
        let mut g = LayoutGrid::default();
        g.add_row(GridTrack { index: 0, size: TrackSize::Fixed(100.0), gap: 0.0 });
        g.add_row(GridTrack { index: 1, size: TrackSize::Auto, gap: 0.0 });
        g.add_col(GridTrack { index: 0, size: TrackSize::Fixed(50.0), gap: 0.0 });
        g.add_col(GridTrack { index: 1, size: TrackSize::Fixed(50.0), gap: 0.0 });
        g.add_col(GridTrack { index: 2, size: TrackSize::Fraction(0.5), gap: 0.0 });
        assert_eq!(g.cell_count(), 6);
    }

    #[test]
    fn grid_flexible_rows() {
        let mut g = LayoutGrid::default();
        g.add_row(GridTrack { index: 0, size: TrackSize::Fixed(40.0), gap: 0.0 });
        g.add_row(GridTrack { index: 1, size: TrackSize::Auto, gap: 0.0 });
        g.add_row(GridTrack { index: 2, size: TrackSize::Fraction(0.5), gap: 0.0 });
        let flex = g.flexible_rows();
        assert_eq!(flex.len(), 2);
        assert_eq!(flex[0].index, 1);
        assert_eq!(flex[1].index, 2);
    }

    #[test]
    fn placement_fits_in_grid() {
        let mut g = LayoutGrid::default();
        for i in 0..3 {
            g.add_row(GridTrack { index: i, size: TrackSize::Fixed(50.0), gap: 0.0 });
            g.add_col(GridTrack { index: i, size: TrackSize::Fixed(50.0), gap: 0.0 });
        }

        // Fits: ends at row 2, col 2 (both <= 3).
        let good = GridPlacement {
            cell: GridCell { row: 1, col: 1, row_span: 1, col_span: 1 },
            element_id: 1,
        };
        assert!(good.fits_in_grid(&g));

        // Does not fit: end_row = 4 > 3.
        let bad = GridPlacement {
            cell: GridCell { row: 2, col: 0, row_span: 2, col_span: 1 },
            element_id: 2,
        };
        assert!(!bad.fits_in_grid(&g));
    }
}
