use crate::prelude::new_widget::*;

/// Node that fills the widget area with a color.
pub fn flood(color: impl IntoVar<Rgba>) -> impl UiNode {
    #[impl_ui_node(struct FloodNode {
        #[var] color: impl Var<Rgba>,
        frame_key: FrameVarKey<RenderColor>,
        final_size: PxSize,
    })]
    impl UiNode for FloodNode {
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.color.is_new(ctx) {
                ctx.updates.render_update();
            }
        }
        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.constrains().fill_size()
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            let final_size = ctx.constrains().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                ctx.updates.render();
            }
            final_size
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_color(PxRect::from_size(self.final_size), self.frame_key.bind(&self.color, |&c| c.into()));
        }

        fn render_update(&self, _: &mut RenderContext, update: &mut FrameUpdate) {
            update.update_color_opt(self.frame_key.update(&self.color, |&c| c.into()));
        }
    }

    FloodNode {
        frame_key: FrameVarKey::new(),
        color: color.into_var(),
        final_size: PxSize::zero(),
    }
    .cfg_boxed()
}
