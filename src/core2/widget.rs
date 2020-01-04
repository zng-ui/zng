use super::*;
use zero_ui_macros::impl_ui_node_crate;

struct Widget<T: UiNode> {
    id: WidgetId,
    child: T
}

#[impl_ui_node_crate]
impl<T: UiNode> UiNode for Widget<T> {
    fn update(&mut self, ctx: &mut AppContext) {
        ctx.widget_update(self.id, |ctx|self.child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut AppContext) {
        ctx.widget_update(self.id, |ctx|self.child.update_hp(ctx));
    }
}

/// Creates a widget bondary.
pub fn widget(id: WidgetId, child: impl UiNode) -> impl UiNode {
    Widget {
        id,
        child
    }
}