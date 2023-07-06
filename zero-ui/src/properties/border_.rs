use crate::prelude::new_property::*;

pub use crate::core::border::{border_align, corner_radius, corner_radius_fit, CornerRadiusFit};
use crate::core::border::{border_node, BORDER};

/// Widget border.
///
/// Defines a widget border, it coordinates with any other border in the widget to fit inside or outside the
/// other borders, it also works with the [`corner_radius`] property drawing round corners if configured.
///
/// This property disables inline layout for the widget.
///
/// # Examples
///
/// A border of width `1.dip()`, solid color `BLUE` in all border sides and corner radius `4.dip()`.
///
/// ```
/// use zero_ui::prelude::*;
/// #
/// # fn main() { let _ =
/// Container! {
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
/// use zero_ui::prelude::*;
/// #
/// # fn main() { let _ =
/// Container! {
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
/// use zero_ui::prelude::new_property::*;
///
/// /// Another border property.
/// #[property(BORDER, default(0, BorderStyle::Hidden))]
/// pub fn my_border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
///     zero_ui::properties::border(child, widths, sides)
/// }
/// #
/// # fn main() { }
/// ```
///
/// Now we can set two borders in the same widget:
///
/// ```
/// # mod my_properties {
/// #     use zero_ui::prelude::new_property::*;
/// #
/// #     /// Another border property.
/// #     #[property(BORDER, default(0, BorderStyle::Hidden))]
/// #     pub fn my_border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
/// #         zero_ui::properties::border(child, widths, sides)
/// #     }
/// # }
/// # use my_properties::*;
/// #
/// use zero_ui::prelude::*;
///
/// # fn main() {
/// # let _ =
/// Container! {
///     border = 4, colors::RED;
///     my_border = 4, colors::GREEN;
///     corner_radius = 8;
/// }
/// # ; }
/// ```
///
/// This will render a `RED` border around a `GREEN` one, the inner border will fit perfectly inside the outer one,
/// the `corner_radius` defines the the outer radius, the inner radius is computed automatically to fit.
///
/// Note that because both borders have the same [`NestGroup::BORDER`] the position they are declared in the widget matters:
///
/// ```
/// # mod my_properties {
/// #     use zero_ui::prelude::new_property::*;
/// #
/// #     /// Another border property.
/// #     #[property(BORDER, default(0, BorderStyle::Hidden))]
/// #     pub fn my_border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
/// #         zero_ui::properties::border(child, widths, sides)
/// #     }
/// # }
/// # use my_properties::*;
/// #
/// use zero_ui::prelude::*;
///
/// # fn main() {
/// # let _ =
/// Container! {
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
/// use zero_ui::prelude::new_property::*;
///
/// /// Border that is always around the other borders.
/// #[property(BORDER-1, default(0, BorderStyle::Hidden))]
/// pub fn outside_border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
///     zero_ui::properties::border(child, widths, sides)
/// }
///  
/// /// Border that is always inside the other borders.
/// #[property(BORDER+1, default(0, BorderStyle::Hidden))]
/// pub fn inside_border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
///     zero_ui::properties::border(child, widths, sides)
/// }
/// #
/// # fn main() { }
/// ```
///
/// [`corner_radius`]: fn@corner_radius
/// [`NestGroup::BORDER`]: crate::core::widget_builder::NestGroup::BORDER
#[property(BORDER, default(0, BorderStyle::Hidden))]
pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
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
                frame.push_border(rect, offsets, sides.get(), corners);
            }
            _ => {}
        }),
    )
}
