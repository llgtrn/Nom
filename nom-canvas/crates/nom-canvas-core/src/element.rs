//! Canvas element — the atomic unit of the infinite canvas.
//!
//! An `Element` is a self-contained record: geometry, styling, CRDT versioning,
//! and the `Shape` variant that drives rendering. Every mutation goes through
//! [`crate::mutation`] so the version counters stay consistent.

use nom_gpui::{Bounds, Pixels, LinearRgba};

use crate::shapes::Shape;

/// Unique identifier for a canvas element (content-addressed or server-assigned).
pub type ElementId = u64;

/// Identifier for a frame element that other elements may belong to.
pub type FrameId = u64;

/// Identifier for a logical group of elements (selection group, named layer, etc.).
pub type GroupId = u64;

/// A single canvas element — geometry + style + CRDT bookkeeping + shape.
///
/// Designed after the element record used by Excalidraw's element model
/// (`packages/element/src/types.ts`) but expressed entirely in Nom-native
/// vocabulary with zero foreign identifiers.
#[derive(Clone, Debug)]
pub struct Element {
    /// Stable unique identifier for this element.
    pub id: ElementId,

    /// Axis-aligned bounding box in canvas (logical pixel) space.
    pub bounds: Bounds<Pixels>,

    /// Rotation angle in radians (clockwise from the positive X axis).
    pub angle: f32,

    /// Stroke colour; `None` means the shape has no visible border.
    pub stroke: Option<LinearRgba>,

    /// Fill colour; `None` means the shape interior is transparent.
    pub fill: Option<LinearRgba>,

    /// Opacity multiplier applied on top of stroke/fill alpha (0.0 – 1.0).
    pub opacity: f32,

    /// When `true` the element cannot be selected or moved by the user.
    pub locked: bool,

    /// Logical group this element belongs to, if any.
    pub group_id: Option<GroupId>,

    /// Frame element that clips/contains this element, if any.
    pub frame_id: Option<FrameId>,

    /// Paint order relative to sibling elements (lower = further back).
    pub z_index: i32,

    /// Soft-delete flag — deleted elements are retained for CRDT merge resolution.
    pub is_deleted: bool,

    /// Other elements (arrows, connectors) whose lifecycle is bound to this element.
    pub bound_elements: Vec<ElementId>,

    /// Monotonically increasing edit counter; incremented on every mutation.
    pub version: u32,

    /// Random nonce mixed with `version` for CRDT last-write-wins conflict resolution.
    ///
    /// MVP strategy: derived from `(id, version)` via a cheap bit-mix.
    /// TODO(CRDT): replace with a true random u32 once a PRNG is available in this layer.
    pub version_nonce: u32,

    /// The visual shape this element renders as.
    pub shape: Shape,
}

impl Element {
    /// Construct a new element with sensible defaults.
    ///
    /// `version` starts at `1` so that the first [`crate::mutation::mutate`] call
    /// advances it to `2`, making it easy to detect "never mutated" elements.
    pub fn new(id: ElementId, shape: Shape, bounds: Bounds<Pixels>) -> Self {
        let version_nonce = version_nonce_for(id, 1);
        Self {
            id,
            bounds,
            angle: 0.0,
            stroke: None,
            fill: None,
            opacity: 1.0,
            locked: false,
            group_id: None,
            frame_id: None,
            z_index: 0,
            is_deleted: false,
            bound_elements: Vec::new(),
            version: 1,
            version_nonce,
            shape,
        }
    }
}

/// Derive a deterministic nonce from the element id and current version.
///
/// Uses a cheap Murmur3-style finalizer so that consecutive `(id, version)`
/// pairs produce well-distributed nonces without requiring a global PRNG.
pub(crate) fn version_nonce_for(id: ElementId, version: u32) -> u32 {
    // Pack id and version into a single u64 seed.
    let seed = id.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(version as u64);
    // Murmur3-style finalizer (64→32 bit mix).
    let h = seed ^ (seed >> 33);
    let h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    let h = h ^ (h >> 33);
    let h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    let h = h ^ (h >> 33);
    h as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutation::{mutate, replace_with};
    use crate::shapes::{Rectangle, Shape};
    use nom_gpui::{Bounds, Pixels, Point, Size};

    fn unit_bounds() -> Bounds<Pixels> {
        Bounds::new(
            Point::new(Pixels(0.0), Pixels(0.0)),
            Size::new(Pixels(100.0), Pixels(100.0)),
        )
    }

    fn rect_element(id: ElementId) -> Element {
        Element::new(id, Shape::Rectangle(Rectangle {}), unit_bounds())
    }

    #[test]
    fn new_element_has_version_1() {
        let e = rect_element(1);
        assert_eq!(e.version, 1, "freshly created element must start at version 1");
    }

    #[test]
    fn mutate_bumps_version() {
        let mut e = rect_element(2);
        assert_eq!(e.version, 1);
        mutate(&mut e, |el| el.opacity = 0.5);
        assert_eq!(e.version, 2, "mutate must increment version by 1");
        assert!((e.opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn replace_with_does_not_mutate_original() {
        let original = rect_element(3);
        let updated = replace_with(&original, |el| el.locked = true);

        // Original is unchanged.
        assert_eq!(original.version, 1);
        assert!(!original.locked);

        // Updated copy has advanced version and new field.
        assert_eq!(updated.version, 2);
        assert!(updated.locked);
    }
}
