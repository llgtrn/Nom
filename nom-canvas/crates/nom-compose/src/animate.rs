#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtrapolateMode {
    Clamp,
    Extend,
    Identity,
    Wrap,
}

/// Map `frame` from input_range to output_range with optional easing and extrapolation.
pub fn interpolate(
    frame: f32,
    input_range: (f32, f32),
    output_range: (f32, f32),
    easing: Option<fn(f32) -> f32>,
    extrapolate_left: ExtrapolateMode,
    extrapolate_right: ExtrapolateMode,
) -> f32 {
    let (in_lo, in_hi) = input_range;
    let (out_lo, out_hi) = output_range;

    let clamped = if frame < in_lo {
        match extrapolate_left {
            ExtrapolateMode::Clamp => in_lo,
            ExtrapolateMode::Extend => frame,
            ExtrapolateMode::Identity => return frame,
            ExtrapolateMode::Wrap => in_lo + ((frame - in_lo).rem_euclid(in_hi - in_lo)),
        }
    } else if frame > in_hi {
        match extrapolate_right {
            ExtrapolateMode::Clamp => in_hi,
            ExtrapolateMode::Extend => frame,
            ExtrapolateMode::Identity => return frame,
            ExtrapolateMode::Wrap => in_lo + ((frame - in_lo).rem_euclid(in_hi - in_lo)),
        }
    } else {
        frame
    };

    let t = (clamped - in_lo) / (in_hi - in_lo);
    let eased_t = if let Some(ease) = easing { ease(t) } else { t };
    out_lo + eased_t * (out_hi - out_lo)
}

#[derive(Debug, Clone, Copy)]
pub struct SpringConfig {
    pub damping: f32,
    pub mass: f32,
    pub tension: f32,
    pub overshoot_clamping: bool,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            damping: 28.0,
            mass: 1.0,
            tension: 170.0,
            overshoot_clamping: false,
        }
    }
}

/// Compute spring physics value at `frame`.
/// Returns value between `from` and `to` (with possible overshoot unless clamped).
pub fn spring(frame: u32, fps: u32, config: SpringConfig, from: f32, to: f32) -> f32 {
    if fps == 0 {
        return from;
    }
    let t = frame as f32 / fps as f32;
    let omega = (config.tension / config.mass).sqrt();
    let zeta = config.damping / (2.0 * (config.tension * config.mass).sqrt());

    let value = if zeta < 1.0 {
        // Underdamped
        let omega_d = omega * (1.0 - zeta * zeta).sqrt();
        let envelope = (-zeta * omega * t).exp();
        let oscillation = (omega_d * t).cos() + (zeta * omega / omega_d) * (omega_d * t).sin();
        to - (to - from) * envelope * oscillation
    } else {
        // Critically or overdamped
        let envelope = (-omega * t).exp();
        to - (to - from) * envelope * (1.0 + omega * t)
    };

    if config.overshoot_clamping {
        value.clamp(from.min(to), from.max(to))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_linear_midpoint() {
        // midpoint of [0,10] input → [0,100] output should be 50
        let result = interpolate(
            5.0,
            (0.0, 10.0),
            (0.0, 100.0),
            None,
            ExtrapolateMode::Clamp,
            ExtrapolateMode::Clamp,
        );
        assert!(
            (result - 50.0).abs() < 1e-4,
            "midpoint must be 50, got {result}"
        );
    }

    #[test]
    fn test_interpolate_clamp_below_range() {
        // frame -5 with Clamp left → clamped to in_lo=0 → output 0
        let result = interpolate(
            -5.0,
            (0.0, 10.0),
            (0.0, 100.0),
            None,
            ExtrapolateMode::Clamp,
            ExtrapolateMode::Clamp,
        );
        assert!(
            (result - 0.0).abs() < 1e-4,
            "clamped below must give out_lo=0, got {result}"
        );
    }

    #[test]
    fn test_interpolate_extend_above_range() {
        // frame 15 with Extend right → clamped = 15, t = (15-0)/(10-0) = 1.5 → output = 150
        let result = interpolate(
            15.0,
            (0.0, 10.0),
            (0.0, 100.0),
            None,
            ExtrapolateMode::Clamp,
            ExtrapolateMode::Extend,
        );
        assert!(
            (result - 150.0).abs() < 1e-3,
            "extend above must extrapolate linearly, got {result}"
        );
    }

    #[test]
    fn test_spring_at_frame_zero_returns_from() {
        let cfg = SpringConfig::default();
        let value = spring(0, 30, cfg, 0.0, 1.0);
        // at t=0, envelope=1, oscillation=cos(0)+...=1, value = to - (to-from)*1*1 = from
        assert!(
            (value - 0.0).abs() < 1e-4,
            "spring at frame 0 must equal from=0.0, got {value}"
        );
    }

    #[test]
    fn test_spring_large_frame_approaches_to() {
        let cfg = SpringConfig::default();
        // at a large frame number the spring should have settled near `to`
        let value = spring(300, 30, cfg, 0.0, 1.0);
        assert!(
            (value - 1.0).abs() < 0.01,
            "spring at large frame must approach to=1.0, got {value}"
        );
    }
}
