/// Video timeline editing model — clips, tracks, overlap detection.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipKind {
    Video,
    Audio,
    Text,
    Image,
    Effect,
}

impl ClipKind {
    pub fn is_av(&self) -> bool {
        matches!(self, ClipKind::Video | ClipKind::Audio)
    }

    pub fn track_type(&self) -> &'static str {
        match self {
            ClipKind::Video => "video",
            ClipKind::Audio => "audio",
            ClipKind::Text => "text",
            ClipKind::Image => "image",
            ClipKind::Effect => "effect",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimelineClip {
    pub id: u64,
    pub kind: ClipKind,
    pub start_frame: u64,
    pub duration_frames: u64,
    pub track: u32,
}

impl TimelineClip {
    pub fn end_frame(&self) -> u64 {
        self.start_frame + self.duration_frames
    }

    pub fn overlaps(&self, other: &TimelineClip) -> bool {
        self.track == other.track
            && self.start_frame < other.end_frame()
            && self.end_frame() > other.start_frame
    }
}

#[derive(Debug, Clone)]
pub struct VideoTimeline {
    pub clips: Vec<TimelineClip>,
    pub frame_rate: f32,
}

impl VideoTimeline {
    pub fn new(frame_rate: f32) -> Self {
        Self { clips: Vec::new(), frame_rate }
    }

    pub fn add_clip(&mut self, c: TimelineClip) {
        self.clips.push(c);
    }

    pub fn duration_frames(&self) -> u64 {
        self.clips.iter().map(|c| c.end_frame()).max().unwrap_or(0)
    }

    pub fn clips_on_track(&self, track: u32) -> Vec<&TimelineClip> {
        self.clips.iter().filter(|c| c.track == track).collect()
    }

    pub fn has_overlaps(&self) -> bool {
        for i in 0..self.clips.len() {
            for j in (i + 1)..self.clips.len() {
                if self.clips[i].overlaps(&self.clips[j]) {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct ClipOverlap {
    pub clip_a_id: u64,
    pub clip_b_id: u64,
    pub overlap_frames: u64,
}

impl ClipOverlap {
    pub fn is_significant(&self) -> bool {
        self.overlap_frames > 0
    }
}

pub struct TimelineRenderer {
    pub timeline: VideoTimeline,
}

impl TimelineRenderer {
    pub fn new(tl: VideoTimeline) -> Self {
        Self { timeline: tl }
    }

    pub fn find_overlaps(&self) -> Vec<ClipOverlap> {
        let clips = &self.timeline.clips;
        let mut result = Vec::new();
        for i in 0..clips.len() {
            for j in (i + 1)..clips.len() {
                let a = &clips[i];
                let b = &clips[j];
                if a.track != b.track {
                    continue;
                }
                let overlap_start = a.start_frame.max(b.start_frame);
                let overlap_end = a.end_frame().min(b.end_frame());
                if overlap_end > overlap_start {
                    result.push(ClipOverlap {
                        clip_a_id: a.id,
                        clip_b_id: b.id,
                        overlap_frames: overlap_end - overlap_start,
                    });
                }
            }
        }
        result
    }

    pub fn track_count(&self) -> u32 {
        if self.timeline.clips.is_empty() {
            return 0;
        }
        let max_track = self.timeline.clips.iter().map(|c| c.track).max().unwrap_or(0);
        max_track + 1
    }
}

#[cfg(test)]
mod video_timeline_tests {
    use super::*;

    #[test]
    fn clip_kind_is_av() {
        assert!(ClipKind::Video.is_av());
        assert!(ClipKind::Audio.is_av());
        assert!(!ClipKind::Text.is_av());
        assert!(!ClipKind::Image.is_av());
        assert!(!ClipKind::Effect.is_av());
    }

    #[test]
    fn clip_kind_track_type() {
        assert_eq!(ClipKind::Video.track_type(), "video");
        assert_eq!(ClipKind::Audio.track_type(), "audio");
        assert_eq!(ClipKind::Text.track_type(), "text");
        assert_eq!(ClipKind::Image.track_type(), "image");
        assert_eq!(ClipKind::Effect.track_type(), "effect");
    }

    #[test]
    fn clip_end_frame() {
        let clip = TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 10, duration_frames: 30, track: 0 };
        assert_eq!(clip.end_frame(), 40);
    }

    #[test]
    fn clip_overlaps_true() {
        let a = TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 0, duration_frames: 20, track: 0 };
        let b = TimelineClip { id: 2, kind: ClipKind::Video, start_frame: 10, duration_frames: 20, track: 0 };
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn clip_overlaps_false_different_track() {
        let a = TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 0, duration_frames: 20, track: 0 };
        let b = TimelineClip { id: 2, kind: ClipKind::Video, start_frame: 10, duration_frames: 20, track: 1 };
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn timeline_duration_frames() {
        let mut tl = VideoTimeline::new(30.0);
        assert_eq!(tl.duration_frames(), 0, "empty timeline must return 0");
        tl.add_clip(TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 0, duration_frames: 100, track: 0 });
        tl.add_clip(TimelineClip { id: 2, kind: ClipKind::Audio, start_frame: 50, duration_frames: 80, track: 1 });
        assert_eq!(tl.duration_frames(), 130);
    }

    #[test]
    fn timeline_clips_on_track() {
        let mut tl = VideoTimeline::new(24.0);
        tl.add_clip(TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 0, duration_frames: 50, track: 0 });
        tl.add_clip(TimelineClip { id: 2, kind: ClipKind::Audio, start_frame: 0, duration_frames: 50, track: 1 });
        tl.add_clip(TimelineClip { id: 3, kind: ClipKind::Video, start_frame: 60, duration_frames: 30, track: 0 });
        let track0 = tl.clips_on_track(0);
        assert_eq!(track0.len(), 2);
        assert!(track0.iter().all(|c| c.track == 0));
        let track1 = tl.clips_on_track(1);
        assert_eq!(track1.len(), 1);
    }

    #[test]
    fn timeline_has_overlaps_true() {
        let mut tl = VideoTimeline::new(30.0);
        tl.add_clip(TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 0, duration_frames: 30, track: 0 });
        tl.add_clip(TimelineClip { id: 2, kind: ClipKind::Video, start_frame: 20, duration_frames: 30, track: 0 });
        assert!(tl.has_overlaps());
    }

    #[test]
    fn renderer_find_overlaps_count() {
        let mut tl = VideoTimeline::new(30.0);
        // two overlapping clips on track 0
        tl.add_clip(TimelineClip { id: 1, kind: ClipKind::Video, start_frame: 0, duration_frames: 40, track: 0 });
        tl.add_clip(TimelineClip { id: 2, kind: ClipKind::Video, start_frame: 30, duration_frames: 40, track: 0 });
        // non-overlapping clip on track 0 (starts at 80, after clip 2 ends at 70)
        tl.add_clip(TimelineClip { id: 3, kind: ClipKind::Video, start_frame: 80, duration_frames: 20, track: 0 });
        // clip on different track — no overlap counted
        tl.add_clip(TimelineClip { id: 4, kind: ClipKind::Audio, start_frame: 0, duration_frames: 40, track: 1 });
        let renderer = TimelineRenderer::new(tl);
        let overlaps = renderer.find_overlaps();
        assert_eq!(overlaps.len(), 1, "expected exactly 1 overlap pair");
        assert!(overlaps[0].is_significant());
        assert_eq!(overlaps[0].overlap_frames, 10);
    }
}
