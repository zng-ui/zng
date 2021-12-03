use std::cell::Cell;

use crate::prelude::new_widget::*;

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<Rgba>) -> impl UiNode {
    struct FillColorNode<C> {
        color: C,
        frame_key: Option<FrameBindingKey<RenderColor>>,
        final_size: PxSize,
        requested_update: Cell<bool>,
    }
    #[impl_ui_node(none)]
    impl<C: Var<Rgba>> UiNode for FillColorNode<C> {
        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            widget.subscriptions().var(ctx, &self.color);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.color.is_new(ctx) {
                ctx.updates.render_update();
                self.requested_update.set(true);
            }
        }
        fn arrange(&mut self, ctx: &mut LayoutContext, _: &mut WidgetOffset, final_size: PxSize) {
            if self.final_size != final_size {
                self.final_size = final_size;
                ctx.updates.render();
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.requested_update.set(false);
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
                if self.requested_update.take() {
                    let color = key.update(self.color.copy(ctx).into());
                    update.update_color(color);
                }
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
        requested_update: Cell::new(false),
    }
}
