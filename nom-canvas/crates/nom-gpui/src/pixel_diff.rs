/// Pixel-difference utilities for visual regression testing.
///
/// # Format
/// Images are represented as flat RGBA byte buffers: `width * height * 4` bytes,
/// row-major, top-to-bottom, left-to-right.  Each pixel is four bytes: R, G, B, A.
///
/// # Baseline files
/// A baseline is stored as a tiny header-prefixed raw file:
/// ```text
/// magic:   b"NOMRAW\0\0"  (8 bytes)
/// width:   u32 little-endian (4 bytes)
/// height:  u32 little-endian (4 bytes)
/// pixels:  width * height * 4 bytes (RGBA)
/// ```
/// If the baseline file does not exist, `diff_or_save` writes the current
/// frame as the new baseline and returns `Ok(DiffResult::BaselineSaved)`.
use std::io::{self, Read, Write};
use std::path::Path;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// An in-memory RGBA image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawImage {
    pub width: u32,
    pub height: u32,
    /// Row-major RGBA bytes, length == width * height * 4.
    pub pixels: Vec<u8>,
}

impl RawImage {
    /// Construct from raw RGBA bytes.  Panics if `pixels.len() != width * height * 4`.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        assert_eq!(
            pixels.len(),
            (width as usize) * (height as usize) * 4,
            "pixel buffer length mismatch"
        );
        Self { width, height, pixels }
    }

    /// Create a solid-colour image of the given dimensions.
    pub fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Self {
        let pixels = rgba.iter().copied().cycle().take((width as usize) * (height as usize) * 4).collect();
        Self { width, height, pixels }
    }

    /// Total pixel count.
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}

/// Result of a diff-or-save call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffResult {
    /// No baseline existed; the current frame was saved as the new baseline.
    BaselineSaved,
    /// Baseline existed; comparison ran.
    Compared(DiffStats),
}

/// Statistics from a pixel-level comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffStats {
    /// Total pixels in the image.
    pub total_pixels: usize,
    /// Number of pixels whose per-channel L1 distance exceeds `threshold`.
    pub differing_pixels: usize,
    /// The threshold used (intensity units per channel, 0–255).
    pub threshold: u8,
}

impl DiffStats {
    /// Fraction of pixels that differ, in range [0.0, 1.0].
    pub fn diff_fraction(&self) -> f64 {
        if self.total_pixels == 0 {
            0.0
        } else {
            self.differing_pixels as f64 / self.total_pixels as f64
        }
    }

    /// Returns `true` if the diff fraction is within `max_fraction` (0.0–1.0).
    pub fn within_tolerance(&self, max_fraction: f64) -> bool {
        self.diff_fraction() <= max_fraction
    }
}

// ---------------------------------------------------------------------------
// Magic constant
// ---------------------------------------------------------------------------

const MAGIC: &[u8; 8] = b"NOMRAW\0\0";

// ---------------------------------------------------------------------------
// Core diff logic
// ---------------------------------------------------------------------------

/// Compare two same-dimension RGBA images pixel by pixel.
///
/// A pixel is counted as "differing" if the maximum per-channel absolute
/// difference exceeds `threshold` (0–255).
///
/// # Errors
/// Returns `Err` if the images have different dimensions.
pub fn pixel_diff(a: &RawImage, b: &RawImage, threshold: u8) -> Result<DiffStats, String> {
    if a.width != b.width || a.height != b.height {
        return Err(format!(
            "image size mismatch: {}x{} vs {}x{}",
            a.width, a.height, b.width, b.height
        ));
    }
    let total_pixels = a.pixel_count();
    let mut differing_pixels = 0usize;
    for (chunk_a, chunk_b) in a.pixels.chunks_exact(4).zip(b.pixels.chunks_exact(4)) {
        let max_channel_diff = chunk_a
            .iter()
            .zip(chunk_b.iter())
            .map(|(&x, &y)| x.abs_diff(y))
            .max()
            .unwrap_or(0);
        if max_channel_diff > threshold {
            differing_pixels += 1;
        }
    }
    Ok(DiffStats { total_pixels, differing_pixels, threshold })
}

// ---------------------------------------------------------------------------
// Baseline I/O
// ---------------------------------------------------------------------------

/// Write a raw baseline file.
pub fn save_baseline(path: &Path, image: &RawImage) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(MAGIC)?;
    file.write_all(&image.width.to_le_bytes())?;
    file.write_all(&image.height.to_le_bytes())?;
    file.write_all(&image.pixels)?;
    Ok(())
}

/// Load a raw baseline file previously saved by `save_baseline`.
pub fn load_baseline(path: &Path) -> io::Result<RawImage> {
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a NOMRAW baseline file"));
    }
    let mut w_buf = [0u8; 4];
    let mut h_buf = [0u8; 4];
    file.read_exact(&mut w_buf)?;
    file.read_exact(&mut h_buf)?;
    let width = u32::from_le_bytes(w_buf);
    let height = u32::from_le_bytes(h_buf);
    let pixel_len = (width as usize) * (height as usize) * 4;
    let mut pixels = vec![0u8; pixel_len];
    file.read_exact(&mut pixels)?;
    Ok(RawImage { width, height, pixels })
}

// ---------------------------------------------------------------------------
// Combined diff-or-save
// ---------------------------------------------------------------------------

/// Compare `current` against the baseline at `baseline_path`.
///
/// - If the file does not exist, saves `current` as the new baseline and
///   returns `Ok(DiffResult::BaselineSaved)`.
/// - If the file exists, loads it and runs `pixel_diff` with `threshold`.
pub fn diff_or_save(
    baseline_path: &Path,
    current: &RawImage,
    threshold: u8,
) -> Result<DiffResult, String> {
    if !baseline_path.exists() {
        save_baseline(baseline_path, current)
            .map_err(|e| format!("failed to save baseline: {e}"))?;
        return Ok(DiffResult::BaselineSaved);
    }
    let baseline =
        load_baseline(baseline_path).map_err(|e| format!("failed to load baseline: {e}"))?;
    let stats = pixel_diff(&baseline, current, threshold)?;
    Ok(DiffResult::Compared(stats))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -----------------------------------------------------------------------
    // pixel_diff — unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn identical_images_produce_zero_diff() {
        let img = RawImage::solid(4, 4, [128, 64, 32, 255]);
        let stats = pixel_diff(&img, &img, 10).unwrap();
        assert_eq!(stats.differing_pixels, 0);
        assert_eq!(stats.total_pixels, 16);
        assert_eq!(stats.diff_fraction(), 0.0);
    }

    #[test]
    fn single_pixel_change_detected_correctly() {
        let mut a = RawImage::solid(2, 2, [0, 0, 0, 255]);
        // Change pixel (0,0) red channel by 50 — exceeds threshold of 10.
        a.pixels[0] = 50;
        let b = RawImage::solid(2, 2, [0, 0, 0, 255]);
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(stats.differing_pixels, 1);
        assert_eq!(stats.total_pixels, 4);
    }

    #[test]
    fn pixel_below_threshold_not_counted() {
        let a = RawImage::solid(3, 3, [100, 100, 100, 255]);
        // Diff of 5 on one channel — below threshold of 10.
        let mut b = RawImage::solid(3, 3, [105, 100, 100, 255]);
        // Also make pixel (1,1) differ by exactly threshold — not counted.
        b.pixels[4 * 4] = 110; // pixel index 4, R channel
        let stats = pixel_diff(&a, &b, 10).unwrap();
        // Pixel (0,0..8): all channels diff <= 5, not counted.
        // Pixel at offset 4 (second row first column in 3x3): diff == 10, NOT > 10.
        // So none should be counted.
        assert_eq!(stats.differing_pixels, 0);
    }

    #[test]
    fn all_pixels_above_threshold_counted() {
        let a = RawImage::solid(5, 5, [0, 0, 0, 255]);
        let b = RawImage::solid(5, 5, [50, 50, 50, 255]);
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(stats.differing_pixels, 25);
        assert_eq!(stats.total_pixels, 25);
        assert_eq!(stats.diff_fraction(), 1.0);
    }

    #[test]
    fn dimension_mismatch_returns_error() {
        let a = RawImage::solid(2, 2, [0, 0, 0, 255]);
        let b = RawImage::solid(3, 3, [0, 0, 0, 255]);
        assert!(pixel_diff(&a, &b, 10).is_err());
    }

    #[test]
    fn within_tolerance_passes_for_small_diff() {
        let a = RawImage::solid(10, 10, [0, 0, 0, 255]);
        let mut b = a.clone();
        // Change 3 pixels out of 100.
        b.pixels[0] = 50;
        b.pixels[4] = 50;
        b.pixels[8] = 50;
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(stats.differing_pixels, 3);
        // 3/100 = 3% — within 5% tolerance.
        assert!(stats.within_tolerance(0.05));
        // 3/100 = 3% — NOT within 2% tolerance.
        assert!(!stats.within_tolerance(0.02));
    }

    #[test]
    fn threshold_math_boundary_cases() {
        // Channel diff == threshold: NOT counted (condition is strictly greater-than).
        let a = RawImage::solid(1, 1, [100, 0, 0, 255]);
        let b = RawImage::solid(1, 1, [110, 0, 0, 255]); // diff == 10 == threshold
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(stats.differing_pixels, 0, "diff == threshold should not be counted");

        // Channel diff == threshold + 1: IS counted.
        let c = RawImage::solid(1, 1, [111, 0, 0, 255]); // diff == 11 > 10
        let stats2 = pixel_diff(&a, &c, 10).unwrap();
        assert_eq!(stats2.differing_pixels, 1, "diff > threshold should be counted");
    }

    #[test]
    fn zero_size_image_has_zero_fraction() {
        // Edge case: 0x0 image (pixel_count == 0).
        let a = RawImage { width: 0, height: 0, pixels: vec![] };
        let stats = DiffStats { total_pixels: 0, differing_pixels: 0, threshold: 10 };
        assert_eq!(stats.diff_fraction(), 0.0);
        // pixel_diff of two 0x0 images should succeed with 0 differing.
        let result = pixel_diff(&a, &a, 10).unwrap();
        assert_eq!(result.differing_pixels, 0);
    }

    // -----------------------------------------------------------------------
    // Baseline I/O round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn save_and_load_baseline_round_trips() {
        let img = RawImage::solid(8, 6, [10, 20, 30, 255]);
        let dir = std::env::temp_dir();
        let path: PathBuf = dir.join("nom_gpui_test_baseline_round_trip.nomraw");
        // Clean up any leftover.
        let _ = std::fs::remove_file(&path);

        save_baseline(&path, &img).expect("save failed");
        let loaded = load_baseline(&path).expect("load failed");
        assert_eq!(loaded, img);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_baseline_returns_error() {
        let path = PathBuf::from("/nonexistent/path/baseline.nomraw");
        assert!(load_baseline(&path).is_err());
    }

    #[test]
    fn diff_or_save_saves_when_no_baseline_exists() {
        let img = RawImage::solid(4, 4, [255, 0, 0, 255]);
        let dir = std::env::temp_dir();
        let path: PathBuf = dir.join("nom_gpui_diff_or_save_baseline.nomraw");
        let _ = std::fs::remove_file(&path);

        let result = diff_or_save(&path, &img, 10).unwrap();
        assert_eq!(result, DiffResult::BaselineSaved);
        assert!(path.exists(), "baseline file should have been created");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn diff_or_save_compares_when_baseline_exists() {
        let img = RawImage::solid(4, 4, [0, 255, 0, 255]);
        let dir = std::env::temp_dir();
        let path: PathBuf = dir.join("nom_gpui_diff_or_save_compare.nomraw");
        let _ = std::fs::remove_file(&path);

        // First call — saves baseline.
        diff_or_save(&path, &img, 10).unwrap();

        // Second call — identical image, should compare with 0 differing.
        let result = diff_or_save(&path, &img, 10).unwrap();
        match result {
            DiffResult::Compared(stats) => {
                assert_eq!(stats.differing_pixels, 0);
            }
            DiffResult::BaselineSaved => panic!("expected comparison, got baseline save"),
        }

        let _ = std::fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // Screenshot assertion helper
    // -----------------------------------------------------------------------

    /// Demonstrates the full pixel-difference assertion pattern for a screenshot.
    ///
    /// In a real test scenario, `current_frame` would be populated from the
    /// wgpu framebuffer; here we use a synthetic image to exercise the full path.
    #[test]
    fn screenshot_pixel_diff_assertion_pattern() {
        // 5% tolerance: at most 5% of pixels may change by more than 10 intensity units.
        const TOLERANCE: f64 = 0.05;
        const THRESHOLD: u8 = 10;

        let dir = std::env::temp_dir();
        let baseline_path: PathBuf =
            dir.join("nom_gpui_window_first_paint_pixel_diff_test.nomraw");
        let _ = std::fs::remove_file(&baseline_path);

        let width = 640u32;
        let height = 420u32;
        // Simulate a "current frame": uniform grey background.
        let current_frame = RawImage::solid(width, height, [40, 40, 40, 255]);

        let result = diff_or_save(&baseline_path, &current_frame, THRESHOLD).unwrap();

        match result {
            DiffResult::BaselineSaved => {
                // First run — baseline created, no diff to assert.
            }
            DiffResult::Compared(stats) => {
                assert!(
                    stats.within_tolerance(TOLERANCE),
                    "pixel diff {:.2}% exceeds {:.0}% tolerance ({} / {} pixels changed)",
                    stats.diff_fraction() * 100.0,
                    TOLERANCE * 100.0,
                    stats.differing_pixels,
                    stats.total_pixels,
                );
            }
        }

        let _ = std::fs::remove_file(&baseline_path);
    }

    // -----------------------------------------------------------------------
    // AE Wave: Additional pixel_diff tests
    // -----------------------------------------------------------------------

    #[test]
    fn threshold_exactly_at_boundary_not_counted() {
        // diff == threshold → NOT counted (strictly greater-than semantics).
        let a = RawImage::solid(2, 2, [50, 50, 50, 255]);
        let b = RawImage::solid(2, 2, [60, 50, 50, 255]); // diff = 10 == threshold
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(
            stats.differing_pixels, 0,
            "diff exactly at threshold must NOT be counted (strictly >)"
        );
    }

    #[test]
    fn threshold_one_over_boundary_is_counted() {
        // diff == threshold + 1 → IS counted.
        let a = RawImage::solid(2, 2, [50, 50, 50, 255]);
        let b = RawImage::solid(2, 2, [61, 50, 50, 255]); // diff = 11 > 10
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(
            stats.differing_pixels, 4,
            "diff one above threshold must be counted for all 4 pixels"
        );
    }

    #[test]
    fn large_image_1000x1000_zeros_has_zero_diff() {
        // Performance test: 1000x1000 = 1M pixels, all identical.
        let a = RawImage::solid(1000, 1000, [0, 0, 0, 255]);
        let b = RawImage::solid(1000, 1000, [0, 0, 0, 255]);
        let stats = pixel_diff(&a, &b, 0).unwrap();
        assert_eq!(stats.total_pixels, 1_000_000);
        assert_eq!(stats.differing_pixels, 0);
    }

    #[test]
    fn large_image_1000x1000_all_different() {
        let a = RawImage::solid(1000, 1000, [0, 0, 0, 255]);
        let b = RawImage::solid(1000, 1000, [128, 128, 128, 255]);
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(stats.total_pixels, 1_000_000);
        assert_eq!(stats.differing_pixels, 1_000_000);
        assert_eq!(stats.diff_fraction(), 1.0);
    }

    #[test]
    fn alpha_channel_difference_is_detected() {
        // Diff on alpha channel alone must be caught.
        let a = RawImage::solid(1, 1, [128, 128, 128, 0]);
        let b = RawImage::solid(1, 1, [128, 128, 128, 255]); // alpha diff = 255 >> threshold
        let stats = pixel_diff(&a, &b, 10).unwrap();
        assert_eq!(stats.differing_pixels, 1, "alpha-channel diff must be detected");
    }

    #[test]
    fn raw_image_new_panics_on_wrong_size() {
        let result = std::panic::catch_unwind(|| {
            RawImage::new(2, 2, vec![0u8; 5]); // wrong length — must panic
        });
        assert!(result.is_err(), "RawImage::new with wrong pixel length must panic");
    }

    #[test]
    fn raw_image_pixel_count_matches_dimensions() {
        let img = RawImage::solid(7, 11, [0, 0, 0, 255]);
        assert_eq!(img.pixel_count(), 77);
        assert_eq!(img.pixels.len(), 77 * 4);
    }

    #[test]
    fn raw_image_solid_fills_all_pixels() {
        let img = RawImage::solid(3, 3, [10, 20, 30, 40]);
        for chunk in img.pixels.chunks_exact(4) {
            assert_eq!(chunk, &[10, 20, 30, 40], "every pixel must match the solid color");
        }
    }

    #[test]
    fn diff_stats_within_tolerance_at_exact_boundary() {
        let stats = DiffStats { total_pixels: 100, differing_pixels: 5, threshold: 10 };
        // 5/100 = 5% — exactly at 5% tolerance boundary → within.
        assert!(stats.within_tolerance(0.05));
    }

    #[test]
    fn diff_stats_within_tolerance_one_over_fails() {
        let stats = DiffStats { total_pixels: 100, differing_pixels: 6, threshold: 10 };
        // 6/100 = 6% > 5% tolerance → not within.
        assert!(!stats.within_tolerance(0.05));
    }

    #[test]
    fn diff_fraction_all_match_is_zero() {
        let stats = DiffStats { total_pixels: 50, differing_pixels: 0, threshold: 10 };
        assert_eq!(stats.diff_fraction(), 0.0);
    }

    #[test]
    fn diff_fraction_half_differ() {
        let stats = DiffStats { total_pixels: 100, differing_pixels: 50, threshold: 5 };
        assert!((stats.diff_fraction() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn pixel_diff_threshold_zero_counts_any_change() {
        // threshold=0: any non-zero difference is counted.
        let a = RawImage::solid(2, 2, [10, 10, 10, 255]);
        let b = RawImage::solid(2, 2, [11, 10, 10, 255]); // diff=1 > 0
        let stats = pixel_diff(&a, &b, 0).unwrap();
        assert_eq!(stats.differing_pixels, 4, "threshold=0 must catch any change");
    }

    #[test]
    fn pixel_diff_threshold_255_counts_nothing_under_max() {
        // threshold=255: no pixel can exceed max channel value 255, so nothing counted.
        let a = RawImage::solid(4, 4, [0, 0, 0, 0]);
        let b = RawImage::solid(4, 4, [255, 255, 255, 255]); // diff=255, not > 255
        let stats = pixel_diff(&a, &b, 255).unwrap();
        assert_eq!(stats.differing_pixels, 0, "diff==255 with threshold=255 must not count");
    }

    #[test]
    fn load_corrupted_magic_returns_error() {
        let dir = std::env::temp_dir();
        let path = dir.join("nom_gpui_bad_magic_test.nomraw");
        // Write garbage header.
        std::fs::write(&path, b"BADMAGIC12345678").unwrap();
        let result = load_baseline(&path);
        assert!(result.is_err(), "corrupted magic must return an error");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn diff_or_save_saves_on_missing_and_compares_on_second_call_different_image() {
        let dir = std::env::temp_dir();
        let path = dir.join("nom_gpui_diff_compare_changed.nomraw");
        let _ = std::fs::remove_file(&path);

        let original = RawImage::solid(4, 4, [200, 200, 200, 255]);
        let changed = RawImage::solid(4, 4, [0, 0, 0, 255]); // big diff

        // Save baseline.
        let r1 = diff_or_save(&path, &original, 10).unwrap();
        assert_eq!(r1, DiffResult::BaselineSaved);

        // Compare with very different image.
        let r2 = diff_or_save(&path, &changed, 10).unwrap();
        match r2 {
            DiffResult::Compared(stats) => {
                assert_eq!(stats.differing_pixels, 16, "all 16 pixels must differ");
                assert_eq!(stats.diff_fraction(), 1.0);
            }
            DiffResult::BaselineSaved => panic!("expected comparison on second call"),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn raw_image_equality_holds_for_identical() {
        let a = RawImage::solid(3, 3, [1, 2, 3, 4]);
        let b = RawImage::solid(3, 3, [1, 2, 3, 4]);
        assert_eq!(a, b);
    }

    #[test]
    fn raw_image_clone_is_equal() {
        let a = RawImage::solid(5, 5, [10, 20, 30, 255]);
        let b = a.clone();
        assert_eq!(a, b);
    }
}
