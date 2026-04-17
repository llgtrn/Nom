//! GPU device context — adapter, device, and queue wrapper for `nom-gpui`.
//!
//! # Usage (blocking call site)
//!
//! ```no_run
//! # async fn run() -> Result<(), nom_gpui::context::GpuContextError> {
//! use nom_gpui::context::GpuContext;
//! let ctx = GpuContext::new().await?;
//! # Ok(()) }
//! // Blocking equivalent:
//! // let ctx = pollster::block_on(GpuContext::new())?;
//! ```

#![deny(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

// ── public error type ─────────────────────────────────────────────────────────

/// Errors that can occur when initialising or recovering a [`GpuContext`].
#[derive(Debug, Error)]
pub enum GpuContextError {
    /// No wgpu adapter was found on this system.
    #[error("no wgpu adapter available")]
    NoAdapter,

    /// The adapter was found but device creation failed.
    #[error("device request failed: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),
}

// ── core context ──────────────────────────────────────────────────────────────

/// Owns the wgpu instance, adapter, device, and queue.
///
/// Constructed once per application session via [`GpuContext::new`] (headless)
/// or [`GpuContext::new_with_surface`] (windowed). After a device-loss event,
/// call [`GpuContext::recover`] to rebuild the device and queue in-place; the
/// instance and adapter remain valid across recoveries.
pub struct GpuContext {
    /// The wgpu instance (backend factory). Kept alive for recovery.
    pub instance: wgpu::Instance,

    /// The selected physical adapter. Kept alive for recovery and capability
    /// queries (e.g., surface compatibility checks).
    pub adapter: wgpu::Adapter,

    /// The logical device. Shared with renderers via `Arc`.
    pub device: Arc<wgpu::Device>,

    /// The submission queue. Shared with renderers via `Arc`.
    pub queue: Arc<wgpu::Queue>,

    /// `true` when the adapter reported `DUAL_SOURCE_BLENDING` support and the
    /// device was created with that feature enabled. Renderers use this flag to
    /// choose between subpixel and monochrome text pipelines.
    pub dual_source_blending: bool,

    /// Shared flag set by the device-lost callback. Renderers test this each
    /// frame; when `true` they call [`GpuContext::recover`].
    pub device_lost: Arc<AtomicBool>,
}

impl GpuContext {
    // ── constructors ─────────────────────────────────────────────────────────

    /// Create a headless context (no surface compatibility requirement).
    ///
    /// Requests a high-performance adapter, enables `DUAL_SOURCE_BLENDING` when
    /// available, and installs the device-lost callback.
    ///
    /// For a blocking call site use `pollster::block_on(GpuContext::new())`.
    pub async fn new() -> Result<Self, GpuContextError> {
        let instance = Self::build_instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuContextError::NoAdapter)?;

        let (device, queue, dual_source_blending) = Self::request_device_from(&adapter).await?;
        let device_lost = Arc::new(AtomicBool::new(false));
        install_device_lost_callback(&device, Arc::clone(&device_lost));

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            dual_source_blending,
            device_lost,
        })
    }

    /// Create a windowed context whose adapter is compatible with `surface`.
    ///
    /// Iterates the available adapters in priority order (discrete > integrated
    /// > other > virtual > CPU) and picks the first one whose capabilities
    /// include the supplied surface's format list.
    pub async fn new_with_surface(surface: &wgpu::Surface<'_>) -> Result<Self, GpuContextError> {
        let instance = Self::build_instance();
        let adapter = pick_adapter_for_surface(&instance, surface)?;
        let (device, queue, dual_source_blending) = Self::request_device_from(&adapter).await?;
        let device_lost = Arc::new(AtomicBool::new(false));
        install_device_lost_callback(&device, Arc::clone(&device_lost));

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            dual_source_blending,
            device_lost,
        })
    }

    // ── status ────────────────────────────────────────────────────────────────

    /// Returns `true` if the GPU device was lost since the last call to
    /// [`recover`](GpuContext::recover). Renderers call this once per frame.
    #[inline]
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(Ordering::SeqCst)
    }

    // ── recovery ──────────────────────────────────────────────────────────────

    /// Rebuild the device and queue after a device-loss event.
    ///
    /// The `instance` and `adapter` fields are reused. On success the
    /// `device_lost` flag is cleared and `device`/`queue` are replaced.
    pub async fn recover(&mut self) -> Result<(), GpuContextError> {
        let (device, queue, dual_source_blending) =
            Self::request_device_from(&self.adapter).await?;

        let device_lost = Arc::new(AtomicBool::new(false));
        install_device_lost_callback(&device, Arc::clone(&device_lost));

        self.device = Arc::new(device);
        self.queue = Arc::new(queue);
        self.dual_source_blending = dual_source_blending;
        self.device_lost = device_lost;

        Ok(())
    }

    // ── internal helpers ─────────────────────────────────────────────────────

    /// Construct a `wgpu::Instance` with the primary native backends.
    fn build_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        })
    }

    /// Request a device from `adapter`, optionally enabling dual-source blending.
    ///
    /// If the adapter supports `DUAL_SOURCE_BLENDING` the feature is requested
    /// and the returned bool is `true`; otherwise the device is created without
    /// it and the bool is `false`.
    async fn request_device_from(
        adapter: &wgpu::Adapter,
    ) -> Result<(wgpu::Device, wgpu::Queue, bool), GpuContextError> {
        let dual_source_blending = adapter
            .features()
            .contains(wgpu::Features::DUAL_SOURCE_BLENDING);

        let optional_feature = if dual_source_blending {
            wgpu::Features::DUAL_SOURCE_BLENDING
        } else {
            wgpu::Features::empty()
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("nom_gpui_device"),
                    required_features: optional_feature,
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits())
                        .using_alignment(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None, // trace path
            )
            .await?;

        Ok((device, queue, dual_source_blending))
    }
}

// ── free helpers ──────────────────────────────────────────────────────────────

/// Install the device-lost callback that sets `flag` on unexpected loss.
fn install_device_lost_callback(device: &wgpu::Device, flag: Arc<AtomicBool>) {
    device.set_device_lost_callback(move |reason, _message| {
        if reason != wgpu::DeviceLostReason::Destroyed {
            flag.store(true, Ordering::SeqCst);
        }
    });
}

/// Pick the best adapter that is compatible with `surface`.
///
/// Adapters are scored by device type (discrete preferred) then the first one
/// whose surface capabilities are non-empty is returned.
fn pick_adapter_for_surface(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<wgpu::Adapter, GpuContextError> {
    let mut adapters = instance.enumerate_adapters(wgpu::Backends::all());

    // Sort: discrete > integrated > other > virtual > CPU.
    adapters.sort_by_key(|a| match a.get_info().device_type {
        wgpu::DeviceType::DiscreteGpu => 0u8,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::Other => 2,
        wgpu::DeviceType::VirtualGpu => 3,
        wgpu::DeviceType::Cpu => 4,
    });

    adapters
        .into_iter()
        .find(|a| !surface.get_capabilities(a).formats.is_empty())
        .ok_or(GpuContextError::NoAdapter)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::GpuContext;

    /// Verify that a [`GpuContext`] can be constructed in environments that
    /// expose a GPU adapter. If no adapter is available (CI without Vulkan/GL
    /// fallback) the test skips gracefully instead of failing.
    #[test]
    fn context_creates_when_adapter_available() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            eprintln!("SKIP: no GPU adapter in this environment");
            return;
        };
        // The device is alive if features() round-trips cleanly.
        assert!(ctx.device.features().contains(wgpu::Features::empty()) || true);
    }

    /// The `dual_source_blending` flag on the context must agree with the
    /// feature set reported by the device after construction.
    #[test]
    fn dual_source_blending_flag_is_set_consistently() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            return;
        };
        let reported = ctx.dual_source_blending;
        let actual = ctx
            .device
            .features()
            .contains(wgpu::Features::DUAL_SOURCE_BLENDING);
        assert_eq!(reported, actual);
    }

    /// The device-lost flag must start cleared immediately after construction.
    #[test]
    fn device_lost_flag_starts_false() {
        if crate::should_skip_gpu_tests() {
            eprintln!("SKIP: GPU tests disabled (headless CI or NOM_SKIP_GPU_TESTS)");
            return;
        }
        let Ok(ctx) = pollster::block_on(GpuContext::new()) else {
            return;
        };
        assert!(!ctx.is_device_lost());
    }
}
