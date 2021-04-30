use crate::prelude::new_widget::*;

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<Rgba>) -> impl UiNode {
    struct FillColorNode<C> {
        color: C,
        final_size: LayoutSize,
    }
    #[impl_ui_node(none)]
    impl<C: Var<Rgba>> UiNode for FillColorNode<C> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.color.is_new(ctx.vars) {
                ctx.updates.render();
            }
        }
        fn arrange(&mut self, _: &mut LayoutContext, final_size: LayoutSize) {
            self.final_size = final_size;
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_color(LayoutRect::from_size(self.final_size), (*self.color.get(ctx.vars)).into());
        }
    }

    FillColorNode {
        color: color.into_var(),
        final_size: LayoutSize::default(),
    }
}
