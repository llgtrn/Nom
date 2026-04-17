//! GPU instance buffer manager for nom-gpui batch-2.
//!
//! [`InstanceBuffer`] owns a single `wgpu::Buffer` used as a streaming write
//! target for per-instance data during a frame.  Each frame the caller resets
//! the cursor with [`InstanceBuffer::begin_frame`], then writes batches with
//! [`InstanceBuffer::write`].  When capacity runs out the caller calls
//! [`InstanceBuffer::grow`] (which doubles the backing allocation) and retries.
//!
//! # Pure math helpers
//!
//! [`align_up`], [`compute_write_slot`], and [`next_capacity`] contain all
//! arithmetic and are unit-tested without a real GPU device.

#![deny(unsafe_code)]

use std::num::NonZeroU64;
use thiserror::Error;

// ── Constants ────────────────────────────────────────────────────────────────

/// Initial backing-buffer size: 2 MiB.
const INITIAL_CAPACITY: u64 = 2 * 1024 * 1024;

/// Minimum binding size required by wgpu for storage-buffer ranges.
const MIN_BINDING_BYTES: u64 = 16;

// ── Pure math helpers ─────────────────────────────────────────────────────────

/// Round `value` up to the nearest multiple of `alignment`.
///
/// `alignment` must be a power of two and greater than zero; if it is zero
/// the function returns `value` unchanged (defensive, not normally triggered).
pub fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }
    (value + alignment - 1) & !(alignment - 1)
}

/// Compute the aligned write offset and the exclusive end offset for a new
/// write into the buffer.
///
/// Returns `Some((aligned_offset, write_end))` when the write fits, `None`
/// when it would overflow `capacity`.  The effective `size` is clamped to a
/// minimum of [`MIN_BINDING_BYTES`] to satisfy wgpu binding constraints.
///
/// # Parameters
///
/// - `cursor`    — next free byte (updated by the caller after a successful write)
/// - `size`      — length of the data to write in bytes
/// - `alignment` — `storage_alignment` from device limits
/// - `capacity`  — total allocated bytes in the buffer
pub fn compute_write_slot(
    cursor: u64,
    size: u64,
    alignment: u64,
    capacity: u64,
) -> Option<(u64, u64)> {
    let aligned_offset = align_up(cursor, alignment);
    let effective_size = size.max(MIN_BINDING_BYTES);
    let write_end = aligned_offset.checked_add(effective_size)?;
    if write_end > capacity {
        return None;
    }
    Some((aligned_offset, write_end))
}

/// Compute the next buffer capacity by doubling `current`, clamped to `max`.
///
/// Returns [`GrowError::AtMax`] when `current >= max` (already saturated).
pub fn next_capacity(current: u64, max: u64) -> Result<u64, GrowError> {
    if current >= max {
        return Err(GrowError::AtMax(max));
    }
    Ok(current.saturating_mul(2).min(max))
}

// ── Public types ──────────────────────────────────────────────────────────────

/// A resolved write range within an [`InstanceBuffer`].
///
/// Pass `offset` and `size` to `wgpu::BufferBinding` when constructing bind
/// groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferSlice {
    /// Byte offset from the start of the buffer where this slice begins.
    pub offset: u64,
    /// Length of the slice in bytes (at least 16, as required by wgpu).
    pub size: NonZeroU64,
}

/// Error returned by [`InstanceBuffer::grow`].
#[derive(Debug, Error)]
pub enum GrowError {
    /// The buffer is already at the device-reported `max_buffer_size`.
    #[error("instance buffer already at device max_buffer_size {0}")]
    AtMax(u64),
}

// ── InstanceBuffer ────────────────────────────────────────────────────────────

/// A growable, frame-reset GPU buffer used to stream per-instance data.
///
/// The buffer is created with `STORAGE | COPY_DST` usage flags so it can be
/// bound as a read-only storage binding in render pipelines and written via
/// the queue each frame.
///
/// # Frame lifecycle
///
/// ```text
/// begin_frame()           — reset cursor to 0
/// write(queue, &data)     — returns Some(BufferSlice) or None on overflow
/// grow(device)            — double capacity, returns Err if already at max
/// write(queue, &data)     — retry after grow succeeds
/// ```
pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
    /// Currently allocated bytes.
    capacity: u64,
    /// Next write offset within the current frame.
    cursor: u64,
    /// Device-reported maximum buffer size.
    max_capacity: u64,
    /// Required alignment for storage-buffer offsets.
    storage_alignment: u64,
}

impl InstanceBuffer {
    /// Allocate an [`InstanceBuffer`] backed by a fresh `wgpu::Buffer`.
    ///
    /// The initial capacity is [`INITIAL_CAPACITY`] (2 MiB).  `max_capacity`
    /// and `storage_alignment` are read directly from `device.limits()`.
    pub fn new(device: &wgpu::Device) -> Self {
        let limits = device.limits();
        let capacity = INITIAL_CAPACITY;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nom_gpui_instance_buffer"),
            size: capacity,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity,
            cursor: 0,
            max_capacity: limits.max_buffer_size,
            storage_alignment: u64::from(limits.min_storage_buffer_offset_alignment),
        }
    }

    /// Reset the write cursor to the start of the buffer.
    ///
    /// Call once at the beginning of each frame before any [`Self::write`]
    /// calls.  Previous frame data is silently overwritten.
    pub fn begin_frame(&mut self) {
        self.cursor = 0;
    }

    /// Write `bytes` into the buffer at the next aligned slot.
    ///
    /// Returns `Some(BufferSlice)` with the offset and size of the written
    /// region on success.  Returns `None` when the remaining capacity is
    /// insufficient; the caller should call [`Self::grow`] and retry.
    ///
    /// The effective write size is clamped upward to 16 bytes to satisfy the
    /// minimum wgpu storage-buffer binding size, even when `bytes` is shorter.
    pub fn write(&mut self, queue: &wgpu::Queue, bytes: &[u8]) -> Option<BufferSlice> {
        let (aligned_offset, write_end) = compute_write_slot(
            self.cursor,
            bytes.len() as u64,
            self.storage_alignment,
            self.capacity,
        )?;
        queue.write_buffer(&self.buffer, aligned_offset, bytes);
        self.cursor = write_end;
        let effective_size = (bytes.len() as u64).max(MIN_BINDING_BYTES);
        Some(BufferSlice {
            offset: aligned_offset,
            // SAFETY: effective_size >= MIN_BINDING_BYTES >= 16 > 0
            size: NonZeroU64::new(effective_size).expect("effective_size is non-zero"),
        })
    }

    /// Double the buffer capacity, replacing the underlying `wgpu::Buffer`.
    ///
    /// Returns [`GrowError::AtMax`] when the buffer is already at the device
    /// maximum.  On success the cursor is preserved so the caller can retry
    /// the write that triggered the overflow.
    ///
    /// After growing, any existing bind groups that reference the old buffer
    /// are invalid and must be recreated.
    pub fn grow(&mut self, device: &wgpu::Device) -> Result<(), GrowError> {
        let new_capacity = next_capacity(self.capacity, self.max_capacity)?;
        self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nom_gpui_instance_buffer"),
            size: new_capacity,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.capacity = new_capacity;
        Ok(())
    }

    /// Return a reference to the underlying `wgpu::Buffer`.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Return the currently allocated byte capacity.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

// ── Unit tests (no GPU required) ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── align_up ─────────────────────────────────────────────────────────────

    #[test]
    fn align_up_already_aligned() {
        assert_eq!(align_up(256, 256), 256);
        assert_eq!(align_up(512, 256), 512);
        assert_eq!(align_up(0, 256), 0);
    }

    #[test]
    fn align_up_rounds_up() {
        assert_eq!(align_up(1, 256), 256);
        assert_eq!(align_up(255, 256), 256);
        assert_eq!(align_up(257, 256), 512);
    }

    #[test]
    fn align_up_alignment_one_is_identity() {
        for v in [0u64, 1, 100, 999, u16::MAX as u64] {
            assert_eq!(align_up(v, 1), v);
        }
    }

    #[test]
    fn align_up_zero_alignment_is_identity() {
        assert_eq!(align_up(42, 0), 42);
    }

    // ── compute_write_slot ────────────────────────────────────────────────────

    #[test]
    fn write_slot_fits_exactly() {
        // capacity = 512, cursor = 0, write 512 bytes, alignment = 256
        let result = compute_write_slot(0, 512, 256, 512);
        assert_eq!(result, Some((0, 512)));
    }

    #[test]
    fn write_slot_fits_with_cursor_advance() {
        // cursor at 256, write 100 bytes (→ effective 100), capacity 1024
        let result = compute_write_slot(256, 100, 256, 1024);
        // offset = align_up(256, 256) = 256; end = 256 + 100 = 356
        assert_eq!(result, Some((256, 356)));
    }

    #[test]
    fn write_slot_applies_min_binding() {
        // data is only 8 bytes — must be clamped to 16
        let result = compute_write_slot(0, 8, 256, 1024);
        // offset = 0; end = 0 + 16 = 16
        assert_eq!(result, Some((0, 16)));
    }

    #[test]
    fn write_slot_returns_none_when_overflow() {
        // cursor at 500, data 100, capacity 512
        // align_up(500, 256) = 512; 512 + 100 = 612 > 512 → None
        let result = compute_write_slot(500, 100, 256, 512);
        assert!(result.is_none());
    }

    #[test]
    fn write_slot_returns_none_when_cursor_past_capacity() {
        let result = compute_write_slot(512, 1, 256, 512);
        assert!(result.is_none());
    }

    #[test]
    fn write_slot_zero_size_uses_min_binding() {
        // size 0 → clamped to 16
        let result = compute_write_slot(0, 0, 256, 1024);
        assert_eq!(result, Some((0, 16)));
    }

    // ── next_capacity ─────────────────────────────────────────────────────────

    #[test]
    fn next_capacity_doubles() {
        assert_eq!(next_capacity(1024, u64::MAX).unwrap(), 2048);
        assert_eq!(
            next_capacity(INITIAL_CAPACITY, u64::MAX).unwrap(),
            INITIAL_CAPACITY * 2
        );
    }

    #[test]
    fn next_capacity_clamps_to_max() {
        let max = 3000u64;
        assert_eq!(next_capacity(2000, max).unwrap(), max);
    }

    #[test]
    fn next_capacity_errors_when_at_max() {
        let max = 4096u64;
        assert!(matches!(next_capacity(max, max), Err(GrowError::AtMax(4096))));
    }

    #[test]
    fn next_capacity_errors_when_above_max() {
        // current > max is a degenerate state but must not panic
        let max = 1000u64;
        assert!(matches!(next_capacity(2000, max), Err(GrowError::AtMax(1000))));
    }
}
