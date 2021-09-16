use crate::prelude::new_property::*;

/// Draws a border around the widget.
///
/// TODO radii clip:
//
// https://github.com/servo/servo/blob/0d0cfd030347ab0711b3c0607a9ee07ffe7124cf/components/layout/display_list/border.rs
// https://github.com/servo/servo/blob/0d0cfd030347ab0711b3c0607a9ee07ffe7124cf/components/layout/display_list/background.rs
#[property(inner, default(0, BorderStyle::Hidden, 0))]
pub fn border(
    child: impl UiNode,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
    radius: impl IntoVar<BorderRadius>,
) -> impl UiNode {
    struct BorderNode<T, L, S, R> {
        child: T,

        widths: L,
        sides: S,
        radius: R,
        child_rect: PxRect,

        final_widths: PxSideOffsets,
        final_size: PxSize,
        final_radius: PxCornerRadius,
    }

    #[impl_ui_node(child)]
    impl<T, L, S, R> BorderNode<T, L, S, R>
    where
        T: UiNode,
        L: Var<SideOffsets>,
        S: Var<BorderSides>,
        R: Var<BorderRadius>,
    {
        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.widths.is_new(ctx) || self.radius.is_new(ctx) {
                ctx.updates.layout()
            }
            if self.sides.is_new(ctx) {
                ctx.updates.render()
            }
        }

        #[UiNode]
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            self.final_widths = self.widths.get(ctx).to_layout(ctx, available_size);
            self.final_radius = self.radius.get(ctx).to_layout(ctx, available_size);

            let size_inc = self.size_increment();
            self.child.measure(ctx, available_size.sub_px(size_inc)) + size_inc
        }

        #[UiNode]
        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: PxSize) {
            self.child_rect.origin = PxPoint::new(self.final_widths.left, self.final_widths.top);
            let child_size = final_size - self.size_increment();
            self.child_rect.size = child_size;
            self.final_size = final_size;
            self.child.arrange(ctx, child_size);
        }

        fn size_increment(&self) -> PxSize {
            let rw = self.final_widths;
            PxSize::new(rw.left + rw.right, rw.top + rw.bottom)
        }

        #[UiNode]
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_border(
                PxRect::from_size(self.final_size),
                self.final_widths,
                *self.sides.get(ctx),
                self.final_radius,
            );
            frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(ctx, frame));
        }
    }

    BorderNode {
        child,

        widths: widths.into_var(),
        sides: sides.into_var(),
        radius: radius.into_var(),

        child_rect: PxRect::zero(),
        final_size: PxSize::zero(),
        final_widths: PxSideOffsets::zero(),
        final_radius: PxCornerRadius::zero(),
    }
}
