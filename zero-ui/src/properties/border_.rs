use crate::prelude::new_property::*;

#[doc(inline)]
pub use crate::core::border::{border_align, corner_radius};

/// Widget border.
///
/// Defines a widget border, it coordinates with any other border in the widget, meaning this property can be safely set
/// more the nonce for a single widget, it also works with the [`corner_radius`] property drawing round corners if configured.
///
/// [`corner_radius`]: fn@corner_radius
#[property(border, default(0, BorderStyle::Hidden))]
pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
    struct BorderNode<T, O, S> {
        child: T,

        widths: O,
        sides: S,

        final_widths: PxSideOffsets,
        final_rect: PxRect,
        final_radius: PxCornerRadius,
    }

    #[impl_ui_node(child)]
    impl<T, O, S> UiNode for BorderNode<T, O, S>
    where
        T: UiNode,
        O: Var<SideOffsets>,
        S: Var<BorderSides>,
    {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.vars(ctx).var(&self.widths).var(&self.sides);

            self.child.subscriptions(ctx, subscriptions);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.widths.is_new(ctx) {
                ctx.updates.layout()
            }
            if self.sides.is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            self.final_widths = self.widths.get(ctx.vars).layout(ctx.metrics, PxSideOffsets::zero());

            let diff = PxSize::new(self.final_widths.horizontal(), self.final_widths.vertical());

            // TODO !!: with_border
            ctx.with_sub_size(diff, |ctx| self.child.layout(ctx, wl))
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);
            frame.push_border(self.final_rect, self.final_widths, self.sides.copy(ctx), self.final_radius);
        }
    }

    BorderNode {
        child,

        widths: widths.into_var(),
        sides: sides.into_var(),

        final_rect: PxRect::zero(),
        final_widths: PxSideOffsets::zero(),
        final_radius: PxCornerRadius::zero(),
    }
}
