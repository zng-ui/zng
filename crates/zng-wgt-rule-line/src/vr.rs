//! Vertical rule line.

use zng_wgt::prelude::*;

/// Draws a vertical [`RuleLine!`](struct@crate::RuleLine).
#[widget($crate::vr::Vr)]
pub struct Vr(super::RuleLine);
impl Vr {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            orientation = LineOrientation::Vertical;
            color = COLOR_VAR;
            stroke_thickness = STROKE_THICKNESS_VAR;
            line_style = LINE_STYLE_VAR;
            margin = MARGIN_VAR;
            length = HEIGHT_VAR;
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
    /// Is `(0, 4)` by default, 0 top-bottom, 4 left-right.
    pub static MARGIN_VAR: SideOffsets = (0, 4);

    /// Vertical line length.
    ///
    /// Is `Default` by default, that fills height.
    pub static HEIGHT_VAR: Length = Length::Default;
}

/// Sets the line color of all descendant `Vr!()`.
///
/// The default is the `FONT_COLOR_VAR` with 30% alpha.
///
/// This property sets the [`COLOR_VAR`].
#[property(CONTEXT, default(COLOR_VAR))]
pub fn color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
    with_context_var(child, COLOR_VAR, color)
}

/// Sets the line stroke thickness of all descendant `Vr!()`.
///
/// The default is `1.dip()`.
///
/// This property sets the [`STROKE_THICKNESS_VAR`].
#[property(CONTEXT, default(STROKE_THICKNESS_VAR))]
pub fn stroke_thickness(child: impl IntoUiNode, thickness: impl IntoVar<Length>) -> UiNode {
    with_context_var(child, STROKE_THICKNESS_VAR, thickness)
}

/// Sets the line style of all descendant `Vr!()`.
///
/// The default is `Solid`.
///
/// This property sets the [`LINE_STYLE_VAR`].
#[property(CONTEXT, default(LINE_STYLE_VAR))]
pub fn line_style(child: impl IntoUiNode, style: impl IntoVar<LineStyle>) -> UiNode {
    with_context_var(child, LINE_STYLE_VAR, style)
}

/// Sets the margin around line of all descendant `Vr!()`.
///
/// Is `(0, 4)` by default, 0 top-bottom, 4 left-right.
///
/// This property sets the [`HEIGHT_VAR`].
#[property(CONTEXT, default(MARGIN_VAR))]
pub fn margin(child: impl IntoUiNode, margin: impl IntoVar<SideOffsets>) -> UiNode {
    with_context_var(child, MARGIN_VAR, margin)
}

/// Sets the vertical line length of all descendant `Vr!()`.
///
/// Is `Default` by default, that fills the height or collapses if not aligned to fill.
///
/// This property sets the [`HEIGHT_VAR`].
#[property(CONTEXT, default(HEIGHT_VAR))]
pub fn height(child: impl IntoUiNode, height: impl IntoVar<Length>) -> UiNode {
    with_context_var(child, HEIGHT_VAR, height)
}
