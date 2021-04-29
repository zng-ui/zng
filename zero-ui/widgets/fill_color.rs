use crate::prelude::new_widget::*;

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<Rgba>) -> impl UiNode {
    struct FillColorNode<C: VarLocal<Rgba>> {
        color: C,
        final_size: LayoutSize,
    }
    #[impl_ui_node(none)]
    impl<C: VarLocal<Rgba>> UiNode for FillColorNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.color.init_local(ctx.vars);
        }
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.color.update_local(ctx.vars).is_some() {
                ctx.updates.render();
            }
        }
        fn arrange(&mut self, _: &mut LayoutContext, final_size: LayoutSize) {
            self.final_size = final_size;
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_color(LayoutRect::from_size(self.final_size), (*self.color.get_local()).into());
        }
    }

    FillColorNode {
        color: color.into_local(),
        final_size: LayoutSize::default(),
    }
}
