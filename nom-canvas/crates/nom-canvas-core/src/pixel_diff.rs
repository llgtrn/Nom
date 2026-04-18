//! Region-level pixel diff primitives for canvas rendering verification.
//!
//! Distinct from nom-gpui's pixel_diff (pixel-level comparison); this module
//! operates on spatial regions and aggregated change statistics.

/// Axis-aligned rectangular region in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRegion {
    /// Left edge (inclusive).
    pub x: u32,
    /// Top edge (inclusive).
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PixelRegion {
    /// Total pixel count in the region.
    pub fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Returns `true` if `(px, py)` falls within this region.
    pub fn contains_point(&self, px: u32, py: u32) -> bool {
        px >= self.x
            && px < self.x + self.width
            && py >= self.y
            && py < self.y + self.height
    }
}

/// Aggregated diff result for a single [`PixelRegion`].
#[derive(Debug, Clone)]
pub struct PixelDiff {
    /// The region this diff covers.
    pub region: PixelRegion,
    /// Number of pixels that changed.
    pub changed_pixels: u64,
    /// Largest per-channel absolute difference seen across all changed pixels.
    pub max_delta: u8,
}

impl PixelDiff {
    /// Fraction of region pixels that changed (0.0–1.0). Returns `0.0` when area is zero.
    pub fn change_ratio(&self) -> f64 {
        let area = self.region.area();
        if area == 0 {
            return 0.0;
        }
        self.changed_pixels as f64 / area as f64
    }

    /// Returns `true` when [`change_ratio`] exceeds `threshold`.
    pub fn is_significant(&self, threshold: f64) -> bool {
        self.change_ratio() > threshold
    }
}

/// Acceptance criteria for a diff.
#[derive(Debug, Clone, Copy)]
pub struct DiffThreshold {
    /// Maximum allowable change ratio (inclusive).
    pub max_change_ratio: f64,
    /// Maximum allowable per-channel delta (inclusive).
    pub max_delta: u8,
}

impl DiffThreshold {
    /// Returns `true` when `diff` satisfies both constraints.
    pub fn passes(&self, diff: &PixelDiff) -> bool {
        diff.change_ratio() <= self.max_change_ratio && diff.max_delta <= self.max_delta
    }
}

/// Collection of [`PixelDiff`] results for a frame or test scenario.
#[derive(Debug, Default)]
pub struct DiffReport {
    /// All recorded diffs.
    pub diffs: Vec<PixelDiff>,
}

impl DiffReport {
    /// Append a diff to this report.
    pub fn add(&mut self, d: PixelDiff) {
        self.diffs.push(d);
    }

    /// Number of diffs whose change ratio exceeds `threshold`.
    pub fn significant_count(&self, threshold: f64) -> usize {
        self.diffs.iter().filter(|d| d.is_significant(threshold)).count()
    }

    /// Sum of changed pixels across all diffs.
    pub fn total_changed_pixels(&self) -> u64 {
        self.diffs.iter().map(|d| d.changed_pixels).sum()
    }

    /// The diff with the highest change ratio, or `None` if the report is empty.
    pub fn worst_diff(&self) -> Option<&PixelDiff> {
        self.diffs
            .iter()
            .max_by(|a, b| a.change_ratio().partial_cmp(&b.change_ratio()).unwrap())
    }
}

/// Applies a [`DiffThreshold`] to an entire [`DiffReport`].
pub struct RegionDiffer {
    /// The threshold used for all evaluations.
    pub threshold: DiffThreshold,
}

impl RegionDiffer {
    /// Construct a new differ with the given acceptance criteria.
    pub fn new(max_ratio: f64, max_delta: u8) -> Self {
        Self {
            threshold: DiffThreshold {
                max_change_ratio: max_ratio,
                max_delta,
            },
        }
    }

    /// Returns `true` when every diff in `report` passes the threshold.
    pub fn passes_all(&self, report: &DiffReport) -> bool {
        report.diffs.iter().all(|d| self.threshold.passes(d))
    }
}

#[cfg(test)]
mod pixel_diff_tests {
    use super::*;

    fn region(x: u32, y: u32, w: u32, h: u32) -> PixelRegion {
        PixelRegion { x, y, width: w, height: h }
    }

    fn diff(region: PixelRegion, changed: u64, delta: u8) -> PixelDiff {
        PixelDiff { region, changed_pixels: changed, max_delta: delta }
    }

    #[test]
    fn region_area() {
        assert_eq!(region(0, 0, 10, 20).area(), 200);
        assert_eq!(region(5, 5, 0, 10).area(), 0);
    }

    #[test]
    fn region_contains_point_true() {
        let r = region(10, 20, 30, 40);
        assert!(r.contains_point(10, 20), "top-left corner");
        assert!(r.contains_point(25, 35), "interior point");
        assert!(r.contains_point(39, 59), "bottom-right exclusive boundary");
    }

    #[test]
    fn region_contains_point_false() {
        let r = region(10, 20, 30, 40);
        assert!(!r.contains_point(9, 20), "left of region");
        assert!(!r.contains_point(40, 20), "right exclusive edge");
        assert!(!r.contains_point(10, 60), "below region");
    }

    #[test]
    fn diff_change_ratio() {
        let d = diff(region(0, 0, 10, 10), 25, 0);
        assert!((d.change_ratio() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn diff_change_ratio_zero_area() {
        let d = diff(region(0, 0, 0, 10), 0, 0);
        assert_eq!(d.change_ratio(), 0.0);
    }

    #[test]
    fn diff_is_significant() {
        let d = diff(region(0, 0, 10, 10), 60, 10);
        assert!(d.is_significant(0.5), "60% > 50% threshold");
        assert!(!d.is_significant(0.7), "60% not > 70% threshold");
    }

    #[test]
    fn threshold_passes_true() {
        let t = DiffThreshold { max_change_ratio: 0.5, max_delta: 20 };
        let d = diff(region(0, 0, 10, 10), 30, 15);
        assert!(t.passes(&d));
    }

    #[test]
    fn threshold_passes_false_ratio_too_high() {
        let t = DiffThreshold { max_change_ratio: 0.2, max_delta: 255 };
        let d = diff(region(0, 0, 10, 10), 50, 5);
        assert!(!t.passes(&d), "change_ratio 0.5 exceeds max 0.2");
    }

    #[test]
    fn report_significant_count() {
        let mut report = DiffReport::default();
        report.add(diff(region(0, 0, 10, 10), 80, 5));  // 80% significant
        report.add(diff(region(0, 0, 10, 10), 10, 5));  // 10% not significant
        report.add(diff(region(0, 0, 10, 10), 60, 5));  // 60% significant
        assert_eq!(report.significant_count(0.5), 2);
    }

    #[test]
    fn report_worst_diff_found() {
        let mut report = DiffReport::default();
        report.add(diff(region(0, 0, 10, 10), 10, 1));  // 10%
        report.add(diff(region(0, 0, 10, 10), 90, 1));  // 90% — worst
        report.add(diff(region(0, 0, 10, 10), 50, 1));  // 50%
        let worst = report.worst_diff().expect("report is non-empty");
        assert!((worst.change_ratio() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn differ_passes_all_with_all_passing_diffs() {
        let differ = RegionDiffer::new(0.5, 30);
        let mut report = DiffReport::default();
        report.add(diff(region(0, 0, 10, 10), 20, 10));  // 20%, delta 10
        report.add(diff(region(0, 0, 10, 10), 40, 25));  // 40%, delta 25
        assert!(differ.passes_all(&report));
    }
}
