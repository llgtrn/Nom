#![deny(unsafe_code)]
use serde::{Deserialize, Serialize};
use crate::block_model::NomtuRef;

// Hsla-compatible color stored as [h,s,l,a] f32
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrokeColor { pub h: f32, pub s: f32, pub l: f32, pub a: f32 }

impl StrokeColor {
    pub fn black() -> Self { Self { h: 0.0, s: 0.0, l: 0.0, a: 1.0 } }
    pub fn white() -> Self { Self { h: 0.0, s: 0.0, l: 1.0, a: 1.0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<[f32; 2]>,
    pub pressure: Vec<f32>,
    pub color: StrokeColor,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: StrokeColor, width: f32) -> Self {
        Self { points: Vec::new(), pressure: Vec::new(), color, width }
    }
    pub fn add_point(&mut self, pt: [f32; 2], pressure: f32) {
        self.points.push(pt);
        self.pressure.push(pressure);
    }
    pub fn bounding_box(&self) -> Option<([f32; 2], [f32; 2])> {
        if self.points.is_empty() { return None; }
        let min_x = self.points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let min_y = self.points.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let max_x = self.points.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
        let max_y = self.points.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
        Some(([min_x, min_y], [max_x, max_y]))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawingBlock {
    pub entity: NomtuRef,
    pub strokes: Vec<Stroke>,
}

impl DrawingBlock {
    pub fn new(entity: NomtuRef) -> Self {
        Self { entity, strokes: Vec::new() }
    }
    pub fn add_stroke(&mut self, stroke: Stroke) {
        self.strokes.push(stroke);
    }
    pub fn clear(&mut self) {
        self.strokes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stroke_bounding_box() {
        let mut s = Stroke::new(StrokeColor::black(), 2.0);
        s.add_point([0.0, 0.0], 1.0);
        s.add_point([100.0, 50.0], 1.0);
        let bb = s.bounding_box().unwrap();
        assert_eq!(bb.0, [0.0, 0.0]);
        assert_eq!(bb.1, [100.0, 50.0]);
    }
}
