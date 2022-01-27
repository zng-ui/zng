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

        spatial_id: SpatialFrameId,

        widths: L,
        sides: S,
        radius: R,
        child_rect: PxRect,

        final_widths: PxSideOffsets,
        final_sides: BorderSides,
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
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.vars(ctx).var(&self.widths).var(&self.radius).var(&self.sides);

            self.child.subscriptions(ctx, subscriptions);
        }

        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.final_sides = self.sides.copy(ctx);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if self.widths.is_new(ctx) || self.radius.is_new(ctx) {
                ctx.updates.layout()
            }
            if let Some(s) = self.sides.copy_new(ctx) {
                ctx.updates.render();

                self.final_sides = s;
            }
        }

        #[UiNode]
        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let final_widths = self.widths.get(ctx).to_layout(ctx, available_size, PxSideOffsets::zero());
            let final_radius = self.radius.get(ctx).to_layout(ctx, available_size, PxCornerRadius::zero());

            if final_widths != self.final_widths || final_radius != self.final_radius {
                ctx.updates.render();

                self.final_widths = final_widths;
                self.final_radius = final_radius;
            }

            let size_inc = self.size_increment();
            self.child.measure(ctx, available_size.sub_px(size_inc)) + size_inc
        }

        #[UiNode]
        fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            let origin = PxPoint::new(self.final_widths.left, self.final_widths.top);
            let child_size = final_size - self.size_increment();
            let child_rect = PxRect::new(origin, child_size);

            if self.final_size != final_size || child_rect != self.child_rect {
                self.child_rect = child_rect;
                self.final_size = final_size;

                ctx.updates.render();
            }

            widget_layout.with_pre_translate(origin.to_vector(), |wo| self.child.arrange(ctx, wo, child_size));
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
                self.final_sides,
                self.final_radius,
            );
            if self.child_rect.origin != PxPoint::zero() {
                let transform = RenderTransform::translation(self.child_rect.origin.x.0 as f32, self.child_rect.origin.y.0 as f32, 0.0);
                frame.push_reference_frame(self.spatial_id, FrameBinding::Value(transform), true, |frame| {
                    self.child.render(ctx, frame)
                });
            } else {
                self.child.render(ctx, frame);
            }
        }
    }

    BorderNode {
        child,

        spatial_id: SpatialFrameId::new_unique(),

        widths: widths.into_var(),
        sides: sides.into_var(),
        radius: radius.into_var(),

        child_rect: PxRect::zero(),
        final_size: PxSize::zero(),
        final_widths: PxSideOffsets::zero(),
        final_sides: BorderSides::solid(colors::BLACK.transparent()),
        final_radius: PxCornerRadius::zero(),
    }
}
