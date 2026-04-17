//! nom-canvas-core — infinite-canvas element model + (future) spatial index + viewport.
//!
//! # Modules (this agent)
//!
//! - [`element`]   — `Element` struct, `ElementId`, `FrameId`, `GroupId`.
//! - [`mutation`]  — In-place mutation + spread-replacement helpers (CRDT-friendly versioning).
//! - [`shapes`]    — 8-variant `Shape` enum covering all canvas primitives.
//!
//! # Modules (added by sibling agents)
//!
//! TODO: other agents add: hit_testing, spatial_index, viewport, coords, zoom, pan, fit

#![deny(unsafe_code)]

pub mod coords;
pub mod element;
pub mod groups;
pub mod fit;
pub mod history;
pub mod hit_testing;
pub mod marquee;
pub mod mutation;
pub mod pan;
pub mod rendering_hints;
pub mod selection;
pub mod shapes;
pub mod snapping;
pub mod spatial_index;
pub mod transform_handles;
pub mod viewport;
pub mod zoom;

pub use element::{Element, ElementId, FrameId, GroupId};
pub use shapes::Shape;
pub use viewport::Viewport;
pub use spatial_index::{SpatialIndex, DEFAULT_GRID_SIZE};
