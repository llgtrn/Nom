//! Extended hit-zone model with named zones and priority.
//!
//! Provides [`HitZoneKind`], [`HitZone`], [`HitZoneMap`], [`HitTestResult`],
//! and [`ZoneHitTester`] for pointer-interaction hit detection on canvas regions.

/// The kind of interaction a hit zone handles.
#[derive(Debug, Clone, PartialEq)]
pub enum HitZoneKind {
    /// A clickable region.
    Click,
    /// A hover-sensitive region.
    Hover,
    /// A draggable region.
    Drag,
    /// A resize handle region.
    Resize,
    /// A scrollable region.
    Scroll,
}

impl HitZoneKind {
    /// Returns `true` for pointer-active kinds: Click, Hover, Drag.
    pub fn is_pointer(&self) -> bool {
        matches!(self, HitZoneKind::Click | HitZoneKind::Hover | HitZoneKind::Drag)
    }

    /// Returns the CSS cursor name for this zone kind.
    pub fn cursor_name(&self) -> &'static str {
        match self {
            HitZoneKind::Click => "pointer",
            HitZoneKind::Hover => "default",
            HitZoneKind::Drag => "grab",
            HitZoneKind::Resize => "ew-resize",
            HitZoneKind::Scroll => "ns-resize",
        }
    }
}

/// A rectangular hit zone with an id, kind, position, size, and priority.
pub struct HitZone {
    /// Unique identifier for this zone.
    pub id: u64,
    /// The interaction kind this zone handles.
    pub kind: HitZoneKind,
    /// Left edge of the zone in canvas coordinates.
    pub x: f32,
    /// Top edge of the zone in canvas coordinates.
    pub y: f32,
    /// Width of the zone.
    pub width: f32,
    /// Height of the zone.
    pub height: f32,
    /// Higher priority zones are returned first from [`HitZoneMap::zones_at`].
    pub priority: i32,
}

impl HitZone {
    /// Returns `true` if the point `(px, py)` falls within this zone's AABB.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x
            && px <= self.x + self.width
            && py >= self.y
            && py <= self.y + self.height
    }

    /// Returns the area of this zone (`width * height`).
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

/// A collection of [`HitZone`]s with spatial query helpers.
#[derive(Default)]
pub struct HitZoneMap {
    /// The zones in this map.
    pub zones: Vec<HitZone>,
}

impl HitZoneMap {
    /// Creates an empty [`HitZoneMap`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a zone to the map.
    pub fn add(&mut self, z: HitZone) {
        self.zones.push(z);
    }

    /// Returns all zones that contain `(px, py)`, sorted by priority descending.
    pub fn zones_at(&self, px: f32, py: f32) -> Vec<&HitZone> {
        let mut hits: Vec<&HitZone> = self.zones.iter().filter(|z| z.contains(px, py)).collect();
        hits.sort_by(|a, b| b.priority.cmp(&a.priority));
        hits
    }

    /// Returns the highest-priority zone containing `(px, py)`, if any.
    pub fn top_zone(&self, px: f32, py: f32) -> Option<&HitZone> {
        self.zones_at(px, py).into_iter().next()
    }
}

/// The result of a hit test against a [`ZoneHitTester`].
pub struct HitTestResult {
    /// Whether a zone was hit.
    pub hit: bool,
    /// The id of the hit zone, if any.
    pub zone_id: Option<u64>,
    /// The kind of the hit zone, if any.
    pub kind: Option<HitZoneKind>,
}

impl HitTestResult {
    /// Constructs a hit result from a matched [`HitZone`].
    pub fn from_zone(z: &HitZone) -> HitTestResult {
        HitTestResult {
            hit: true,
            zone_id: Some(z.id),
            kind: Some(z.kind.clone()),
        }
    }

    /// Constructs a miss result (no zone hit).
    pub fn miss() -> HitTestResult {
        HitTestResult {
            hit: false,
            zone_id: None,
            kind: None,
        }
    }
}

/// Runs point-in-zone hit tests against a [`HitZoneMap`].
pub struct ZoneHitTester {
    /// The map of zones to test against.
    pub map: HitZoneMap,
}

impl ZoneHitTester {
    /// Creates a new tester wrapping the given map.
    pub fn new(map: HitZoneMap) -> Self {
        Self { map }
    }

    /// Tests the point `(px, py)` and returns a [`HitTestResult`].
    ///
    /// Returns `from_zone` for the top priority zone, or `miss()` if no zone
    /// contains the point.
    pub fn test(&self, px: f32, py: f32) -> HitTestResult {
        match self.map.top_zone(px, py) {
            Some(z) => HitTestResult::from_zone(z),
            None => HitTestResult::miss(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zone(id: u64, kind: HitZoneKind, x: f32, y: f32, w: f32, h: f32, priority: i32) -> HitZone {
        HitZone { id, kind, x, y, width: w, height: h, priority }
    }

    #[test]
    fn kind_is_pointer() {
        assert!(HitZoneKind::Click.is_pointer());
        assert!(HitZoneKind::Hover.is_pointer());
        assert!(HitZoneKind::Drag.is_pointer());
        assert!(!HitZoneKind::Resize.is_pointer());
        assert!(!HitZoneKind::Scroll.is_pointer());
    }

    #[test]
    fn kind_cursor_name_resize() {
        assert_eq!(HitZoneKind::Resize.cursor_name(), "ew-resize");
        assert_eq!(HitZoneKind::Scroll.cursor_name(), "ns-resize");
        assert_eq!(HitZoneKind::Click.cursor_name(), "pointer");
        assert_eq!(HitZoneKind::Hover.cursor_name(), "default");
        assert_eq!(HitZoneKind::Drag.cursor_name(), "grab");
    }

    #[test]
    fn zone_contains_true() {
        let z = make_zone(1, HitZoneKind::Click, 10.0, 20.0, 100.0, 50.0, 0);
        assert!(z.contains(10.0, 20.0), "corner should be inside");
        assert!(z.contains(60.0, 45.0), "centre should be inside");
        assert!(z.contains(110.0, 70.0), "far corner should be inside");
    }

    #[test]
    fn zone_contains_false() {
        let z = make_zone(2, HitZoneKind::Hover, 10.0, 20.0, 100.0, 50.0, 0);
        assert!(!z.contains(9.9, 45.0), "left of zone");
        assert!(!z.contains(60.0, 19.9), "above zone");
        assert!(!z.contains(110.1, 45.0), "right of zone");
        assert!(!z.contains(60.0, 70.1), "below zone");
    }

    #[test]
    fn zone_area() {
        let z = make_zone(3, HitZoneKind::Drag, 0.0, 0.0, 40.0, 25.0, 0);
        assert!((z.area() - 1000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn map_zones_at_sorted_by_priority() {
        let mut map = HitZoneMap::new();
        map.add(make_zone(10, HitZoneKind::Click, 0.0, 0.0, 100.0, 100.0, 1));
        map.add(make_zone(20, HitZoneKind::Drag,  0.0, 0.0, 100.0, 100.0, 5));
        map.add(make_zone(30, HitZoneKind::Hover, 0.0, 0.0, 100.0, 100.0, 3));

        let hits = map.zones_at(50.0, 50.0);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].id, 20); // priority 5 first
        assert_eq!(hits[1].id, 30); // priority 3 second
        assert_eq!(hits[2].id, 10); // priority 1 last
    }

    #[test]
    fn map_top_zone_found() {
        let mut map = HitZoneMap::new();
        map.add(make_zone(1, HitZoneKind::Click, 0.0, 0.0, 50.0, 50.0, 2));
        map.add(make_zone(2, HitZoneKind::Resize, 0.0, 0.0, 50.0, 50.0, 10));

        let top = map.top_zone(25.0, 25.0).expect("should find a zone");
        assert_eq!(top.id, 2); // highest priority
        assert!(map.top_zone(200.0, 200.0).is_none());
    }

    #[test]
    fn hit_test_result_from_zone() {
        let z = make_zone(42, HitZoneKind::Drag, 0.0, 0.0, 10.0, 10.0, 0);
        let r = HitTestResult::from_zone(&z);
        assert!(r.hit);
        assert_eq!(r.zone_id, Some(42));
        assert_eq!(r.kind, Some(HitZoneKind::Drag));
    }

    #[test]
    fn hit_test_result_miss() {
        let r = HitTestResult::miss();
        assert!(!r.hit);
        assert!(r.zone_id.is_none());
        assert!(r.kind.is_none());
    }

    #[test]
    fn tester_test_hit() {
        let mut map = HitZoneMap::new();
        map.add(make_zone(7, HitZoneKind::Click, 0.0, 0.0, 80.0, 80.0, 1));
        let tester = ZoneHitTester::new(map);

        let hit = tester.test(40.0, 40.0);
        assert!(hit.hit);
        assert_eq!(hit.zone_id, Some(7));
        assert_eq!(hit.kind, Some(HitZoneKind::Click));

        let miss = tester.test(200.0, 200.0);
        assert!(!miss.hit);
        assert!(miss.zone_id.is_none());
    }
}
