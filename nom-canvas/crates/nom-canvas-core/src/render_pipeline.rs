//! Render pipeline coordinator: phase ordering, draw command queues, and a
//! stub frame-graph that bridges toward a real wgpu draw loop.

// ─── RenderPhase ─────────────────────────────────────────────────────────────

/// Ordered render phases executed within a single frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPhase {
    /// Emit opaque geometry (background fills, solid shapes).
    Geometry,
    /// Compute and apply directional / ambient lighting.
    Lighting,
    /// Full-screen post-processing effects (blur, tone-map, etc.).
    PostProcess,
    /// Transparent overlays, UI chrome, debug guides.
    Overlay,
    /// Blit the composed image to the swap-chain surface.
    Present,
}

// ─── DrawCommand ─────────────────────────────────────────────────────────────

/// A single drawing instruction enqueued on a [`RenderQueue`].
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// Clear the target with a solid RGBA colour.
    Clear {
        /// Red channel, `0.0..=1.0`.
        r: f32,
        /// Green channel, `0.0..=1.0`.
        g: f32,
        /// Blue channel, `0.0..=1.0`.
        b: f32,
        /// Alpha channel, `0.0..=1.0`.
        a: f32,
    },
    /// Fill an axis-aligned rectangle.
    DrawRect {
        /// Left edge in pixels.
        x: f32,
        /// Top edge in pixels.
        y: f32,
        /// Width in pixels.
        w: f32,
        /// Height in pixels.
        h: f32,
        /// Packed ARGB colour.
        color: u32,
    },
    /// Rasterise a text string at the given position.
    DrawText {
        /// Horizontal position in pixels.
        x: f32,
        /// Vertical position in pixels.
        y: f32,
        /// The string to render.
        text: String,
    },
    /// Alpha-composite one render layer onto another.
    Composite {
        /// Source layer index.
        src_layer: u32,
        /// Destination layer index.
        dst_layer: u32,
        /// Blend factor `0.0` (transparent) … `1.0` (opaque).
        blend: f32,
    },
}

// ─── RenderQueue ─────────────────────────────────────────────────────────────

/// An ordered list of [`DrawCommand`]s associated with a single [`RenderPhase`].
#[derive(Debug)]
pub struct RenderQueue {
    /// The commands buffered in this queue.
    pub commands: Vec<DrawCommand>,
    /// The phase this queue belongs to.
    pub phase: RenderPhase,
}

impl RenderQueue {
    /// Create an empty queue for `phase`.
    pub fn new(phase: RenderPhase) -> Self {
        Self {
            commands: Vec::new(),
            phase,
        }
    }

    /// Append a command to the back of the queue.
    pub fn push(&mut self, cmd: DrawCommand) {
        self.commands.push(cmd);
    }

    /// Discard all buffered commands without changing the phase.
    pub fn clear_commands(&mut self) {
        self.commands.clear();
    }

    /// Return the number of buffered commands.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

// ─── FrameGraph ──────────────────────────────────────────────────────────────

/// A collection of [`RenderQueue`]s that together describe one rendered frame.
#[derive(Debug)]
pub struct FrameGraph {
    /// All queues registered for this frame, in insertion order.
    pub queues: Vec<RenderQueue>,
}

impl FrameGraph {
    /// Create an empty frame graph.
    pub fn new() -> Self {
        Self { queues: Vec::new() }
    }

    /// Append a [`RenderQueue`] to this frame.
    pub fn add_queue(&mut self, queue: RenderQueue) {
        self.queues.push(queue);
    }

    /// Return the sum of commands across all queues.
    pub fn total_commands(&self) -> usize {
        self.queues.iter().map(|q| q.command_count()).sum()
    }

    /// Collect the phases of every registered queue, in insertion order.
    pub fn phases(&self) -> Vec<RenderPhase> {
        self.queues.iter().map(|q| q.phase).collect()
    }

    /// Stub submission: returns `total_commands()`. In a real integration this
    /// would encode and submit GPU command buffers.
    pub fn execute_stub(&self) -> usize {
        self.total_commands()
    }
}

impl Default for FrameGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ─── RenderPipelineCoordinator ───────────────────────────────────────────────

/// High-level coordinator that owns the per-frame loop: begin → build graph →
/// end.  Tracks monotonic frame index and the command count of the last frame.
#[derive(Debug)]
pub struct RenderPipelineCoordinator {
    /// Total number of completed frames.
    pub frame_count: u64,
    /// Command count returned by the most recently completed frame.
    pub last_frame_commands: usize,
}

impl RenderPipelineCoordinator {
    /// Create a new coordinator with all counters zeroed.
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_frame_commands: 0,
        }
    }

    /// Begin a new frame: returns a fresh, empty [`FrameGraph`] for the caller
    /// to populate.
    pub fn begin_frame(&mut self) -> FrameGraph {
        FrameGraph::new()
    }

    /// End the frame: execute the stub, increment `frame_count`, and cache the
    /// command count.  Returns the number of commands executed.
    pub fn end_frame(&mut self, graph: FrameGraph) -> usize {
        let count = graph.execute_stub();
        self.frame_count += 1;
        self.last_frame_commands = count;
        count
    }

    /// Return the number of completed frames.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl Default for RenderPipelineCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_draw_command() {
        let cmd = DrawCommand::Clear {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        if let DrawCommand::Clear { r, g, b, a } = cmd {
            assert!((r - 0.1).abs() < 1e-6);
            assert!((g - 0.2).abs() < 1e-6);
            assert!((b - 0.3).abs() < 1e-6);
            assert!((a - 1.0).abs() < 1e-6);
        } else {
            panic!("expected Clear variant");
        }
    }

    #[test]
    fn draw_rect_command() {
        let cmd = DrawCommand::DrawRect {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
            color: 0xFF_FF_00_00,
        };
        if let DrawCommand::DrawRect { x, y, w, h, color } = cmd {
            assert!((x - 10.0).abs() < 1e-6);
            assert!((y - 20.0).abs() < 1e-6);
            assert!((w - 100.0).abs() < 1e-6);
            assert!((h - 50.0).abs() < 1e-6);
            assert_eq!(color, 0xFF_FF_00_00);
        } else {
            panic!("expected DrawRect variant");
        }
    }

    #[test]
    fn render_queue_push() {
        let mut q = RenderQueue::new(RenderPhase::Geometry);
        q.push(DrawCommand::Clear {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        q.push(DrawCommand::DrawRect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            color: 0,
        });
        assert_eq!(q.commands.len(), 2);
    }

    #[test]
    fn render_queue_count() {
        let mut q = RenderQueue::new(RenderPhase::Overlay);
        assert_eq!(q.command_count(), 0);
        q.push(DrawCommand::DrawText {
            x: 5.0,
            y: 5.0,
            text: "hello".to_string(),
        });
        assert_eq!(q.command_count(), 1);
        q.clear_commands();
        assert_eq!(q.command_count(), 0);
    }

    #[test]
    fn frame_graph_add_queue() {
        let mut fg = FrameGraph::new();
        fg.add_queue(RenderQueue::new(RenderPhase::Geometry));
        fg.add_queue(RenderQueue::new(RenderPhase::Present));
        assert_eq!(fg.queues.len(), 2);
    }

    #[test]
    fn frame_graph_total_commands() {
        let mut fg = FrameGraph::new();

        let mut q1 = RenderQueue::new(RenderPhase::Geometry);
        q1.push(DrawCommand::Clear {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        q1.push(DrawCommand::DrawRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            color: 0,
        });

        let mut q2 = RenderQueue::new(RenderPhase::Overlay);
        q2.push(DrawCommand::DrawText {
            x: 0.0,
            y: 0.0,
            text: "x".to_string(),
        });

        fg.add_queue(q1);
        fg.add_queue(q2);
        assert_eq!(fg.total_commands(), 3);
    }

    #[test]
    fn frame_graph_phases() {
        let mut fg = FrameGraph::new();
        fg.add_queue(RenderQueue::new(RenderPhase::Geometry));
        fg.add_queue(RenderQueue::new(RenderPhase::Lighting));
        fg.add_queue(RenderQueue::new(RenderPhase::Present));

        let phases = fg.phases();
        assert_eq!(
            phases,
            vec![RenderPhase::Geometry, RenderPhase::Lighting, RenderPhase::Present]
        );
    }

    #[test]
    fn frame_graph_execute_stub() {
        let mut fg = FrameGraph::new();
        let mut q = RenderQueue::new(RenderPhase::PostProcess);
        q.push(DrawCommand::Composite {
            src_layer: 0,
            dst_layer: 1,
            blend: 0.5,
        });
        q.push(DrawCommand::Composite {
            src_layer: 1,
            dst_layer: 2,
            blend: 1.0,
        });
        fg.add_queue(q);
        assert_eq!(fg.execute_stub(), 2);
    }

    #[test]
    fn coordinator_begin_end_frame() {
        let mut coord = RenderPipelineCoordinator::new();
        let mut graph = coord.begin_frame();

        let mut q = RenderQueue::new(RenderPhase::Geometry);
        q.push(DrawCommand::Clear {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
        graph.add_queue(q);

        let executed = coord.end_frame(graph);
        assert_eq!(executed, 1);
        assert_eq!(coord.last_frame_commands, 1);
    }

    #[test]
    fn coordinator_frame_count() {
        let mut coord = RenderPipelineCoordinator::new();
        assert_eq!(coord.frame_count(), 0);

        for _ in 0..5 {
            let graph = coord.begin_frame();
            coord.end_frame(graph);
        }
        assert_eq!(coord.frame_count(), 5);
    }
}
