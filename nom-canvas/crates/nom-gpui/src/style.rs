//! Layout style: the Nom equivalent of Zed's `Style`. Mirrors taffy fields so
//! a `Style` value can be converted to `taffy::Style` for layout computation.

use crate::color::Rgba;
use crate::geometry::{Corners, Edges, Pixels};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Flex,
    Block,
    None,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignItems {
    #[default]
    Stretch,
    Start,
    End,
    Center,
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Auto,
    Pixels(Pixels),
    Percent(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Auto
    }
}

/// Full layout + paint style. Mutated via [`Styled`](crate::styled::Styled) fluent methods.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    // Box model
    pub display: Display,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    pub padding: Edges<Pixels>,
    pub margin: Edges<Pixels>,
    pub border_widths: Edges<Pixels>,

    // Flex
    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,
    pub gap_row: Pixels,
    pub gap_col: Pixels,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,

    // Paint
    pub background: Option<Rgba>,
    pub border_color: Option<Rgba>,
    pub corner_radii: Corners<Pixels>,
    pub text_color: Option<Rgba>,
    pub font_size: Option<Pixels>,
    pub opacity: Option<f32>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Convert a `Length` to `taffy::Dimension` using the given rem size.
pub fn to_taffy_dimension(len: Length) -> taffy::Dimension {
    match len {
        Length::Auto => taffy::Dimension::Auto,
        Length::Pixels(p) => taffy::Dimension::Length(p.0),
        Length::Percent(f) => taffy::Dimension::Percent(f),
    }
}

pub fn to_taffy_size(w: Length, h: Length) -> taffy::Size<taffy::Dimension> {
    taffy::Size {
        width: to_taffy_dimension(w),
        height: to_taffy_dimension(h),
    }
}

pub fn to_taffy_rect(e: Edges<Pixels>) -> taffy::Rect<taffy::LengthPercentage> {
    taffy::Rect {
        top: taffy::LengthPercentage::Length(e.top.0),
        right: taffy::LengthPercentage::Length(e.right.0),
        bottom: taffy::LengthPercentage::Length(e.bottom.0),
        left: taffy::LengthPercentage::Length(e.left.0),
    }
}

impl Style {
    /// Translate self into a `taffy::Style` for layout computation.
    pub fn to_taffy(&self) -> taffy::Style {
        // Use fully qualified paths on the left side of match arms so our local
        // Display / FlexDirection / etc. aren't shadowed by taffy's identically
        // named types.
        taffy::Style {
            display: match self.display {
                self::Display::Flex => taffy::Display::Flex,
                self::Display::Block => taffy::Display::Block,
                self::Display::None => taffy::Display::None,
            },
            overflow: taffy::Point {
                x: overflow_to_taffy(self.overflow_x),
                y: overflow_to_taffy(self.overflow_y),
            },
            size: to_taffy_size(self.width, self.height),
            min_size: to_taffy_size(self.min_width, self.min_height),
            max_size: to_taffy_size(self.max_width, self.max_height),
            padding: to_taffy_rect(self.padding),
            margin: taffy::Rect {
                top: taffy::LengthPercentageAuto::Length(self.margin.top.0),
                right: taffy::LengthPercentageAuto::Length(self.margin.right.0),
                bottom: taffy::LengthPercentageAuto::Length(self.margin.bottom.0),
                left: taffy::LengthPercentageAuto::Length(self.margin.left.0),
            },
            border: to_taffy_rect(self.border_widths),
            flex_direction: match self.flex_direction {
                self::FlexDirection::Row => taffy::FlexDirection::Row,
                self::FlexDirection::Column => taffy::FlexDirection::Column,
                self::FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                self::FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: to_taffy_dimension(self.flex_basis),
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(self.gap_col.0),
                height: taffy::LengthPercentage::Length(self.gap_row.0),
            },
            // taffy 0.6: AlignItems/JustifyContent are aliases to AlignContent.
            align_items: Some(match self.align_items {
                self::AlignItems::Stretch => taffy::AlignItems::Stretch,
                self::AlignItems::Start => taffy::AlignItems::FlexStart,
                self::AlignItems::End => taffy::AlignItems::FlexEnd,
                self::AlignItems::Center => taffy::AlignItems::Center,
                self::AlignItems::Baseline => taffy::AlignItems::Baseline,
            }),
            justify_content: Some(match self.justify_content {
                self::JustifyContent::Start => taffy::JustifyContent::FlexStart,
                self::JustifyContent::End => taffy::JustifyContent::FlexEnd,
                self::JustifyContent::Center => taffy::JustifyContent::Center,
                self::JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
                self::JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
                self::JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
            }),
            ..Default::default()
        }
    }
}

fn overflow_to_taffy(o: Overflow) -> taffy::Overflow {
    match o {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Hidden => taffy::Overflow::Hidden,
        Overflow::Scroll => taffy::Overflow::Scroll,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style_converts_to_taffy_flex_row() {
        let s = Style::default();
        let t = s.to_taffy();
        assert_eq!(t.display, taffy::Display::Flex);
        assert_eq!(t.flex_direction, taffy::FlexDirection::Row);
    }

    #[test]
    fn flex_column_translates() {
        let s = Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        };
        let t = s.to_taffy();
        assert_eq!(t.flex_direction, taffy::FlexDirection::Column);
    }

    #[test]
    fn width_pixels_translates() {
        let s = Style {
            width: Length::Pixels(Pixels(128.0)),
            ..Default::default()
        };
        let t = s.to_taffy();
        assert_eq!(t.size.width, taffy::Dimension::Length(128.0));
    }
}
