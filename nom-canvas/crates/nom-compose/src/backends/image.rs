#![deny(unsafe_code)]
use crate::progress::{ComposeEvent, ProgressSink};
use crate::store::ArtifactStore;
use nom_blocks::compose::image_block::ImageBlock;
use nom_blocks::NomtuRef;

// ── Image compositing types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelFormat {
    Rgba8,
    Rgb8,
    Luma8,
    Rgba16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
}

#[derive(Debug, Clone)]
pub struct ImageLayer {
    pub id: String,
    pub path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub x: i32,
    pub y: i32,
}

impl ImageLayer {
    pub fn new(id: &str, width: u32, height: u32) -> Self {
        Self {
            id: id.to_owned(),
            path: None,
            width,
            height,
            format: PixelFormat::Rgba8,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            x: 0,
            y: 0,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_owned());
        self
    }

    pub fn with_opacity(mut self, v: f32) -> Self {
        self.opacity = v.clamp(0.0, 1.0);
        self
    }

    pub fn with_blend(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

#[derive(Debug, Default)]
pub struct ImageComposite {
    pub layers: Vec<ImageLayer>,
    pub output_width: u32,
    pub output_height: u32,
}

impl ImageComposite {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layers: Vec::new(),
            output_width: width,
            output_height: height,
        }
    }

    pub fn push_layer(mut self, layer: ImageLayer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn visible_layers(&self) -> Vec<&ImageLayer> {
        self.layers.iter().filter(|l| l.opacity > 0.0).collect()
    }
}

pub struct ImageInput {
    pub entity: NomtuRef,
    pub pixel_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub prompt_used: String,
}

pub struct ImageBackend;

impl ImageBackend {
    pub fn compose(
        input: ImageInput,
        store: &mut dyn ArtifactStore,
        sink: &dyn ProgressSink,
    ) -> ImageBlock {
        sink.emit(ComposeEvent::Started {
            backend: "image".into(),
            entity_id: input.entity.id.clone(),
        });
        sink.emit(ComposeEvent::Progress {
            percent: 0.5,
            stage: "rasterizing".into(),
            rendered_frames: None,
            encoded_frames: None,
            elapsed_ms: None,
        });
        let artifact_hash = store.write(&input.pixel_data);
        let byte_size = store.byte_size(&artifact_hash).unwrap_or(0);
        sink.emit(ComposeEvent::Completed {
            artifact_hash,
            byte_size,
        });
        ImageBlock {
            entity: input.entity,
            artifact_hash,
            width: input.width,
            height: input.height,
            prompt_used: input.prompt_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LogProgressSink;
    use crate::store::InMemoryStore;

    #[test]
    fn image_compose_basic() {
        let mut store = InMemoryStore::new();
        let input = ImageInput {
            entity: NomtuRef {
                id: "img1".into(),
                word: "banner".into(),
                kind: "media".into(),
            },
            pixel_data: vec![255u8; 64],
            width: 8,
            height: 8,
            prompt_used: "a white square".into(),
        };
        let block = ImageBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 8);
        assert_eq!(block.height, 8);
        assert!(store.exists(&block.artifact_hash));
    }

    #[test]
    fn image_compose_stores_pixel_data() {
        let mut store = InMemoryStore::new();
        let pixel_data: Vec<u8> = (0u8..=255).collect();
        let input = ImageInput {
            entity: NomtuRef {
                id: "img2".into(),
                word: "gradient".into(),
                kind: "media".into(),
            },
            pixel_data: pixel_data.clone(),
            width: 16,
            height: 16,
            prompt_used: "gradient test".into(),
        };
        let block = ImageBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.width, 16);
        assert_eq!(block.height, 16);
        assert_eq!(block.prompt_used, "gradient test");
        let stored = store.read(&block.artifact_hash).unwrap();
        assert_eq!(stored, pixel_data);
    }

    // ── ImageLayer / ImageComposite tests ─────────────────────────────────────

    #[test]
    fn image_layer_new_defaults() {
        let layer = ImageLayer::new("bg", 1920, 1080);
        assert_eq!(layer.id, "bg");
        assert_eq!(layer.width, 1920);
        assert_eq!(layer.height, 1080);
        assert_eq!(layer.format, PixelFormat::Rgba8);
        assert_eq!(layer.opacity, 1.0);
        assert_eq!(layer.blend_mode, BlendMode::Normal);
        assert_eq!(layer.x, 0);
        assert_eq!(layer.y, 0);
        assert!(layer.path.is_none());
    }

    #[test]
    fn image_layer_builder_methods() {
        let layer = ImageLayer::new("fg", 100, 100)
            .with_path("/tmp/fg.png")
            .with_opacity(0.75)
            .with_blend(BlendMode::Multiply)
            .with_position(10, 20);
        assert_eq!(layer.path.as_deref(), Some("/tmp/fg.png"));
        assert!((layer.opacity - 0.75).abs() < f32::EPSILON);
        assert_eq!(layer.blend_mode, BlendMode::Multiply);
        assert_eq!(layer.x, 10);
        assert_eq!(layer.y, 20);
    }

    #[test]
    fn image_layer_opacity_clamped() {
        let over = ImageLayer::new("a", 1, 1).with_opacity(2.5);
        assert_eq!(over.opacity, 1.0);
        let under = ImageLayer::new("b", 1, 1).with_opacity(-1.0);
        assert_eq!(under.opacity, 0.0);
    }

    #[test]
    fn image_layer_pixel_count() {
        let layer = ImageLayer::new("x", 320, 240);
        assert_eq!(layer.pixel_count(), 76_800);
    }

    #[test]
    fn image_composite_push_and_visible() {
        let comp = ImageComposite::new(800, 600)
            .push_layer(ImageLayer::new("a", 800, 600))
            .push_layer(ImageLayer::new("b", 400, 300).with_opacity(0.0))
            .push_layer(ImageLayer::new("c", 200, 200).with_opacity(0.5));
        assert_eq!(comp.layer_count(), 3);
        assert_eq!(comp.output_width, 800);
        assert_eq!(comp.output_height, 600);
        let visible = comp.visible_layers();
        assert_eq!(visible.len(), 2);
        assert!(visible.iter().all(|l| l.opacity > 0.0));
    }

    #[test]
    fn image_compose_entity_propagated() {
        let mut store = InMemoryStore::new();
        let input = ImageInput {
            entity: NomtuRef {
                id: "img3".into(),
                word: "thumbnail".into(),
                kind: "media".into(),
            },
            pixel_data: vec![0u8; 16],
            width: 4,
            height: 4,
            prompt_used: "black thumbnail".into(),
        };
        let block = ImageBackend::compose(input, &mut store, &LogProgressSink);
        assert_eq!(block.entity.id, "img3");
        assert_eq!(block.entity.word, "thumbnail");
    }
}
