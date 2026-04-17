use crate::types::*;

/// Platform abstraction — desktop vs WebGPU
/// Pattern: Zed platform.rs with cfg(target_arch = "wasm32") splits
pub trait Platform: Send + Sync {
    fn create_surface_descriptor(&self) -> SurfaceDescriptor;
    fn adapter_options(&self) -> AdapterOptions;
    fn present_mode(&self) -> PresentMode;
    fn device_features(&self) -> u64; // wgpu::Features flags
}

pub struct SurfaceDescriptor {
    pub format: TextureFormat,
    pub present_mode: PresentMode,
    pub alpha_mode: AlphaMode,
}

#[derive(Debug, Clone, Copy)]
pub enum TextureFormat {
    Bgra8UnormSrgb,
    Rgba8UnormSrgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    Fifo,      // VSync
    Immediate, // No VSync (lower latency)
    Mailbox,   // Triple buffer
}

#[derive(Debug, Clone, Copy)]
pub enum AlphaMode {
    Opaque,
    PreMultiplied,
}

pub struct AdapterOptions {
    pub power_preference: PowerPreference,
    pub force_fallback: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum PowerPreference {
    None,
    LowPower,
    HighPerformance,
}

// Suppress unused import warning — Vec2 may be used by future platform extensions
#[allow(unused_imports)]
use Vec2 as _;

/// Desktop platform (wgpu native surface, winit)
#[cfg(not(target_arch = "wasm32"))]
pub struct DesktopPlatform;

#[cfg(not(target_arch = "wasm32"))]
impl Platform for DesktopPlatform {
    fn create_surface_descriptor(&self) -> SurfaceDescriptor {
        SurfaceDescriptor {
            format: TextureFormat::Bgra8UnormSrgb,
            present_mode: PresentMode::Fifo,
            alpha_mode: AlphaMode::Opaque,
        }
    }
    fn adapter_options(&self) -> AdapterOptions {
        AdapterOptions { power_preference: PowerPreference::HighPerformance, force_fallback: false }
    }
    fn present_mode(&self) -> PresentMode { PresentMode::Fifo }
    fn device_features(&self) -> u64 { 0 }
}

/// WebGPU platform (wgpu WebGPU via wasm-bindgen, web_sys canvas)
#[cfg(target_arch = "wasm32")]
pub struct WebPlatform;

#[cfg(target_arch = "wasm32")]
impl Platform for WebPlatform {
    fn create_surface_descriptor(&self) -> SurfaceDescriptor {
        SurfaceDescriptor {
            format: TextureFormat::Rgba8UnormSrgb,
            present_mode: PresentMode::Fifo,
            alpha_mode: AlphaMode::PreMultiplied,
        }
    }
    fn adapter_options(&self) -> AdapterOptions {
        AdapterOptions { power_preference: PowerPreference::LowPower, force_fallback: false }
    }
    fn present_mode(&self) -> PresentMode { PresentMode::Fifo }
    fn device_features(&self) -> u64 { 0 }
}

/// Get the default platform for the current target
pub fn default_platform() -> Box<dyn Platform> {
    #[cfg(not(target_arch = "wasm32"))]
    { Box::new(DesktopPlatform) }
    #[cfg(target_arch = "wasm32")]
    { Box::new(WebPlatform) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_platform_present_mode_is_fifo() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let p = DesktopPlatform;
            assert_eq!(p.present_mode(), PresentMode::Fifo);
        }
    }

    #[test]
    fn default_platform_does_not_panic() {
        let _p = default_platform();
    }

    #[test]
    fn desktop_platform_surface_descriptor_format() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let p = DesktopPlatform;
            let desc = p.create_surface_descriptor();
            // Desktop should use Bgra format
            assert!(matches!(desc.format, TextureFormat::Bgra8UnormSrgb));
        }
    }

    #[test]
    fn desktop_platform_surface_descriptor_present_mode() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let p = DesktopPlatform;
            let desc = p.create_surface_descriptor();
            assert_eq!(desc.present_mode, PresentMode::Fifo);
        }
    }

    #[test]
    fn desktop_platform_adapter_prefers_high_performance() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let p = DesktopPlatform;
            let opts = p.adapter_options();
            assert!(matches!(opts.power_preference, PowerPreference::HighPerformance));
            assert!(!opts.force_fallback);
        }
    }

    #[test]
    fn desktop_platform_device_features_is_zero() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let p = DesktopPlatform;
            assert_eq!(p.device_features(), 0);
        }
    }

    #[test]
    fn present_mode_equality() {
        assert_eq!(PresentMode::Fifo, PresentMode::Fifo);
        assert_ne!(PresentMode::Fifo, PresentMode::Immediate);
        assert_ne!(PresentMode::Immediate, PresentMode::Mailbox);
    }

    #[test]
    fn default_platform_returns_fifo_present_mode() {
        let p = default_platform();
        assert_eq!(p.present_mode(), PresentMode::Fifo);
    }

    #[test]
    fn default_platform_device_features_is_zero() {
        let p = default_platform();
        assert_eq!(p.device_features(), 0);
    }
}
