/// Horizontal alignment for laid-out text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Visual style parameters used when measuring and laying out text.
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub align: TextAlign,
    pub bold: bool,
    pub italic: bool,
}

impl TextStyle {
    /// Create a style with the given `font_size` and `line_height`.
    /// Alignment defaults to `Left`; bold and italic default to `false`.
    pub fn new(font_size: f32, line_height: f32) -> Self {
        Self {
            font_size,
            line_height,
            align: TextAlign::Left,
            bold: false,
            italic: false,
        }
    }
}

/// A single shaped run of glyphs with aggregate metrics.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub glyphs: Vec<u32>,
    pub advance: f32,
    pub ascent: f32,
    pub descent: f32,
}

impl GlyphRun {
    /// Create an empty glyph run with zeroed metrics.
    pub fn new() -> Self {
        Self {
            glyphs: Vec::new(),
            advance: 0.0,
            ascent: 0.0,
            descent: 0.0,
        }
    }

    /// Append a glyph id and add `advance_delta` to the run's total advance.
    pub fn add_glyph(&mut self, id: u32, advance_delta: f32) {
        self.glyphs.push(id);
        self.advance += advance_delta;
    }

    /// Return the number of glyphs currently in the run.
    pub fn total_glyphs(&self) -> usize {
        self.glyphs.len()
    }
}

impl Default for GlyphRun {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub text-layout engine that approximates metrics without a real font.
pub struct TextLayoutEngine;

impl TextLayoutEngine {
    /// Create a new layout engine instance.
    pub fn new() -> Self {
        Self
    }

    /// Estimate the pixel width of a single line of `text` with the given style.
    ///
    /// Stub formula: `text.len() * font_size * 0.6`.
    pub fn measure_line(&self, text: &str, style: &TextStyle) -> f32 {
        text.len() as f32 * style.font_size * 0.6
    }

    /// Word-wrap `text` into lines that fit within `max_width` pixels.
    ///
    /// Words are split on ASCII spaces. A word that would push the current
    /// line past `max_width` is moved to the next line.
    pub fn layout_paragraph(
        &self,
        text: &str,
        style: &TextStyle,
        max_width: f32,
    ) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();

        for word in text.split(' ') {
            if word.is_empty() {
                continue;
            }
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };

            if self.measure_line(&candidate, style) <= max_width {
                current = candidate;
            } else {
                if !current.is_empty() {
                    lines.push(current);
                }
                current = word.to_owned();
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }

        lines
    }
}

impl Default for TextLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_style_new() {
        let s = TextStyle::new(14.0, 20.0);
        assert_eq!(s.font_size, 14.0);
        assert_eq!(s.line_height, 20.0);
        assert_eq!(s.align, TextAlign::Left);
        assert!(!s.bold);
        assert!(!s.italic);
    }

    #[test]
    fn glyph_run_add() {
        let mut run = GlyphRun::new();
        run.add_glyph(42, 10.0);
        run.add_glyph(99, 5.5);
        assert_eq!(run.glyphs, vec![42, 99]);
    }

    #[test]
    fn glyph_run_advance() {
        let mut run = GlyphRun::new();
        run.add_glyph(1, 8.0);
        run.add_glyph(2, 7.0);
        assert!((run.advance - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn measure_line() {
        let engine = TextLayoutEngine::new();
        let style = TextStyle::new(10.0, 14.0);
        // "hello" = 5 chars → 5 * 10.0 * 0.6 = 30.0
        let width = engine.measure_line("hello", &style);
        assert!((width - 30.0).abs() < 1e-4);
    }

    #[test]
    fn layout_paragraph_single() {
        let engine = TextLayoutEngine::new();
        let style = TextStyle::new(10.0, 14.0);
        // "hi" = 2 * 10 * 0.6 = 12 — fits in 200
        let lines = engine.layout_paragraph("hi", &style, 200.0);
        assert_eq!(lines, vec!["hi"]);
    }

    #[test]
    fn layout_paragraph_wraps() {
        let engine = TextLayoutEngine::new();
        let style = TextStyle::new(10.0, 14.0);
        // Each char costs 6px. "hello world" = 11+1 space = measure("hello world") = 66px
        // max_width=40 → "hello" (30) fits, "hello world" (66) does not → wrap
        let lines = engine.layout_paragraph("hello world foo", &style, 40.0);
        assert!(lines.len() > 1, "expected wrapping into multiple lines");
        assert_eq!(lines[0], "hello");
    }

    #[test]
    fn text_align_variants() {
        let variants = [
            TextAlign::Left,
            TextAlign::Center,
            TextAlign::Right,
            TextAlign::Justify,
        ];
        // All four variants are distinct and Debug-printable.
        for (i, v) in variants.iter().enumerate() {
            for (j, w) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(v, w);
                } else {
                    assert_ne!(v, w);
                }
            }
        }
    }

    #[test]
    fn bold_italic() {
        let mut s = TextStyle::new(12.0, 16.0);
        s.bold = true;
        s.italic = true;
        assert!(s.bold);
        assert!(s.italic);
    }
}
