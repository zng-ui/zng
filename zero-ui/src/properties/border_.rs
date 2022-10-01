use crate::prelude::new_property::*;

#[doc(inline)]
pub use crate::core::border::{border_align, corner_radius, corner_radius_fit, CornerRadiusFit};
use crate::core::border::{border_node, ContextBorders};

/// Widget border.
///
/// Defines a widget border, it coordinates with any other border in the widget, meaning this property can be safely set
/// more than once for a single widget, it also works with the [`corner_radius`] property drawing round corners if configured.
///
/// [`corner_radius`]: fn@corner_radius
#[property(border, default(0, BorderStyle::Hidden))]
pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
    struct BorderNode<S> {
        sides: S,
        corners: PxCornerRadius,
    }
    #[impl_ui_node(none)]
    impl<S: Var<BorderSides>> UiNode for BorderNode<S> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.sides);
        }

        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.sides.is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.constrains().fill_size()
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            self.corners = ContextBorders::border_radius(ctx);
            ctx.constrains().fill_size()
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let (rect, offsets) = ContextBorders::border_layout();
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
