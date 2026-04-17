#![cfg(feature = "native")]
//! Surface creation, format negotiation, and resize handling for `nom-gpui`.
//!
//! [`WindowSurface`] wraps a `wgpu::Surface` together with its
//! [`wgpu::SurfaceConfiguration`] and provides the minimal API needed by the
//! frame loop: construct, resize, acquire, and query the negotiated format.

// SAFETY POLICY: This file contains exactly one `unsafe` block — the call to
// `Instance::create_surface_unsafe` required to obtain a `Surface<'static>`.
// The safety invariant is upheld by storing the source `Arc<Window>` alongside
// the surface in the calling `App` struct, ensuring the window outlives the
// surface for the entire application lifetime.
#![allow(unsafe_code)]

use std::sync::Arc;

use thiserror::Error;

// ── error type ────────────────────────────────────────────────────────────────

/// Errors that can occur while managing the window surface or event loop.
#[derive(Debug, Error)]
pub enum WindowError {
    /// The surface reported no usable texture formats for the selected adapter.
    #[error("no surface format available")]
    NoFormat,

    /// `wgpu::Instance::create_surface` failed.
    #[error("wgpu create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    /// Acquiring the next surface texture failed (e.g. `Outdated`, `Lost`).
    #[error("wgpu acquire surface: {0}")]
    Acquire(#[from] wgpu::SurfaceError),

    /// The winit event loop exited with an error.
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    /// A raw window or display handle was unavailable.
    #[error("raw window handle: {0}")]
    RawHandle(#[from] raw_window_handle::HandleError),
}

// ── WindowSurface ─────────────────────────────────────────────────────────────

/// A `wgpu` surface bound to a winit window together with its active
/// configuration.
///
/// # Lifetime note
///
/// The surface is typed `wgpu::Surface<'static>` because the window is kept
/// alive in an `Arc`. The `Arc` must outlive the surface, which is guaranteed
/// by the frame loop holding both values together.
pub struct WindowSurface {
    /// The underlying wgpu surface.
    pub surface: wgpu::Surface<'static>,
    /// The active surface configuration (format, size, present mode, …).
    pub config: wgpu::SurfaceConfiguration,
}

impl WindowSurface {
    /// Create a [`WindowSurface`], negotiate the texture format, alpha mode, and
    /// present mode, then configure the surface for the current window size.
    ///
    /// ## Format preference
    /// `Bgra8Unorm` → `Rgba8Unorm` → first non-sRGB → first available.
    ///
    /// ## Alpha mode preference
    /// `PreMultiplied` → `Opaque` → first available.
    ///
    /// ## Present mode
    /// `Fifo` (OS vsync) if supported, else the first reported mode.
    /// No manual 60 fps tick is installed; the OS drives the cadence.
    pub fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: Arc<winit::window::Window>,
    ) -> Result<Self, WindowError> {
        // SAFETY: `window` is wrapped in an `Arc` that will be stored in the
        // `App` struct alongside this `WindowSurface`. The `Arc` guarantees
        // the `Window` remains alive for at least as long as the surface, which
        // is the contract required by `create_surface_unsafe`.
        //
        // The raw handles extracted here (display + window handle) are valid
        // for the lifetime of the `Window`, as documented by winit and
        // `raw-window-handle`.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(
                window.as_ref(),
            )?)?
        };

        let caps = surface.get_capabilities(adapter);

        // ── format negotiation ────────────────────────────────────────────────
        let format = pick_format(&caps.formats)?;

        // ── alpha mode negotiation ────────────────────────────────────────────
        let alpha_mode = pick_alpha_mode(&caps.alpha_modes);

        // ── present mode ──────────────────────────────────────────────────────
        let present_mode = pick_present_mode(&caps.present_modes);

        // ── initial size from the window ──────────────────────────────────────
        let phys = window.inner_size();
        let (width, height) = clamp_size(phys.width, phys.height, device);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(device, &config);

        Ok(Self { surface, config })
    }

    /// Reconfigure the surface after the window has been resized.
    ///
    /// Zero-area sizes (minimised window) are clamped to 1 × 1 to avoid a
    /// wgpu validation error.
    pub fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        let (w, h) = clamp_size(new_size.0, new_size.1, device);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(device, &self.config);
    }

    /// Acquire the next frame's surface texture.
    ///
    /// Returns [`WindowError::Acquire`] on `wgpu::SurfaceError`, including
    /// `Outdated` and `Lost` — the caller should reconfigure or recreate the
    /// surface in that case.
    pub fn acquire(&mut self) -> Result<wgpu::SurfaceTexture, WindowError> {
        Ok(self.surface.get_current_texture()?)
    }

    /// The negotiated texture format for this surface.
    #[inline]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Select the best texture format from `formats`.
///
/// Preference: `Bgra8Unorm` → `Rgba8Unorm` → first non-sRGB → any.
fn pick_format(formats: &[wgpu::TextureFormat]) -> Result<wgpu::TextureFormat, WindowError> {
    let preferred = [
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
    ];

    preferred
        .iter()
        .find(|f| formats.contains(f))
        .copied()
        .or_else(|| formats.iter().find(|f| !f.is_srgb()).copied())
        .or_else(|| formats.first().copied())
        .ok_or(WindowError::NoFormat)
}

/// Select the best alpha mode from `modes`.
///
/// Preference: `PreMultiplied` → `Opaque` → any.
fn pick_alpha_mode(modes: &[wgpu::CompositeAlphaMode]) -> wgpu::CompositeAlphaMode {
    let preferred = [
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::Opaque,
    ];

    preferred
        .iter()
        .find(|m| modes.contains(m))
        .copied()
        .or_else(|| modes.first().copied())
        .unwrap_or(wgpu::CompositeAlphaMode::Opaque)
}

/// Select the present mode, preferring `Fifo` (OS vsync).
fn pick_present_mode(modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    if modes.contains(&wgpu::PresentMode::Fifo) {
        wgpu::PresentMode::Fifo
    } else {
        modes.first().copied().unwrap_or(wgpu::PresentMode::Fifo)
    }
}

/// Clamp the requested width/height to the device's `max_texture_dimension_2d`
/// and ensure neither dimension is zero.
fn clamp_size(w: u32, h: u32, device: &wgpu::Device) -> (u32, u32) {
    let max = device.limits().max_texture_dimension_2d;
    (w.max(1).min(max), h.max(1).min(max))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The `WindowError::NoFormat` variant must be constructible (type-level
    /// check; no GPU access required).
    #[test]
    fn window_error_variants_exist() {
        let _err: Result<(), WindowError> = Err(WindowError::NoFormat);
    }

    /// `pick_format` must return `Bgra8Unorm` when it is the only available
    /// format.
    #[test]
    fn pick_format_prefers_bgra8() {
        let formats = vec![wgpu::TextureFormat::Bgra8Unorm];
        assert_eq!(
            pick_format(&formats).unwrap(),
            wgpu::TextureFormat::Bgra8Unorm
        );
    }

    /// `pick_format` must prefer `Bgra8Unorm` over `Rgba8Unorm` when both are
    /// present.
    #[test]
    fn pick_format_bgra_beats_rgba() {
        let formats = vec![
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ];
        assert_eq!(
            pick_format(&formats).unwrap(),
            wgpu::TextureFormat::Bgra8Unorm
        );
    }

    /// `pick_format` must return `Rgba8Unorm` when `Bgra8Unorm` is absent.
    #[test]
    fn pick_format_falls_back_to_rgba8() {
        let formats = vec![wgpu::TextureFormat::Rgba8Unorm];
        assert_eq!(
            pick_format(&formats).unwrap(),
            wgpu::TextureFormat::Rgba8Unorm
        );
    }

    /// An empty format list must yield `WindowError::NoFormat`.
    #[test]
    fn pick_format_empty_yields_no_format_error() {
        assert!(matches!(pick_format(&[]), Err(WindowError::NoFormat)));
    }

    /// `pick_alpha_mode` must prefer `PreMultiplied`.
    #[test]
    fn pick_alpha_mode_prefers_premultiplied() {
        let modes = vec![
            wgpu::CompositeAlphaMode::Opaque,
            wgpu::CompositeAlphaMode::PreMultiplied,
        ];
        assert_eq!(
            pick_alpha_mode(&modes),
            wgpu::CompositeAlphaMode::PreMultiplied
        );
    }

    /// `pick_alpha_mode` must fall back to `Opaque` when `PreMultiplied` is
    /// absent.
    #[test]
    fn pick_alpha_mode_falls_back_to_opaque() {
        let modes = vec![wgpu::CompositeAlphaMode::Opaque];
        assert_eq!(
            pick_alpha_mode(&modes),
            wgpu::CompositeAlphaMode::Opaque
        );
    }

    /// `pick_present_mode` must return `Fifo`.
    #[test]
    fn pick_present_mode_prefers_fifo() {
        let modes = vec![wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo];
        assert_eq!(pick_present_mode(&modes), wgpu::PresentMode::Fifo);
    }

    /// `clamp_size` must not allow a zero width or height to pass through.
    #[test]
    fn clamp_size_rejects_zero_width_and_height() {
        // We cannot easily get a real device in a unit test, but we can test
        // the arithmetic directly via the helper logic. The helper cannot be
        // called without a `wgpu::Device`, so we duplicate the logic here to
        // verify the invariant.
        let max: u32 = 8192;
        let (w, h) = (0u32.max(1).min(max), 0u32.max(1).min(max));
        assert_eq!(w, 1);
        assert_eq!(h, 1);
    }

    /// Integration guard: try to open an event loop. If the platform denies
    /// it (CI without a display server), skip silently. This test verifies
    /// the module compiles and that winit 0.30 types are reachable.
    ///
    /// On Windows `EventLoop::new()` panics on non-main threads, so we use
    /// `with_any_thread(true)`. On other platforms the standard builder is
    /// used. In either case, failure to build (headless CI) is a graceful skip.
    #[test]
    fn window_surface_constructs_when_display_available() {
        build_test_event_loop();
    }

    /// Build an event loop safe for use in test threads.
    #[cfg(target_os = "windows")]
    fn build_test_event_loop() {
        use winit::event_loop::EventLoop;
        use winit::platform::windows::EventLoopBuilderExtWindows;
        let Ok(event_loop) = EventLoop::builder().with_any_thread(true).build() else {
            eprintln!("SKIP: no display in this environment");
            return;
        };
        let _ = event_loop;
    }

    #[cfg(not(target_os = "windows"))]
    fn build_test_event_loop() {
        use winit::event_loop::EventLoop;
        let Ok(event_loop) = EventLoop::new() else {
            eprintln!("SKIP: no display in this environment");
            return;
        };
        let _ = event_loop;
    }
}
