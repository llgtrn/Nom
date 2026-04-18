/// Axis-aligned bounding rectangle used for clip regions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ClipRect {
    /// Returns the right edge (x + width).
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Returns the bottom edge (y + height).
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// AABB intersection. Returns `None` if the rectangles do not overlap.
    pub fn intersect(&self, other: &ClipRect) -> Option<ClipRect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= x || bottom <= y {
            return None;
        }
        Some(ClipRect {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }

    /// Returns `true` if the point (px, py) lies inside this rectangle.
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }
}

/// A push-down stack of clip rectangles.
#[derive(Debug, Default)]
pub struct ClipStack {
    pub rects: Vec<ClipRect>,
}

impl ClipStack {
    /// Pushes a clip rectangle onto the stack.
    pub fn push(&mut self, r: ClipRect) {
        self.rects.push(r);
    }

    /// Pops the top clip rectangle from the stack.
    pub fn pop(&mut self) -> Option<ClipRect> {
        self.rects.pop()
    }

    /// Returns a reference to the topmost clip rectangle, or `None` if empty.
    pub fn current(&self) -> Option<&ClipRect> {
        self.rects.last()
    }

    /// Returns the number of rectangles currently on the stack.
    pub fn depth(&self) -> usize {
        self.rects.len()
    }
}

/// Manages viewport-relative clip regions via a push-down stack.
pub struct ViewportClipper {
    pub viewport: ClipRect,
    pub stack: ClipStack,
}

impl ViewportClipper {
    /// Creates a new `ViewportClipper` with the given viewport and an empty stack.
    pub fn new(viewport: ClipRect) -> Self {
        Self {
            viewport,
            stack: ClipStack::default(),
        }
    }

    /// Intersects `r` with the viewport and, if the intersection is non-empty,
    /// pushes it onto the stack.
    pub fn push_clip(&mut self, r: ClipRect) {
        if let Some(clipped) = self.viewport.intersect(&r) {
            self.stack.push(clipped);
        }
    }

    /// Pops the top clip rectangle from the stack.
    pub fn pop_clip(&mut self) -> Option<ClipRect> {
        self.stack.pop()
    }

    /// Returns the active clip rectangle: the top of the stack, or the full
    /// viewport if the stack is empty.
    pub fn active_clip(&self) -> Option<&ClipRect> {
        self.stack.current().or(Some(&self.viewport))
    }
}

/// Result of intersecting an element's bounds against an active clip rectangle.
#[derive(Debug, Clone)]
pub struct ClipResult {
    /// `true` if the element was clipped (its bounds differ from the intersection).
    pub clipped: bool,
    /// The visible portion of the element, or `None` if fully clipped.
    pub visible_rect: Option<ClipRect>,
}

impl ClipResult {
    /// Builds a `ClipResult` by intersecting `element` with `clip`.
    ///
    /// `clipped` is `true` when the intersection result is not identical to the
    /// element bounds (i.e. something was cut off or the element is fully hidden).
    pub fn from_intersect(element: &ClipRect, clip: &ClipRect) -> ClipResult {
        let intersection = element.intersect(clip);
        let clipped = intersection.as_ref() != Some(element);
        ClipResult {
            clipped,
            visible_rect: intersection,
        }
    }
}

/// Accumulates a batch of `ClipResult` values for bulk visibility queries.
#[derive(Debug, Default)]
pub struct ClipBatch {
    pub results: Vec<ClipResult>,
}

impl ClipBatch {
    /// Appends a `ClipResult` to the batch.
    pub fn add(&mut self, r: ClipResult) {
        self.results.push(r);
    }

    /// Returns the number of results with a non-`None` visible rect.
    pub fn visible_count(&self) -> usize {
        self.results.iter().filter(|r| r.visible_rect.is_some()).count()
    }

    /// Returns the number of results with a `None` visible rect (fully clipped).
    pub fn fully_clipped_count(&self) -> usize {
        self.results.iter().filter(|r| r.visible_rect.is_none()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> ClipRect {
        ClipRect { x, y, width: w, height: h }
    }

    #[test]
    fn clip_rect_right_bottom() {
        let r = rect(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
    }

    #[test]
    fn intersect_overlapping() {
        let a = rect(0.0, 0.0, 100.0, 100.0);
        let b = rect(50.0, 50.0, 100.0, 100.0);
        let result = a.intersect(&b).expect("expected intersection");
        assert_eq!(result.x, 50.0);
        assert_eq!(result.y, 50.0);
        assert_eq!(result.width, 50.0);
        assert_eq!(result.height, 50.0);
    }

    #[test]
    fn intersect_no_overlap() {
        let a = rect(0.0, 0.0, 50.0, 50.0);
        let b = rect(100.0, 100.0, 50.0, 50.0);
        assert!(a.intersect(&b).is_none());
    }

    #[test]
    fn contains_point() {
        let r = rect(10.0, 10.0, 80.0, 60.0);
        assert!(r.contains_point(50.0, 40.0));
        assert!(!r.contains_point(5.0, 40.0));
        assert!(!r.contains_point(50.0, 80.0));
    }

    #[test]
    fn stack_push_pop_depth() {
        let mut stack = ClipStack::default();
        assert_eq!(stack.depth(), 0);
        assert!(stack.current().is_none());

        stack.push(rect(0.0, 0.0, 100.0, 100.0));
        assert_eq!(stack.depth(), 1);

        stack.push(rect(10.0, 10.0, 50.0, 50.0));
        assert_eq!(stack.depth(), 2);

        let popped = stack.pop().expect("expected a rect");
        assert_eq!(popped.x, 10.0);
        assert_eq!(stack.depth(), 1);

        let _ = stack.pop();
        assert_eq!(stack.depth(), 0);
        assert!(stack.pop().is_none());
    }

    #[test]
    fn clipper_active_clip_falls_back_to_viewport() {
        let vp = rect(0.0, 0.0, 800.0, 600.0);
        let clipper = ViewportClipper::new(vp);
        let active = clipper.active_clip().expect("expected Some");
        assert_eq!(*active, vp);
    }

    #[test]
    fn clipper_push_clip_intersects_with_viewport() {
        let vp = rect(0.0, 0.0, 800.0, 600.0);
        let mut clipper = ViewportClipper::new(vp);

        // Partially outside the viewport on the right/bottom.
        clipper.push_clip(rect(700.0, 500.0, 200.0, 200.0));
        let active = clipper.active_clip().expect("expected Some");
        assert_eq!(active.x, 700.0);
        assert_eq!(active.y, 500.0);
        assert_eq!(active.width, 100.0);   // clamped to viewport right (800)
        assert_eq!(active.height, 100.0);  // clamped to viewport bottom (600)
    }

    #[test]
    fn clip_result_from_intersect_visible() {
        let element = rect(20.0, 20.0, 60.0, 60.0);
        let clip = rect(0.0, 0.0, 100.0, 100.0);
        let result = ClipResult::from_intersect(&element, &clip);
        // Element is entirely inside clip — not clipped.
        assert!(!result.clipped);
        assert!(result.visible_rect.is_some());
        assert_eq!(result.visible_rect.unwrap(), element);
    }

    #[test]
    fn batch_visible_count() {
        let mut batch = ClipBatch::default();

        let full = rect(0.0, 0.0, 100.0, 100.0);
        let clip = rect(0.0, 0.0, 50.0, 50.0);

        // Element entirely inside clip → visible.
        batch.add(ClipResult::from_intersect(&rect(10.0, 10.0, 20.0, 20.0), &clip));
        // Element entirely outside clip → fully clipped.
        batch.add(ClipResult::from_intersect(&rect(60.0, 60.0, 20.0, 20.0), &clip));
        // Another visible element.
        batch.add(ClipResult::from_intersect(&rect(5.0, 5.0, 10.0, 10.0), &full));

        assert_eq!(batch.visible_count(), 2);
        assert_eq!(batch.fully_clipped_count(), 1);
    }
}
