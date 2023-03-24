use crate::prelude::new_property::*;

#[doc(inline)]
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
    #[ui_node(struct BorderNode {
        #[var] sides: impl Var<BorderSides>,
        corners: PxCornerRadius,
    })]
    impl UiNode for BorderNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.sides.is_new() {
                WIDGET.render();
            }
        }

        fn measure(&self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constrains().fill_size()
        }
        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            self.corners = BORDER.border_radius();
            LAYOUT.constrains().fill_size()
        }

        fn render(&self, frame: &mut FrameBuilder) {
            let (rect, offsets) = BORDER.border_layout();
            frame.push_border(rect, offsets, self.sides.get(), self.corners);
        }
    }

    border_node(
        child,
        widths,
        BorderNode {
            sides: sides.into_var(),
            corners: PxCornerRadius::zero(),
        },
    )
}
