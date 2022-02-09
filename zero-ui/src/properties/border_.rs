use crate::prelude::new_property::*;

/// Draws a border around the widget.
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

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let parent_widths = widget_layout.border_offsets();
            self.final_rect.origin = PxPoint::new(parent_widths.left, parent_widths.top);
            self.final_rect.size = final_size;

            self.final_rect.size.width -= parent_widths.left + parent_widths.right;
            self.final_rect.size.height -= parent_widths.top + parent_widths.bottom;

            self.final_radius = widget_layout.corner_radius();

            self.final_widths =
                self.widths
                    .get(ctx.vars)
                    .to_layout(ctx.metrics, AvailableSize::finite(self.final_rect.size), PxSideOffsets::zero());

            widget_layout.with_border(self.final_widths, |wl| {
                // border padding is up to the widget.
                self.child.arrange(ctx, wl, final_size);
            });
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
