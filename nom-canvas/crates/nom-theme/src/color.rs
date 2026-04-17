/// HSLA color value. All components are in [0.0, 1.0] range.
/// Hue is normalized: 0.0 = 0°, 1.0 = 360°.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl Hsla {
    /// Construct from individual components.
    #[inline]
    pub const fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    /// Construct from a pre-packed `(h, s, l, a)` tuple (as used by `tokens`).
    #[inline]
    pub const fn from_tuple(t: (f32, f32, f32, f32)) -> Self {
        Self {
            h: t.0,
            s: t.1,
            l: t.2,
            a: t.3,
        }
    }

    /// Convert to linear RGBA. Returns `[r, g, b, a]` with components in
    /// [0.0, 1.0]. This is the standard HSLA → RGBA algorithm.
    pub fn to_rgba(self) -> [f32; 4] {
        let h = self.h;
        let s = self.s;
        let l = self.l;

        if s == 0.0 {
            return [l, l, l, self.a];
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let r = hue_to_channel(p, q, h + 1.0 / 3.0);
        let g = hue_to_channel(p, q, h);
        let b = hue_to_channel(p, q, h - 1.0 / 3.0);

        [r, g, b, self.a]
    }

    /// Alpha-premultiplied RGBA. Returns `[r*a, g*a, b*a, a]`.
    pub fn premultiplied(self) -> [f32; 4] {
        let [r, g, b, a] = self.to_rgba();
        [r * a, g * a, b * a, a]
    }
}

fn hue_to_channel(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn rgba_approx(got: [f32; 4], expected: [f32; 4]) -> bool {
        got.iter()
            .zip(expected.iter())
            .all(|(a, b)| approx_eq(*a, *b))
    }

    #[test]
    fn white_to_rgba() {
        let c = Hsla::new(0.0, 0.0, 1.0, 1.0);
        assert!(rgba_approx(c.to_rgba(), [1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn black_to_rgba() {
        let c = Hsla::new(0.0, 0.0, 0.0, 1.0);
        assert!(rgba_approx(c.to_rgba(), [0.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn pure_red_to_rgba() {
        // Hue = 0° → 0.0, saturation = 1.0, lightness = 0.5
        let c = Hsla::new(0.0, 1.0, 0.5, 1.0);
        assert!(rgba_approx(c.to_rgba(), [1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn premultiplied_halves_rgb_at_half_alpha() {
        let c = Hsla::new(0.0, 1.0, 0.5, 0.5); // pure red, α=0.5
        let pm = c.premultiplied();
        // r * 0.5 ≈ 0.5, g * 0.5 = 0.0, b * 0.5 = 0.0, a = 0.5
        assert!(rgba_approx(pm, [0.5, 0.0, 0.0, 0.5]));
    }
}
