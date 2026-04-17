//! Fluent style builder trait. Any element that exposes `&mut Style` gets
//! the full API automatically: `.flex_col().padding(8.0).bg(colors::BG)`.

use crate::color::Rgba;
use crate::geometry::{Corners, Edges, Pixels};
use crate::style::{
    AlignItems, Display, FlexDirection, JustifyContent, Length, Overflow, Style,
};

/// Marker trait for types that own a [`Style`] value. Provides fluent setters.
///
/// Any UI element (see [`crate::element::Element`]) should implement this so
/// that callers can chain CSS-like method calls.
pub trait Styled: Sized {
    fn style(&mut self) -> &mut Style;

    // ─── Display / overflow ─────────────────────────────────────────────
    fn display(mut self, d: Display) -> Self {
        self.style().display = d;
        self
    }
    fn block(mut self) -> Self {
        self.style().display = Display::Block;
        self
    }
    fn flex(mut self) -> Self {
        self.style().display = Display::Flex;
        self
    }
    fn hidden(mut self) -> Self {
        self.style().display = Display::None;
        self
    }
    fn overflow(mut self, o: Overflow) -> Self {
        self.style().overflow_x = o;
        self.style().overflow_y = o;
        self
    }
    fn overflow_hidden(self) -> Self {
        self.overflow(Overflow::Hidden)
    }

    // ─── Flex layout ────────────────────────────────────────────────────
    fn flex_direction(mut self, d: FlexDirection) -> Self {
        self.style().flex_direction = d;
        self
    }
    fn flex_row(self) -> Self {
        self.flex_direction(FlexDirection::Row)
    }
    fn flex_col(self) -> Self {
        self.flex_direction(FlexDirection::Column)
    }
    fn flex_grow(mut self, g: f32) -> Self {
        self.style().flex_grow = g;
        self
    }
    fn flex_shrink(mut self, s: f32) -> Self {
        self.style().flex_shrink = s;
        self
    }
    fn flex_1(self) -> Self {
        self.flex_grow(1.0).flex_shrink(1.0)
    }
    fn gap(mut self, v: f32) -> Self {
        let p = Pixels(v);
        self.style().gap_row = p;
        self.style().gap_col = p;
        self
    }
    fn align_items(mut self, a: AlignItems) -> Self {
        self.style().align_items = a;
        self
    }
    fn items_center(self) -> Self {
        self.align_items(AlignItems::Center)
    }
    fn justify_content(mut self, j: JustifyContent) -> Self {
        self.style().justify_content = j;
        self
    }
    fn justify_center(self) -> Self {
        self.justify_content(JustifyContent::Center)
    }
    fn justify_between(self) -> Self {
        self.justify_content(JustifyContent::SpaceBetween)
    }

    // ─── Size ───────────────────────────────────────────────────────────
    fn w(mut self, v: f32) -> Self {
        self.style().width = Length::Pixels(Pixels(v));
        self
    }
    fn h(mut self, v: f32) -> Self {
        self.style().height = Length::Pixels(Pixels(v));
        self
    }
    fn w_full(mut self) -> Self {
        self.style().width = Length::Percent(1.0);
        self
    }
    fn h_full(mut self) -> Self {
        self.style().height = Length::Percent(1.0);
        self
    }
    fn min_w(mut self, v: f32) -> Self {
        self.style().min_width = Length::Pixels(Pixels(v));
        self
    }
    fn max_w(mut self, v: f32) -> Self {
        self.style().max_width = Length::Pixels(Pixels(v));
        self
    }

    // ─── Spacing ────────────────────────────────────────────────────────
    fn padding(mut self, v: f32) -> Self {
        self.style().padding = Edges::all(Pixels(v));
        self
    }
    fn px(mut self, v: f32) -> Self {
        self.style().padding.left = Pixels(v);
        self.style().padding.right = Pixels(v);
        self
    }
    fn py(mut self, v: f32) -> Self {
        self.style().padding.top = Pixels(v);
        self.style().padding.bottom = Pixels(v);
        self
    }
    fn margin(mut self, v: f32) -> Self {
        self.style().margin = Edges::all(Pixels(v));
        self
    }

    // ─── Paint ──────────────────────────────────────────────────────────
    fn bg(mut self, color: Rgba) -> Self {
        self.style().background = Some(color);
        self
    }
    fn text_color(mut self, color: Rgba) -> Self {
        self.style().text_color = Some(color);
        self
    }
    fn font_size(mut self, v: f32) -> Self {
        self.style().font_size = Some(Pixels(v));
        self
    }
    fn border(mut self, width: f32, color: Rgba) -> Self {
        self.style().border_widths = Edges::all(Pixels(width));
        self.style().border_color = Some(color);
        self
    }
    fn rounded(mut self, radius: f32) -> Self {
        self.style().corner_radii = Corners::all(Pixels(radius));
        self
    }
    fn opacity(mut self, v: f32) -> Self {
        self.style().opacity = Some(v.clamp(0.0, 1.0));
        self
    }
}

/// Plain owner of a Style that any test can use directly.
#[derive(Clone, Copy, Debug, Default)]
pub struct StyledBox {
    pub style: Style,
}

impl Styled for StyledBox {
    fn style(&mut self) -> &mut Style {
        &mut self.style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chained_calls_accumulate() {
        let b = StyledBox::default()
            .flex_col()
            .w(100.0)
            .h(50.0)
            .padding(8.0)
            .bg(Rgba::WHITE)
            .rounded(4.0);
        assert_eq!(b.style.flex_direction, FlexDirection::Column);
        assert_eq!(b.style.width, Length::Pixels(Pixels(100.0)));
        assert_eq!(b.style.padding.top, Pixels(8.0));
        assert_eq!(b.style.background, Some(Rgba::WHITE));
        assert_eq!(b.style.corner_radii.top_left, Pixels(4.0));
    }

    #[test]
    fn opacity_clamps_to_unit_range() {
        let b = StyledBox::default().opacity(1.5);
        assert_eq!(b.style.opacity, Some(1.0));
        let b = StyledBox::default().opacity(-0.2);
        assert_eq!(b.style.opacity, Some(0.0));
    }
}
