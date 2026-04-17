//! Editor display pipeline: Buffer → InlayMap → FoldMap → TabMap → WrapMap → Display.
//!
//! Minimal skeleton: each stage is a thin transform over a buffer-offset range,
//! producing a display-offset range. Full SumTree<Transform> deferred;
//! data-flow interface matches Zed so upgrade is in-place.
#![deny(unsafe_code)]

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferOffset(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayOffset(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct FoldRange {
    pub buffer_range: Range<usize>,
    pub placeholder_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlayEntry {
    pub id: u64,
    pub buffer_offset: usize,
    pub text: String,
}

#[derive(Default)]
pub struct DisplayMap {
    /// Byte offsets that are hidden (folded ranges collapsed to placeholders).
    pub folds: Vec<FoldRange>,
    /// Inlay hints / virtual text at specific offsets (does NOT shift buffer offsets).
    pub inlays: Vec<InlayEntry>,
    /// Tab width for expansion to display columns.
    pub tab_width: u32,
    /// Soft-wrap column; 0 disables.
    pub soft_wrap_col: u32,
}

impl DisplayMap {
    pub fn new(tab_width: u32) -> Self {
        Self {
            folds: Vec::new(),
            inlays: Vec::new(),
            tab_width,
            soft_wrap_col: 0,
        }
    }

    pub fn add_fold(&mut self, range: Range<usize>, placeholder: impl Into<String>) {
        self.folds.push(FoldRange {
            buffer_range: range,
            placeholder_text: placeholder.into(),
        });
        // Keep folds sorted by start offset for deterministic iteration.
        self.folds.sort_by_key(|f| f.buffer_range.start);
    }

    pub fn remove_folds_overlapping(&mut self, range: Range<usize>) {
        self.folds.retain(|f| {
            // retain only folds that do NOT overlap the given range
            f.buffer_range.end <= range.start || f.buffer_range.start >= range.end
        });
    }

    pub fn add_inlay(&mut self, id: u64, offset: usize, text: impl Into<String>) {
        self.inlays.push(InlayEntry {
            id,
            buffer_offset: offset,
            text: text.into(),
        });
        self.inlays.sort_by_key(|i| i.buffer_offset);
    }

    pub fn remove_inlay(&mut self, id: u64) -> bool {
        if let Some(pos) = self.inlays.iter().position(|i| i.id == id) {
            self.inlays.remove(pos);
            true
        } else {
            false
        }
    }

    /// Map a buffer offset to a display offset given current folds + inlays.
    /// Naive implementation: iterate folds/inlays up to the offset, add inlay
    /// lengths, subtract fold spans.
    pub fn to_display(&self, buf: BufferOffset) -> DisplayOffset {
        let pos = buf.0;
        let mut display = pos as i64;

        // Subtract folded bytes that lie before pos.
        for fold in &self.folds {
            if fold.buffer_range.end <= pos {
                // Fold is entirely before pos: subtract its hidden span, add placeholder len.
                let hidden = (fold.buffer_range.end - fold.buffer_range.start) as i64;
                let shown = fold.placeholder_text.len() as i64;
                display = display - hidden + shown;
            } else if fold.buffer_range.start < pos {
                // Fold starts before pos but extends into/past pos: pos is inside fold,
                // clamp to start of placeholder.
                let hidden = (pos - fold.buffer_range.start) as i64;
                display -= hidden;
                // Don't add placeholder yet — we're inside it.
            }
        }

        // Add inlay lengths for inlays that are strictly before pos.
        for inlay in &self.inlays {
            if inlay.buffer_offset < pos {
                display += inlay.text.len() as i64;
            }
        }

        DisplayOffset(display.max(0) as usize)
    }

    /// Inverse mapping. Saturates when target falls inside a fold or inlay.
    pub fn to_buffer(&self, disp: DisplayOffset) -> BufferOffset {
        // Walk buffer offsets from 0, accumulating display offset, until we reach
        // the target display offset.
        let target = disp.0;
        let mut display_pos: i64 = 0;
        let mut buf_pos: usize = 0;

        // Collect events sorted by buffer offset.
        #[derive(PartialEq, Eq, PartialOrd, Ord)]
        enum Event {
            FoldStart,
            FoldEnd,
            Inlay,
        }

        struct Evt {
            buf_offset: usize,
            kind: Event,
            data: usize, // fold index or inlay len
        }

        let mut events: Vec<Evt> = Vec::new();
        for (i, fold) in self.folds.iter().enumerate() {
            events.push(Evt { buf_offset: fold.buffer_range.start, kind: Event::FoldStart, data: i });
            events.push(Evt { buf_offset: fold.buffer_range.end, kind: Event::FoldEnd, data: i });
        }
        for inlay in &self.inlays {
            events.push(Evt { buf_offset: inlay.buffer_offset, kind: Event::Inlay, data: inlay.text.len() });
        }
        events.sort_by_key(|e| (e.buf_offset, match e.kind { Event::FoldStart => 0, Event::Inlay => 1, Event::FoldEnd => 2 }));

        let mut skip_until: Option<usize> = None; // skip buffer bytes inside a fold

        for evt in &events {
            // Advance buf_pos → evt.buf_offset, updating display_pos
            if let Some(skip_end) = skip_until {
                if evt.buf_offset <= skip_end {
                    // still inside fold
                    match evt.kind {
                        Event::FoldEnd => {
                            skip_until = None;
                            buf_pos = evt.buf_offset;
                        }
                        _ => continue,
                    }
                    continue;
                }
            }

            // Advance through un-folded bytes
            let advance = evt.buf_offset.saturating_sub(buf_pos);
            if display_pos as usize + advance >= target {
                // Target is in this plain-text stretch
                let remaining = target - display_pos as usize;
                return BufferOffset(buf_pos + remaining);
            }
            display_pos += advance as i64;
            buf_pos = evt.buf_offset;

            match evt.kind {
                Event::FoldStart => {
                    let fold = &self.folds[evt.data];
                    let placeholder_len = fold.placeholder_text.len();
                    if display_pos as usize + placeholder_len >= target {
                        // Target is inside the placeholder; saturate to fold start
                        return BufferOffset(fold.buffer_range.start);
                    }
                    display_pos += placeholder_len as i64;
                    skip_until = Some(fold.buffer_range.end);
                    buf_pos = fold.buffer_range.end;
                }
                Event::FoldEnd => {
                    skip_until = None;
                }
                Event::Inlay => {
                    if display_pos as usize + evt.data >= target {
                        // Target is inside inlay; saturate to inlay anchor
                        return BufferOffset(evt.buf_offset);
                    }
                    display_pos += evt.data as i64;
                }
            }
        }

        // Remaining plain text after all events
        let remaining = target - display_pos as usize;
        BufferOffset(buf_pos + remaining)
    }

    /// Expand a tab character at `column` to the next tab stop.
    pub fn expand_tab(&self, column: u32) -> u32 {
        if self.tab_width == 0 {
            column + 1
        } else {
            column + (self.tab_width - column % self.tab_width)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_tab_width_set() {
        let map = DisplayMap::new(4);
        assert!(map.folds.is_empty());
        assert!(map.inlays.is_empty());
        assert_eq!(map.tab_width, 4);
        assert_eq!(map.soft_wrap_col, 0);
    }

    #[test]
    fn to_display_no_folds_or_inlays_is_identity() {
        let map = DisplayMap::new(4);
        assert_eq!(map.to_display(BufferOffset(0)), DisplayOffset(0));
        assert_eq!(map.to_display(BufferOffset(10)), DisplayOffset(10));
        assert_eq!(map.to_display(BufferOffset(100)), DisplayOffset(100));
    }

    #[test]
    fn add_fold_shifts_display_past_fold() {
        let mut map = DisplayMap::new(4);
        // fold bytes 5..15 (10 bytes) with placeholder "..."  (3 chars)
        map.add_fold(5..15, "...");
        // buffer offset 20 → display: 20 - 10 + 3 = 13
        assert_eq!(map.to_display(BufferOffset(20)), DisplayOffset(13));
    }

    #[test]
    fn add_inlay_shifts_display_after_inlay() {
        let mut map = DisplayMap::new(4);
        // inlay at offset 5, text "HINT" (4 chars)
        map.add_inlay(1, 5, "HINT");
        // buffer offset 10 → display: 10 + 4 = 14
        assert_eq!(map.to_display(BufferOffset(10)), DisplayOffset(14));
        // buffer offset 3 (before inlay) → identity
        assert_eq!(map.to_display(BufferOffset(3)), DisplayOffset(3));
    }

    #[test]
    fn remove_inlay_true_on_hit_false_on_miss() {
        let mut map = DisplayMap::new(4);
        map.add_inlay(42, 10, "x");
        assert!(map.remove_inlay(42));
        assert!(!map.remove_inlay(42));
    }

    #[test]
    fn remove_folds_overlapping_removes_matching_only() {
        let mut map = DisplayMap::new(4);
        map.add_fold(0..10, "A");
        map.add_fold(20..30, "B");
        map.add_fold(5..25, "C"); // overlaps both
        map.remove_folds_overlapping(8..22);
        // fold A (0..10) overlaps 8..22 → removed
        // fold B (20..30) overlaps 8..22 → removed
        // fold C (5..25) overlaps 8..22 → removed
        assert!(map.folds.is_empty());
    }

    #[test]
    fn remove_folds_overlapping_keeps_non_overlapping() {
        let mut map = DisplayMap::new(4);
        map.add_fold(0..5, "A");
        map.add_fold(20..30, "B");
        map.remove_folds_overlapping(8..18);
        assert_eq!(map.folds.len(), 2);
    }

    #[test]
    fn expand_tab_at_col_0_with_tab_width_4() {
        let map = DisplayMap::new(4);
        assert_eq!(map.expand_tab(0), 4);
    }

    #[test]
    fn expand_tab_at_col_3_with_tab_width_4() {
        let map = DisplayMap::new(4);
        assert_eq!(map.expand_tab(3), 4);
    }

    #[test]
    fn expand_tab_at_col_5_with_tab_width_4() {
        let map = DisplayMap::new(4);
        assert_eq!(map.expand_tab(5), 8);
    }

    #[test]
    fn to_buffer_roundtrip_no_transforms() {
        let map = DisplayMap::new(4);
        for offset in [0usize, 1, 5, 10, 50] {
            let buf = BufferOffset(offset);
            let disp = map.to_display(buf);
            let back = map.to_buffer(disp);
            assert_eq!(back, buf, "roundtrip failed for offset {offset}");
        }
    }
}
