use crate::prelude::new_property::*;

pub use crate::core::border::{border_align, corner_radius, corner_radius_fit, CornerRadiusFit};
use crate::core::border::{border_node, BORDER};

/// Widget border.
///
/// Defines a widget border, it coordinates with any other border in the widget, meaning this property can be safely set
/// more than once for a single widget, it also works with the [`corner_radius`] property drawing round corners if configured.
///
/// This property disables inline layout for the widget.
///
/// [`corner_radius`]: fn@corner_radius
#[property(BORDER, default(0, BorderStyle::Hidden))]
pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
    let sides = sides.into_var();
    let mut corners = PxCornerRadius::zero();

    border_node(
        child,
        widths,
        match_node_leaf(move |op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&sides);
            }
            UiNodeOp::Update { .. } => {
                if sides.is_new() {
                    WIDGET.render();
                }
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
