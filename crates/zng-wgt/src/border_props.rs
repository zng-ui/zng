use zng_app::widget::border::{BORDER, BORDER_ALIGN_VAR, BORDER_OVER_VAR, CORNER_RADIUS_FIT_VAR, CORNER_RADIUS_VAR};

use crate::prelude::*;

/// Corner radius of widget and inner widgets.
///
/// The [`Default`] value is calculated to fit inside the parent widget corner curve, see [`corner_radius_fit`] for more details.
///
/// [`Default`]: zng_layout::unit::Length::Default
/// [`corner_radius_fit`]: fn@corner_radius_fit
#[property(CONTEXT, default(CORNER_RADIUS_VAR))]
pub fn corner_radius(child: impl IntoUiNode, radius: impl IntoVar<CornerRadius>) -> UiNode {
    let child = match_node(child, move |child, op| {
        if let UiNodeOp::Layout { wl, final_size } = op {
            *final_size = BORDER.with_corner_radius(|| child.layout(wl));
        }
    });
    with_context_var(child, CORNER_RADIUS_VAR, radius)
}

/// Defines how the [`corner_radius`] is computed for each usage.
///
/// Nesting borders with round corners need slightly different radius values to perfectly fit, the [`BORDER`]
/// coordinator can adjusts the radius inside each border to match the inside curve of the border.
///
/// Sets the [`CORNER_RADIUS_FIT_VAR`].
///
/// [`corner_radius`]: fn@corner_radius
/// [`BORDER`]: zng_app::widget::border::BORDER
/// [`CORNER_RADIUS_FIT_VAR`]: zng_app::widget::border::CORNER_RADIUS_FIT_VAR
#[property(CONTEXT, default(CORNER_RADIUS_FIT_VAR))]
pub fn corner_radius_fit(child: impl IntoUiNode, fit: impl IntoVar<CornerRadiusFit>) -> UiNode {
    with_context_var(child, CORNER_RADIUS_FIT_VAR, fit)
}

/// Position of a widget borders in relation to the widget fill.
///
/// This property defines how much the widget's border offsets affect the layout of the fill content, by default
/// (0%) the fill content stretchers *under* the borders and is clipped by the [`corner_radius`], in the other end
/// of the scale (100%), the fill content is positioned *inside* the borders and clipped by the adjusted [`corner_radius`]
/// that fits the insider of the inner most border.
///
/// Note that widget's child is always inside the borders, this property only affects the fill properties, like the background.
///
/// Fill property implementers, see [`fill_node`], a helper function for quickly implementing support for `border_align`.
///
/// Sets the [`BORDER_ALIGN_VAR`].
///
/// [`corner_radius`]: fn@corner_radius
/// [`BORDER_ALIGN_VAR`]: zng_app::widget::border::BORDER_ALIGN_VAR
#[property(CONTEXT, default(BORDER_ALIGN_VAR))]
pub fn border_align(child: impl IntoUiNode, align: impl IntoVar<FactorSideOffsets>) -> UiNode {
    with_context_var(child, BORDER_ALIGN_VAR, align)
}

/// If the border is rendered over the fill and child visuals.
///
/// Is `true` by default, if set to `false` the borders will render under the fill. Note that
/// this means the border will be occluded by the background if [`border_align`] is not set to `1.fct()`.
///
/// Sets the [`BORDER_OVER_VAR`].
///
/// [`border_align`]: fn@border_align
/// [`BORDER_OVER_VAR`]: zng_app::widget::border::BORDER_OVER_VAR
#[property(CONTEXT, default(BORDER_OVER_VAR))]
pub fn border_over(child: impl IntoUiNode, over: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, BORDER_OVER_VAR, over)
}

/// Border widths, color and style.
///
/// This property coordinates with any other border in the widget to fit inside or outside the
/// other borders, it also works with the [`corner_radius`] property drawing round corners if configured.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// A border of width `1.dip()`, solid color `BLUE` in all border sides and corner radius `4.dip()`.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::Wgt;
/// # use zng_wgt::{corner_radius, border};
/// # use zng_color::colors;
/// # fn main() {
/// # let _ =
/// Wgt! {
///     border = 1, colors::BLUE;
///     corner_radius = 4;
/// }
/// # ; }
/// ```
///
/// A border that sets each border line to a different width `top: 1, right: 2, bottom: 3, left: 4`, each corner
/// radius to a different size `top_left: 1x1, top_right: 2x2, bottom_right: 3x3, bottom_left: 4x4` and each border
/// line to a different style and color.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::Wgt;
/// # use zng_wgt::{corner_radius, border};
/// # use zng_app::widget::border::{BorderSide, BorderSides};
/// # use zng_color::colors;
/// # fn main() {
/// # let _ =
/// Wgt! {
///     border = {
///         widths: (1, 2, 3, 4),
///         sides: BorderSides::new(
///             BorderSide::solid(colors::RED),
///             BorderSide::dashed(colors::GREEN),
///             BorderSide::dotted(colors::BLUE),
///             BorderSide::double(colors::YELLOW),
///         ),
///     };
///     corner_radius = (1, 2, 3, 4);
/// }
/// # ; }
/// ```
///
/// ## Multiple Borders
///
/// The border fits in with other borders in the widget, in this example we declare a
/// new border property by copying the signature of this one:
///
/// ```
/// # use zng_wgt::prelude::*;
/// #
/// /// Another border property.
/// #[property(BORDER, default(0, BorderStyle::Hidden))]
/// pub fn my_border(
///     child: impl IntoUiNode,
///     widths: impl IntoVar<SideOffsets>,
///     sides: impl IntoVar<BorderSides>
/// ) -> UiNode {
///     zng_wgt::border(child, widths, sides)
/// }
/// #
/// # fn main() { }
/// ```
///
/// Now we can set two borders in the same widget:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::Wgt;
/// # use zng_wgt::{corner_radius, border};
/// # use zng_color::colors;
/// # use zng_wgt::prelude::*;
/// #
/// # #[property(BORDER, default(0, BorderStyle::Hidden))]
/// # pub fn my_border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
/// #    zng_wgt::border(child, widths, sides)
/// # }
/// #
/// # fn main() {
/// # let _ =
/// Wgt! {
///     border = 4, colors::RED;
///     my_border = 4, colors::GREEN;
///     corner_radius = 8;
/// }
/// # ; }
/// ```
///
/// This will render a `RED` border around a `GREEN` one, the inner border will fit perfectly inside the outer one,
/// the `corner_radius` defines the outer radius, the inner radius is computed automatically to fit.
///
/// Note that because both borders have the same [`NestGroup::BORDER`] the position they are declared in the widget matters:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::Wgt;
/// # use zng_wgt::{corner_radius, border};
/// # use zng_color::colors;
/// # use zng_wgt::prelude::*;
/// #
/// # #[property(BORDER, default(0, BorderStyle::Hidden))]
/// # pub fn my_border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
/// #    zng_wgt::border(child, widths, sides)
/// # }
/// #
/// # fn main() {
/// # let _ =
/// Wgt! {
///     my_border = 4, colors::GREEN;
///     border = 4, colors::RED;
///     corner_radius = 8;
/// }
/// # ; }
/// ```
///
/// Now the `GREEN` border is around the `RED`.
///
/// You can adjust the nest group to cause a border to always be outside or inside:
///
/// ```
/// # use zng_wgt::prelude::*;
/// #
/// /// Border that is always around the other borders.
/// #[property(BORDER-1, default(0, BorderStyle::Hidden))]
/// pub fn outside_border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
///     zng_wgt::border(child, widths, sides)
/// }
///  
/// /// Border that is always inside the other borders.
/// #[property(BORDER+1, default(0, BorderStyle::Hidden))]
/// pub fn inside_border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
///     zng_wgt::border(child, widths, sides)
/// }
/// #
/// # fn main() { }
/// ```
///
/// [`corner_radius`]: fn@corner_radius
/// [`NestGroup::BORDER`]: zng_app::widget::builder::NestGroup::BORDER
#[property(BORDER, default(0, BorderStyle::Hidden))]
pub fn border(child: impl IntoUiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> UiNode {
    let sides = sides.into_var();
    let mut corners = PxCornerRadius::zero();

    border_node(
        child,
        widths,
        match_node_leaf(move |op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_var_render(&sides);
            }
            UiNodeOp::Measure { desired_size, .. } => {
                *desired_size = LAYOUT.constraints().fill_size();
            }
            UiNodeOp::Layout { final_size, .. } => {
                corners = BORDER.border_radius();
                *final_size = LAYOUT.constraints().fill_size();
            }
            UiNodeOp::Render { frame } => {
                let (rect, offsets) = BORDER.border_layout();
                if !rect.size.is_empty() {
                    frame.push_border(rect, offsets, sides.get(), corners);
                }
            }
            _ => {}
        }),
    )
}
