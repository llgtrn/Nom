use crate::types::*;

/// Style values for layout and visual rendering.
/// Pattern: Zed StyleRefinement (macro-generated; this is the explicit version).
#[derive(Debug, Clone, Default)]
pub struct StyleRefinement {
    // Layout
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub padding: Option<Edges<Pixels>>,
    pub margin: Option<Edges<Pixels>>,
    // Visual
    pub background: Option<Hsla>,
    pub border_color: Option<Hsla>,
    pub border_widths: Option<Edges<Pixels>>,
    pub corner_radii: Option<Corners<Pixels>>,
    pub text_color: Option<Hsla>,
    pub opacity: Option<f32>,
    pub overflow_hidden: Option<bool>,
}

impl StyleRefinement {
    /// Merge `other` into `self`: only override fields that `other` has set.
    pub fn merge(&mut self, other: &StyleRefinement) {
        if other.width.is_some() {
            self.width = other.width;
        }
        if other.height.is_some() {
            self.height = other.height;
        }
        if other.min_width.is_some() {
            self.min_width = other.min_width;
        }
        if other.min_height.is_some() {
            self.min_height = other.min_height;
        }
        if other.flex_grow.is_some() {
            self.flex_grow = other.flex_grow;
        }
        if other.flex_shrink.is_some() {
            self.flex_shrink = other.flex_shrink;
        }
        if other.padding.is_some() {
            self.padding = other.padding;
        }
        if other.margin.is_some() {
            self.margin = other.margin;
        }
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color;
        }
        if other.border_widths.is_some() {
            self.border_widths = other.border_widths;
        }
        if other.corner_radii.is_some() {
            self.corner_radii = other.corner_radii;
        }
        if other.text_color.is_some() {
            self.text_color = other.text_color;
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.overflow_hidden.is_some() {
            self.overflow_hidden = other.overflow_hidden;
        }
    }
}

/// Fluent style builder trait.
/// Pattern: Zed Styled (APP/zed-main/crates/gpui/src/styled.rs)
pub trait Styled: Sized {
    fn style(&mut self) -> &mut StyleRefinement;

    fn bg(mut self, color: impl Into<Hsla>) -> Self {
        self.style().background = Some(color.into());
        self
    }

    fn border_color(mut self, color: impl Into<Hsla>) -> Self {
        self.style().border_color = Some(color.into());
        self
    }

    fn border(mut self, width: impl Into<Pixels>) -> Self {
        self.style().border_widths = Some(Edges::all(width.into()));
        self
    }

    fn rounded(mut self, radius: impl Into<Pixels>) -> Self {
        self.style().corner_radii = Some(Corners::all(radius.into()));
        self
    }

    fn p(mut self, pixels: impl Into<Pixels>) -> Self {
        self.style().padding = Some(Edges::all(pixels.into()));
        self
    }

    fn m(mut self, pixels: impl Into<Pixels>) -> Self {
        self.style().margin = Some(Edges::all(pixels.into()));
        self
    }

    fn text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.style().text_color = Some(color.into());
        self
    }

    fn opacity(mut self, v: f32) -> Self {
        self.style().opacity = Some(v.clamp(0.0, 1.0));
        self
    }

    fn overflow_hidden(mut self) -> Self {
        self.style().overflow_hidden = Some(true);
        self
    }

    fn w(mut self, width: impl Into<f32>) -> Self {
        self.style().width = Some(width.into());
        self
    }

    fn h(mut self, height: impl Into<f32>) -> Self {
        self.style().height = Some(height.into());
        self
    }

    fn flex_grow(mut self) -> Self {
        self.style().flex_grow = Some(1.0);
        self
    }

    /// Shadow is emitted as a drop-shadow primitive; this is a no-op marker.
    fn shadow(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bg_sets_background() {
        struct S {
            style: StyleRefinement,
        }
        impl Styled for S {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }
        let s = S {
            style: StyleRefinement::default(),
        }
        .bg(Hsla::new(120.0, 0.5, 0.5, 1.0));
        assert_eq!(s.style.background, Some(Hsla::new(120.0, 0.5, 0.5, 1.0)));
    }

    #[test]
    fn rounded_sets_corner_radii() {
        struct S {
            style: StyleRefinement,
        }
        impl Styled for S {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }
        let s = S {
            style: StyleRefinement::default(),
        }
        .rounded(Pixels(8.0));
        assert_eq!(s.style.corner_radii, Some(Corners::all(Pixels(8.0))));
    }

    #[test]
    fn merge_combines_two_refinements() {
        let mut base = StyleRefinement::default();
        base.opacity = Some(0.8);

        let mut patch = StyleRefinement::default();
        patch.background = Some(Hsla::white());
        patch.text_color = Some(Hsla::black());

        base.merge(&patch);

        assert_eq!(base.opacity, Some(0.8));
        assert_eq!(base.background, Some(Hsla::white()));
        assert_eq!(base.text_color, Some(Hsla::black()));
    }

    #[test]
    fn merge_does_not_override_with_none() {
        let mut base = StyleRefinement::default();
        base.background = Some(Hsla::white());

        let empty = StyleRefinement::default();
        base.merge(&empty);

        assert_eq!(base.background, Some(Hsla::white()));
    }

    // ---- New tests ----

    // Helper struct used across new tests
    struct S {
        style: StyleRefinement,
    }
    impl Styled for S {
        fn style(&mut self) -> &mut StyleRefinement {
            &mut self.style
        }
    }

    #[test]
    fn styled_default() {
        let s = StyleRefinement::default();
        assert!(s.background.is_none());
        assert!(s.border_widths.is_none());
        assert!(s.padding.is_none());
        assert!(s.margin.is_none());
        assert!(s.opacity.is_none());
    }

    #[test]
    fn styled_background() {
        let color = Hsla::new(200.0, 0.8, 0.5, 1.0);
        let s = S {
            style: StyleRefinement::default(),
        }
        .bg(color);
        assert_eq!(s.style.background, Some(color));
    }

    #[test]
    fn styled_border() {
        let s = S {
            style: StyleRefinement::default(),
        }
        .border(Pixels(2.0));
        let widths = s.style.border_widths.expect("border_widths should be set");
        assert_eq!(widths.top, Pixels(2.0));
        assert_eq!(widths.right, Pixels(2.0));
        assert_eq!(widths.bottom, Pixels(2.0));
        assert_eq!(widths.left, Pixels(2.0));
    }

    #[test]
    fn styled_padding() {
        let s = S {
            style: StyleRefinement::default(),
        }
        .p(Pixels(8.0));
        let padding = s.style.padding.expect("padding should be set");
        assert_eq!(padding.top, Pixels(8.0));
        assert_eq!(padding.left, Pixels(8.0));
    }

    #[test]
    fn styled_margin() {
        let s = S {
            style: StyleRefinement::default(),
        }
        .m(Pixels(4.0));
        let margin = s.style.margin.expect("margin should be set");
        assert_eq!(margin.top, Pixels(4.0));
        assert_eq!(margin.bottom, Pixels(4.0));
    }

    #[test]
    fn styled_chain() {
        let s = S {
            style: StyleRefinement::default(),
        }
        .bg(Hsla::black())
        .border(Pixels(1.0))
        .p(Pixels(10.0))
        .m(Pixels(5.0))
        .opacity(0.5);
        assert!(s.style.background.is_some());
        assert!(s.style.border_widths.is_some());
        assert!(s.style.padding.is_some());
        assert!(s.style.margin.is_some());
        assert!((s.style.opacity.unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn styled_override_previous() {
        let first = Hsla::new(0.0, 1.0, 0.5, 1.0);
        let second = Hsla::new(120.0, 1.0, 0.5, 1.0);
        let s = S {
            style: StyleRefinement::default(),
        }
        .bg(first)
        .bg(second);
        assert_eq!(s.style.background, Some(second));
    }

    #[test]
    fn styled_merge_second_wins_on_conflict() {
        let mut base = StyleRefinement::default();
        base.background = Some(Hsla::black());
        base.opacity = Some(0.9);

        let mut patch = StyleRefinement::default();
        patch.background = Some(Hsla::white());

        base.merge(&patch);

        // second wins on conflict
        assert_eq!(base.background, Some(Hsla::white()));
        // non-conflicting field preserved
        assert_eq!(base.opacity, Some(0.9));
    }

    #[test]
    fn styled_opacity_clamps_above_one() {
        let s = S { style: StyleRefinement::default() }.opacity(2.0);
        assert!((s.style.opacity.unwrap() - 1.0).abs() < 1e-6, "opacity above 1.0 must clamp to 1.0");
    }

    #[test]
    fn styled_opacity_clamps_below_zero() {
        let s = S { style: StyleRefinement::default() }.opacity(-0.5);
        assert!((s.style.opacity.unwrap() - 0.0).abs() < 1e-6, "opacity below 0.0 must clamp to 0.0");
    }

    #[test]
    fn styled_overflow_hidden_sets_flag() {
        let s = S { style: StyleRefinement::default() }.overflow_hidden();
        assert_eq!(s.style.overflow_hidden, Some(true));
    }

    #[test]
    fn styled_width_and_height() {
        let s = S { style: StyleRefinement::default() }.w(320.0_f32).h(240.0_f32);
        assert!((s.style.width.unwrap() - 320.0).abs() < 1e-5);
        assert!((s.style.height.unwrap() - 240.0).abs() < 1e-5);
    }

    #[test]
    fn styled_flex_grow_sets_one() {
        let s = S { style: StyleRefinement::default() }.flex_grow();
        assert!((s.style.flex_grow.unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn styled_shadow_is_passthrough() {
        // shadow() must return Self unchanged (marker only).
        let s = S {
            style: StyleRefinement::default(),
        }
        .bg(Hsla::black())
        .shadow();
        assert_eq!(s.style.background, Some(Hsla::black()));
    }

    #[test]
    fn styled_border_color() {
        let color = Hsla::new(0.0, 1.0, 0.5, 1.0);
        let s = S { style: StyleRefinement::default() }.border_color(color);
        assert_eq!(s.style.border_color, Some(color));
    }

    #[test]
    fn styled_text_color() {
        let color = Hsla::new(240.0, 0.5, 0.3, 0.9);
        let s = S { style: StyleRefinement::default() }.text_color(color);
        assert_eq!(s.style.text_color, Some(color));
    }

    #[test]
    fn merge_all_fields_propagate() {
        let mut base = StyleRefinement::default();
        let mut patch = StyleRefinement::default();
        patch.width = Some(100.0);
        patch.height = Some(200.0);
        patch.min_width = Some(50.0);
        patch.min_height = Some(60.0);
        patch.flex_grow = Some(1.0);
        patch.flex_shrink = Some(0.5);
        patch.overflow_hidden = Some(true);

        base.merge(&patch);

        assert_eq!(base.width, Some(100.0));
        assert_eq!(base.height, Some(200.0));
        assert_eq!(base.min_width, Some(50.0));
        assert_eq!(base.min_height, Some(60.0));
        assert_eq!(base.flex_grow, Some(1.0));
        assert_eq!(base.flex_shrink, Some(0.5));
        assert_eq!(base.overflow_hidden, Some(true));
    }

    #[test]
    fn merge_base_fields_preserved_when_patch_empty() {
        let mut base = StyleRefinement::default();
        base.width = Some(300.0);
        base.flex_shrink = Some(0.3);
        base.overflow_hidden = Some(false);

        let patch = StyleRefinement::default();
        base.merge(&patch);

        assert_eq!(base.width, Some(300.0));
        assert_eq!(base.flex_shrink, Some(0.3));
        assert_eq!(base.overflow_hidden, Some(false));
    }

    #[test]
    fn styled_chain_all_visual_properties() {
        let s = S { style: StyleRefinement::default() }
            .bg(Hsla::white())
            .border_color(Hsla::black())
            .border(Pixels(1.0))
            .rounded(Pixels(4.0))
            .text_color(Hsla::new(0.0, 0.0, 0.5, 1.0))
            .opacity(0.8)
            .overflow_hidden();
        assert!(s.style.background.is_some());
        assert!(s.style.border_color.is_some());
        assert!(s.style.border_widths.is_some());
        assert!(s.style.corner_radii.is_some());
        assert!(s.style.text_color.is_some());
        assert!((s.style.opacity.unwrap() - 0.8).abs() < 1e-6);
        assert_eq!(s.style.overflow_hidden, Some(true));
    }

    #[test]
    fn styled_reset_field_via_merge() {
        // A field can be "reset" by merging a patch that sets a default-like value.
        let mut base = StyleRefinement::default();
        base.opacity = Some(0.3);

        let mut patch = StyleRefinement::default();
        patch.opacity = Some(1.0);  // reset to opaque

        base.merge(&patch);
        assert!((base.opacity.unwrap() - 1.0).abs() < 1e-6);
    }
}
