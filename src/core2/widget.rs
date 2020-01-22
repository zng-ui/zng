use super::*;
use crate::impl_ui_node;
use contexts::WidgetId;

struct Widget<T: UiNode> {
    id: WidgetId,
    child: T,
}

#[impl_ui_node(child)]
impl<T: UiNode> UiNode for Widget<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        ctx.widget_context(self.id, |ctx| self.child.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        ctx.widget_context(self.id, |ctx| self.child.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        ctx.widget_context(self.id, |ctx| self.child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        ctx.widget_context(self.id, |ctx| self.child.update_hp(ctx));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, &self.child);
    }
}

/// Creates a widget bondary.
pub fn widget(id: WidgetId, child: impl UiNode) -> impl UiNode {
    Widget { id, child }
}
