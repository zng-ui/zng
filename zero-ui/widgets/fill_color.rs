use crate::prelude::new_widget::*;

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<Rgba>) -> impl UiNode {
    struct FillColorNode<C> {
        color: C,
        frame_key: Option<FrameBindingKey<RenderColor>>,
        final_size: PxSize,
    }
    #[impl_ui_node(none)]
    impl<C: Var<Rgba>> UiNode for FillColorNode<C> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.color.is_new(ctx) {
                ctx.updates.render_update();
            }
        }
        fn arrange(&mut self, _: &mut LayoutContext, final_size: PxSize) {
            self.final_size = final_size;
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let color = self.color.copy(ctx).into();
            let color = if let Some(key) = self.frame_key {
                key.bind(color)
            } else {
                FrameBinding::Value(color)
            };
            frame.push_color(PxRect::from_size(self.final_size), color);
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            if let Some(key) = self.frame_key {
                let color = key.update(self.color.copy(ctx).into());
                update.update_color(color);
            }
        }
    }

    let color = color.into_var();
    let frame_key = if color.can_update() {
        Some(FrameBindingKey::new_unique())
    } else {
        None
    };
    FillColorNode {
        color,
        frame_key,
        final_size: PxSize::zero(),
    }
}
