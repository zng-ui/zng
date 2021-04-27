//! Border property and types.

use crate::prelude::new_property::*;

// TODO refactor how corner radii layout is done.

/// Border property
#[property(inner)]
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
        child_rect: LayoutRect,

        final_widths: LayoutSideOffsets,
        final_size: LayoutSize,
        final_radius: LayoutBorderRadius,
    }

    #[impl_ui_node(child)]
    impl<T, L, S, R> BorderNode<T, L, S, R>
    where
        T: UiNode,
        L: VarLocal<SideOffsets>,
        S: VarLocal<BorderSides>,
        R: VarLocal<BorderRadius>,
    {
        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.widths.init_local(ctx.vars);
            self.sides.init_local(ctx.vars);
            self.radius.init_local(ctx.vars);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.widths.update_local(ctx.vars).is_some() {
                ctx.updates.layout()
            }
            if self.sides.update_local(ctx.vars).is_some() {
                ctx.updates.render()
            }
            if self.radius.update_local(ctx.vars).is_some() {
                ctx.updates.layout();
            }
        }

        #[UiNode]
        fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
            self.final_widths = self.widths.get_local().to_layout(available_size, ctx);
            self.final_radius = self.radius.get_local().to_layout(available_size, ctx);

            let size_inc = self.size_increment();
            self.child.measure(available_size - size_inc, ctx) + size_inc
        }

        #[UiNode]
        fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
            self.child_rect.origin = LayoutPoint::new(self.final_widths.left, self.final_widths.top);
            self.child_rect.size = final_size - self.size_increment();
            self.final_size = final_size;
            self.child.arrange(self.child_rect.size, ctx);
        }

        fn size_increment(&self) -> LayoutSize {
            let rw = self.final_widths;
            LayoutSize::new(rw.left + rw.right, rw.top + rw.bottom)
        }

        #[UiNode]
        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_border(
                LayoutRect::from_size(self.final_size),
                self.final_widths,
                *self.sides.get_local(),
                self.final_radius,
            );
            frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
        }
    }

    BorderNode {
        child,

        widths: widths.into_local(),
        sides: sides.into_local(),
        radius: radius.into_local(),

        child_rect: LayoutRect::zero(),
        final_size: LayoutSize::zero(),
        final_widths: LayoutSideOffsets::zero(),
        final_radius: LayoutBorderRadius::zero(),
    }
}
