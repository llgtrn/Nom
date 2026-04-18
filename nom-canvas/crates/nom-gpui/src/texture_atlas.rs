/// A rectangular region allocated within a texture atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub atlas_id: u32,
}

impl AtlasRegion {
    pub fn new(x: u32, y: u32, width: u32, height: u32, atlas_id: u32) -> Self {
        Self { x, y, width, height, atlas_id }
    }

    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

/// A GPU texture atlas that tracks its allocated regions.
pub struct TextureAtlas {
    pub width: u32,
    pub height: u32,
    pub regions: Vec<AtlasRegion>,
    pub atlas_id: u32,
}

impl TextureAtlas {
    pub fn new(width: u32, height: u32, atlas_id: u32) -> Self {
        Self { width, height, regions: Vec::new(), atlas_id }
    }

    pub fn add_region(&mut self, region: AtlasRegion) {
        self.regions.push(region);
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn used_area(&self) -> u32 {
        self.regions.iter().map(|r| r.area()).sum()
    }

    pub fn utilization(&self) -> f32 {
        let total = self.width * self.height;
        if total == 0 {
            return 0.0;
        }
        self.used_area() as f32 / total as f32
    }
}

/// A single horizontal shelf inside the atlas used by the shelf-based allocator.
pub struct AtlasShelf {
    pub y: u32,
    pub height: u32,
    pub cursor_x: u32,
}

impl AtlasShelf {
    pub fn new(y: u32, height: u32) -> Self {
        Self { y, height, cursor_x: 0 }
    }

    /// Returns true when a glyph of width `w` still fits on this shelf given
    /// the atlas is `atlas_width` pixels wide.
    pub fn can_fit(&self, w: u32, atlas_width: u32) -> bool {
        self.cursor_x + w <= atlas_width
    }

    /// Allocates a region of size `(w, self.height)` at the current cursor and
    /// advances the cursor by `w`.
    pub fn allocate(&mut self, w: u32, atlas_id: u32) -> AtlasRegion {
        let region = AtlasRegion::new(self.cursor_x, self.y, w, self.height, atlas_id);
        self.cursor_x += w;
        region
    }
}

/// Shelf-based texture atlas allocator (Zed GPUI pattern).
///
/// Glyphs / sprites are packed into horizontal shelves whose height matches the
/// tallest item placed on that shelf.  A new shelf is opened whenever no
/// existing shelf can accommodate the requested height × width.
pub struct AtlasAllocator {
    pub atlas: TextureAtlas,
    pub shelves: Vec<AtlasShelf>,
    pub cursor_y: u32,
}

impl AtlasAllocator {
    pub fn new(width: u32, height: u32, atlas_id: u32) -> Self {
        Self {
            atlas: TextureAtlas::new(width, height, atlas_id),
            shelves: Vec::new(),
            cursor_y: 0,
        }
    }

    /// Allocates a region of `(w, h)` pixels.
    ///
    /// Strategy:
    /// 1. Search existing shelves for one whose height >= h and that still has
    ///    horizontal space.
    /// 2. If none found, open a new shelf at `cursor_y` with height `h`,
    ///    provided there is vertical space remaining.
    /// 3. Return `None` when the atlas is full.
    pub fn allocate(&mut self, w: u32, h: u32) -> Option<AtlasRegion> {
        let atlas_width = self.atlas.width;
        let atlas_id = self.atlas.atlas_id;

        // Try to fit into an existing shelf (first-fit, height must accommodate h).
        for shelf in &mut self.shelves {
            if shelf.height >= h && shelf.can_fit(w, atlas_width) {
                let region = shelf.allocate(w, atlas_id);
                self.atlas.add_region(region.clone());
                return Some(region);
            }
        }

        // Open a new shelf if there is vertical space.
        if self.cursor_y + h > self.atlas.height {
            return None;
        }

        let mut shelf = AtlasShelf::new(self.cursor_y, h);
        self.cursor_y += h;

        // Allocate from the freshly-opened shelf.
        if !shelf.can_fit(w, atlas_width) {
            return None;
        }

        let region = shelf.allocate(w, atlas_id);
        self.atlas.add_region(region.clone());
        self.shelves.push(shelf);
        Some(region)
    }

    pub fn shelf_count(&self) -> usize {
        self.shelves.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod texture_atlas_tests {
    use super::*;

    #[test]
    fn atlas_region_area() {
        let r = AtlasRegion::new(0, 0, 4, 8, 0);
        assert_eq!(r.area(), 32);
    }

    #[test]
    fn atlas_region_right_bottom() {
        let r = AtlasRegion::new(10, 20, 5, 3, 0);
        assert_eq!(r.right(), 15);
        assert_eq!(r.bottom(), 23);
    }

    #[test]
    fn texture_atlas_add_and_count() {
        let mut atlas = TextureAtlas::new(512, 512, 1);
        assert_eq!(atlas.region_count(), 0);
        atlas.add_region(AtlasRegion::new(0, 0, 32, 32, 1));
        atlas.add_region(AtlasRegion::new(32, 0, 16, 16, 1));
        assert_eq!(atlas.region_count(), 2);
    }

    #[test]
    fn texture_atlas_used_area() {
        let mut atlas = TextureAtlas::new(512, 512, 1);
        atlas.add_region(AtlasRegion::new(0, 0, 10, 10, 1)); // 100
        atlas.add_region(AtlasRegion::new(10, 0, 5, 20, 1)); // 100
        assert_eq!(atlas.used_area(), 200);
    }

    #[test]
    fn texture_atlas_utilization() {
        let mut atlas = TextureAtlas::new(100, 100, 1);
        atlas.add_region(AtlasRegion::new(0, 0, 50, 100, 1)); // 5000 / 10000 = 0.5
        let util = atlas.utilization();
        assert!((util - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn atlas_shelf_can_fit() {
        let shelf = AtlasShelf::new(0, 32);
        assert!(shelf.can_fit(512, 512));
        assert!(!shelf.can_fit(513, 512));
    }

    #[test]
    fn atlas_allocator_allocate_returns_region() {
        let mut alloc = AtlasAllocator::new(512, 512, 0);
        let region = alloc.allocate(64, 32).expect("should allocate");
        assert_eq!(region.x, 0);
        assert_eq!(region.y, 0);
        assert_eq!(region.width, 64);
        assert_eq!(region.height, 32);
    }

    #[test]
    fn atlas_allocator_allocate_new_shelf() {
        // Atlas is only 64 pixels wide.  First allocation fills shelf 0 (y=0,
        // h=32) completely.  Second allocation cannot fit on shelf 0 and must
        // open shelf 1 at y=32.
        let mut alloc = AtlasAllocator::new(64, 512, 0);
        alloc.allocate(64, 32).unwrap();   // fills shelf 0 (cursor_x == atlas_width)
        alloc.allocate(32, 16).unwrap();   // no room on shelf 0 → new shelf 1
        assert_eq!(alloc.shelf_count(), 2);
    }

    #[test]
    fn atlas_allocator_allocate_full_returns_none() {
        // Atlas is exactly 64×32; one allocation fills it completely.
        let mut alloc = AtlasAllocator::new(64, 32, 0);
        assert!(alloc.allocate(64, 32).is_some());
        // The atlas is now full; any further allocation must return None.
        assert!(alloc.allocate(1, 1).is_none());
    }
}
