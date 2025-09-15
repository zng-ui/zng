//! Horizontal rule line.

use zng_wgt::prelude::*;

/// Draws an horizontal [`RuleLine!`](struct@crate::RuleLine).
#[widget($crate::hr::Hr)]
pub struct Hr(super::RuleLine);
impl Hr {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            orientation = LineOrientation::Horizontal;
            color = COLOR_VAR;
            stroke_thickness = STROKE_THICKNESS_VAR;
            line_style = LINE_STYLE_VAR;
            margin = MARGIN_VAR;
            length = WIDTH_VAR;
        }
    }
}

context_var! {
    /// Line color, inherits from [`FONT_COLOR_VAR`].
    ///
    /// [`FONT_COLOR_VAR`]: zng_wgt_text::FONT_COLOR_VAR
    pub static COLOR_VAR: Rgba = zng_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(30.pct()));

    /// Line stroke thickness, default is `1.dip()`
    pub static STROKE_THICKNESS_VAR: Length = 1.dip();

    /// Line style, default is `Solid`.
    pub static LINE_STYLE_VAR: LineStyle = LineStyle::Solid;

    /// Margin around line.
    ///
    /// Is `(4, 0)` by default, 4 top-bottom, 0 left-right.
    pub static MARGIN_VAR: SideOffsets = (4, 0);

    /// Horizontal line length.
    ///
    /// Is `Default` by default, that fills width.
    pub static WIDTH_VAR: Length = Length::Default;
}

/// Sets the line color of all descendant `Hr!()`.
///
/// The default is the `FONT_COLOR_VAR` with 30% alpha.
///
/// This property sets the [`COLOR_VAR`].
#[property(CONTEXT, default(COLOR_VAR))]
pub fn color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
    with_context_var(child, COLOR_VAR, color)
}

/// Sets the line stroke thickness of all descendant `Hr!()`.
///
/// The default is `1.dip()`.
///
/// This property sets the [`STROKE_THICKNESS_VAR`].
#[property(CONTEXT, default(STROKE_THICKNESS_VAR))]
pub fn stroke_thickness(child: impl IntoUiNode, thickness: impl IntoVar<Length>) -> UiNode {
    with_context_var(child, STROKE_THICKNESS_VAR, thickness)
}

/// Sets the line style of all descendant `Hr!()`.
///
/// The default is `Solid`.
///
/// This property sets the [`LINE_STYLE_VAR`].
#[property(CONTEXT, default(LINE_STYLE_VAR))]
pub fn line_style(child: impl IntoUiNode, style: impl IntoVar<LineStyle>) -> UiNode {
    with_context_var(child, LINE_STYLE_VAR, style)
}

/// Sets the margin around line of all descendant `Hr!()`.
///
/// Is `(4, 0)` by default, 4 top-bottom, 0 left-right.
///
/// This property sets the [`MARGIN_VAR`].
#[property(CONTEXT, default(MARGIN_VAR))]
pub fn margin(child: impl IntoUiNode, margin: impl IntoVar<SideOffsets>) -> UiNode {
    with_context_var(child, MARGIN_VAR, margin)
}

/// Sets the horizontal line length of all descendant `Hr!()`.
///
/// Is `Default` by default, that fills the width or collapses if not aligned to fill.
///
/// This property sets the [`WIDTH_VAR`].
#[property(CONTEXT, default(WIDTH_VAR))]
pub fn width(child: impl IntoUiNode, width: impl IntoVar<Length>) -> UiNode {
    with_context_var(child, WIDTH_VAR, width)
}
