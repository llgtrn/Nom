//! Video-composition model: scene-based frame routing for Nom video exports.
//!
//! A `VideoComposition` is a fixed-framerate timeline of `SceneEntry`s, each
//! referring to a content-addressed Artifact (via `ContentHash`) that the
//! renderer paints during `from_frame..from_frame + duration`.
//!
//! Does NOT own ffmpeg process logic — that lives in the media/video backend.
#![deny(unsafe_code)]

pub type ContentHash = String;

#[derive(Clone, Debug, PartialEq)]
pub struct SceneEntry {
    pub from_frame: u32,
    pub duration: u32,
    pub entity_hash: ContentHash,
}

impl SceneEntry {
    pub fn new(from_frame: u32, duration: u32, entity_hash: impl Into<ContentHash>) -> Self {
        Self { from_frame, duration, entity_hash: entity_hash.into() }
    }

    pub fn end_frame(&self) -> u32 {
        self.from_frame + self.duration
    }

    pub fn contains(&self, frame: u32) -> bool {
        frame >= self.from_frame && frame < self.end_frame()
    }

    /// Frame index relative to the scene's start (0-based).  Returns None if
    /// `frame` is outside the scene.
    pub fn relative_frame(&self, frame: u32) -> Option<u32> {
        if self.contains(frame) { Some(frame - self.from_frame) } else { None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoComposition {
    pub fps: u16,
    pub duration_frames: u32,
    pub width: u32,
    pub height: u32,
    pub scenes: Vec<SceneEntry>,
}

impl VideoComposition {
    pub fn new(fps: u16, duration_frames: u32, width: u32, height: u32) -> Self {
        Self { fps, duration_frames, width, height, scenes: Vec::new() }
    }

    pub fn add_scene(&mut self, scene: SceneEntry) {
        self.scenes.push(scene);
    }

    /// Scenes whose range contains `frame`.  Overlapping scenes all returned —
    /// the renderer decides how to composite (later-indexed wins by default).
    pub fn active_scenes(&self, frame: u32) -> Vec<&SceneEntry> {
        self.scenes.iter().filter(|s| s.contains(frame)).collect()
    }

    pub fn duration_ms(&self) -> u64 {
        if self.fps == 0 { return 0; }
        (self.duration_frames as u64 * 1000) / self.fps as u64
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 { 1.0 } else { self.width as f32 / self.height as f32 }
    }

    /// True if all scenes fit inside `duration_frames` and `fps > 0`.
    pub fn validate(&self) -> Result<(), CompositionError> {
        if self.fps == 0 {
            return Err(CompositionError::InvalidFps);
        }
        if self.width == 0 || self.height == 0 {
            return Err(CompositionError::InvalidDimensions);
        }
        for s in &self.scenes {
            if s.end_frame() > self.duration_frames {
                return Err(CompositionError::SceneOutOfRange {
                    end_frame: s.end_frame(),
                    duration_frames: self.duration_frames,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CompositionError {
    #[error("fps must be > 0")]
    InvalidFps,
    #[error("dimensions must be > 0")]
    InvalidDimensions,
    #[error("scene ends at frame {end_frame} but composition has only {duration_frames} frames")]
    SceneOutOfRange { end_frame: u32, duration_frames: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_entry_new_sets_fields() {
        let s = SceneEntry::new(10, 30, "abc");
        assert_eq!(s.from_frame, 10);
        assert_eq!(s.duration, 30);
        assert_eq!(s.entity_hash, "abc");
    }

    #[test]
    fn end_frame_equals_from_plus_duration() {
        let s = SceneEntry::new(5, 20, "h");
        assert_eq!(s.end_frame(), 25);
    }

    #[test]
    fn contains_inside_returns_true() {
        let s = SceneEntry::new(10, 10, "h");
        assert!(s.contains(10));
        assert!(s.contains(15));
        assert!(s.contains(19));
    }

    #[test]
    fn contains_at_end_frame_returns_false() {
        let s = SceneEntry::new(10, 10, "h");
        assert!(!s.contains(20)); // exclusive upper bound
        assert!(!s.contains(9));  // before start
    }

    #[test]
    fn relative_frame_inside_returns_some_offset() {
        let s = SceneEntry::new(10, 10, "h");
        assert_eq!(s.relative_frame(10), Some(0));
        assert_eq!(s.relative_frame(13), Some(3));
        assert_eq!(s.relative_frame(19), Some(9));
    }

    #[test]
    fn relative_frame_outside_returns_none() {
        let s = SceneEntry::new(10, 10, "h");
        assert_eq!(s.relative_frame(9), None);
        assert_eq!(s.relative_frame(20), None);
    }

    #[test]
    fn active_scenes_empty_comp_returns_empty() {
        let comp = VideoComposition::new(30, 90, 1920, 1080);
        assert!(comp.active_scenes(0).is_empty());
    }

    #[test]
    fn active_scenes_at_from_frame_returns_scene() {
        let mut comp = VideoComposition::new(30, 90, 1920, 1080);
        comp.add_scene(SceneEntry::new(0, 30, "a"));
        assert_eq!(comp.active_scenes(0).len(), 1);
    }

    #[test]
    fn active_scenes_at_end_frame_not_returned() {
        let mut comp = VideoComposition::new(30, 90, 1920, 1080);
        comp.add_scene(SceneEntry::new(0, 30, "a"));
        assert!(comp.active_scenes(30).is_empty());
    }

    #[test]
    fn active_scenes_overlapping_both_returned() {
        let mut comp = VideoComposition::new(30, 90, 1920, 1080);
        comp.add_scene(SceneEntry::new(0, 60, "a"));
        comp.add_scene(SceneEntry::new(30, 30, "b"));
        let active = comp.active_scenes(45);
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn duration_ms_30fps_90frames_equals_3000() {
        let comp = VideoComposition::new(30, 90, 1920, 1080);
        assert_eq!(comp.duration_ms(), 3000);
    }

    #[test]
    fn duration_ms_fps_zero_returns_zero_no_panic() {
        let comp = VideoComposition::new(0, 90, 1920, 1080);
        assert_eq!(comp.duration_ms(), 0);
    }

    #[test]
    fn aspect_ratio_1920x1080_approx_1777() {
        let comp = VideoComposition::new(30, 90, 1920, 1080);
        let r = comp.aspect_ratio();
        assert!((r - 1.7777_f32).abs() < 0.001, "got {r}");
    }

    #[test]
    fn aspect_ratio_height_zero_returns_1() {
        let comp = VideoComposition::new(30, 90, 1920, 0);
        assert_eq!(comp.aspect_ratio(), 1.0);
    }

    #[test]
    fn validate_ok_for_valid_comp() {
        let mut comp = VideoComposition::new(30, 90, 1920, 1080);
        comp.add_scene(SceneEntry::new(0, 90, "a"));
        assert!(comp.validate().is_ok());
    }

    #[test]
    fn validate_fps_zero_returns_invalid_fps() {
        let comp = VideoComposition::new(0, 90, 1920, 1080);
        assert!(matches!(comp.validate(), Err(CompositionError::InvalidFps)));
    }

    #[test]
    fn validate_zero_dimension_returns_invalid_dimensions() {
        let comp = VideoComposition::new(30, 90, 0, 1080);
        assert!(matches!(comp.validate(), Err(CompositionError::InvalidDimensions)));
        let comp2 = VideoComposition::new(30, 90, 1920, 0);
        assert!(matches!(comp2.validate(), Err(CompositionError::InvalidDimensions)));
    }

    #[test]
    fn validate_scene_past_end_returns_scene_out_of_range() {
        let mut comp = VideoComposition::new(30, 90, 1920, 1080);
        comp.add_scene(SceneEntry::new(80, 20, "a")); // end_frame = 100 > 90
        assert!(matches!(
            comp.validate(),
            Err(CompositionError::SceneOutOfRange { end_frame: 100, duration_frames: 90 })
        ));
    }
}
