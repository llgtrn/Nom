// SceneBuilder — constructs a scene graph frame-by-frame

/// A single z-ordered layer within a scene frame.
pub struct SceneLayer {
    pub z_index: i32,
    pub element_count: u32,
}

/// Accumulates layers across frames and tracks frame count.
pub struct SceneBuilder {
    pub layers: Vec<SceneLayer>,
    pub frame_count: u64,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            frame_count: 0,
        }
    }

    /// Increments the frame counter and clears layers for the new frame.
    pub fn begin_frame(&mut self) {
        self.frame_count += 1;
        self.layers.clear();
    }

    /// Adds a new layer at the given z_index and returns a mutable reference to it.
    pub fn add_layer(&mut self, z_index: i32) -> &mut SceneLayer {
        self.layers.push(SceneLayer {
            z_index,
            element_count: 0,
        });
        self.layers.last_mut().expect("just pushed")
    }

    /// Returns the number of layers currently in this frame.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Returns references to all layers sorted by z_index ascending.
    pub fn sorted_layers(&self) -> Vec<&SceneLayer> {
        let mut refs: Vec<&SceneLayer> = self.layers.iter().collect();
        refs.sort_by_key(|l| l.z_index);
        refs
    }

    /// Removes all layers from the current frame without incrementing frame_count.
    pub fn clear(&mut self) {
        self.layers.clear();
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty() {
        let sb = SceneBuilder::new();
        assert_eq!(sb.frame_count, 0);
        assert_eq!(sb.layer_count(), 0);
    }

    #[test]
    fn begin_frame_increments() {
        let mut sb = SceneBuilder::new();
        sb.begin_frame();
        assert_eq!(sb.frame_count, 1);
        sb.begin_frame();
        assert_eq!(sb.frame_count, 2);
    }

    #[test]
    fn add_layer_count() {
        let mut sb = SceneBuilder::new();
        sb.add_layer(0);
        sb.add_layer(1);
        assert_eq!(sb.layer_count(), 2);
    }

    #[test]
    fn sorted_layers_order() {
        let mut sb = SceneBuilder::new();
        sb.add_layer(10);
        sb.add_layer(-5);
        sb.add_layer(0);
        let sorted = sb.sorted_layers();
        let z_indices: Vec<i32> = sorted.iter().map(|l| l.z_index).collect();
        assert_eq!(z_indices, vec![-5, 0, 10]);
    }

    #[test]
    fn clear_removes_all() {
        let mut sb = SceneBuilder::new();
        sb.add_layer(0);
        sb.add_layer(1);
        sb.clear();
        assert_eq!(sb.layer_count(), 0);
    }

    #[test]
    fn layer_element_count() {
        let mut sb = SceneBuilder::new();
        let layer = sb.add_layer(5);
        layer.element_count = 42;
        assert_eq!(sb.layers[0].element_count, 42);
    }

    #[test]
    fn begin_multiple_frames() {
        let mut sb = SceneBuilder::new();
        for _ in 0..10 {
            sb.begin_frame();
        }
        assert_eq!(sb.frame_count, 10);
    }

    #[test]
    fn sorted_layers_empty() {
        let sb = SceneBuilder::new();
        assert!(sb.sorted_layers().is_empty());
    }
}
