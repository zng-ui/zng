use crate::prelude::new_widget::*;

/// Node that fills the widget area with a color.
pub fn flood(color: impl IntoVar<Rgba>) -> impl UiNode {
    #[ui_node(struct FloodNode {
        #[var] color: impl Var<Rgba>,
        frame_key: FrameValueKey<RenderColor>,
        final_size: PxSize,
    })]
    impl UiNode for FloodNode {
        fn update(&mut self, _: &WidgetUpdates) {
            if self.color.is_new() {
                WIDGET.render_update();
            }
        }
        fn measure(&self, _: &mut WidgetMeasure) -> PxSize {
            LAYOUT.constrains().fill_size()
        }
        fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
            let final_size = LAYOUT.constrains().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                WIDGET.render();
            }
            final_size
        }

        fn render(&self, frame: &mut FrameBuilder) {
            frame.push_color(
                PxRect::from_size(self.final_size),
                self.frame_key.bind_var(&self.color, |&c| c.into()),
            );
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            update.update_color_opt(self.frame_key.update_var(&self.color, |&c| c.into()));
        }
    }

    FloodNode {
        frame_key: FrameValueKey::new_unique(),
        color: color.into_var(),
        final_size: PxSize::zero(),
    }
    .cfg_boxed()
}
