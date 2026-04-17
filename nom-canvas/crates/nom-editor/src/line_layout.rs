#![deny(unsafe_code)]

/// One glyph run in a laid-out line
#[derive(Clone, Debug)]
pub struct LayoutRun {
    pub start: usize, // char offset in line
    pub end: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_id: u32,
    pub font_size: f32,
}

/// A fully laid-out display line ready for rendering
#[derive(Clone, Debug)]
pub struct LineLayout {
    pub len: usize,  // char length
    pub width: f32,  // total visual width
    pub height: f32, // line height
    pub runs: Vec<LayoutRun>,
    pub ascent: f32,
    pub descent: f32,
}

impl LineLayout {
    pub fn new(len: usize, width: f32, height: f32) -> Self {
        Self {
            len,
            width,
            height,
            runs: Vec::new(),
            ascent: height * 0.8,
            descent: height * 0.2,
        }
    }
    pub fn hit_test_x(&self, x: f32) -> usize {
        for run in &self.runs {
            if x >= run.x && x < run.x + run.width {
                let frac = (x - run.x) / run.width;
                return run.start + ((run.end - run.start) as f32 * frac) as usize;
            }
        }
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_layout_hit_test() {
        let mut ll = LineLayout::new(10, 100.0, 20.0);
        ll.runs.push(LayoutRun {
            start: 0,
            end: 10,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
            font_id: 0,
            font_size: 14.0,
        });
        assert_eq!(ll.hit_test_x(50.0), 5);
        assert_eq!(ll.hit_test_x(0.0), 0);
    }
}
