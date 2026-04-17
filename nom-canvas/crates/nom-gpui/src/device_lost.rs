#![cfg(feature = "native")]
//! Device-loss recovery: check the shared atomic flag each frame and re-create
//! the wgpu device + queue when set, without losing the adapter or instance.

#![deny(unsafe_code)]

use crate::context::{GpuContext, GpuContextError};

/// Outcome of a device-loss check / recovery attempt.
pub enum RecoveryOutcome {
    /// The device was not lost; no action taken.
    NotLost,
    /// The device was lost and has been successfully re-created.
    Ok,
    /// The device was lost and re-creation failed.
    Failed(GpuContextError),
}

/// Check the device-lost flag on `ctx`; if set, synchronously re-create the
/// device and queue. Returns the outcome so callers can skip the next frame
/// or report the failure to the user.
pub fn check_and_recover(ctx: &mut GpuContext) -> RecoveryOutcome {
    if !ctx.is_device_lost() {
        return RecoveryOutcome::NotLost;
    }
    match pollster::block_on(ctx.recover()) {
        Ok(()) => RecoveryOutcome::Ok,
        Err(e) => RecoveryOutcome::Failed(e),
    }
}
