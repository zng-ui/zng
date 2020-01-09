use super::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

struct Widget<T: UiNode> {
    id: WidgetId,
    child: T,
}

#[impl_ui_node_crate]
impl<T: UiNode> UiNode for Widget<T> {
    fn update(&mut self, ctx: &mut AppContext) {
        ctx.widget_update(self.id, |ctx| self.child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut AppContext) {
        ctx.widget_update(self.id, |ctx| self.child.update_hp(ctx));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, &self.child);
    }
}

/// Creates a widget bondary.
pub fn widget(id: WidgetId, child: impl UiNode) -> impl UiNode {
    Widget { id, child }
}

struct Cursor<T: UiNode, C: Var<CursorIcon>> {
    cursor: C,
    child: T,
    render_cursor: CursorIcon,
}

#[impl_ui_node_crate]
impl<T: UiNode, C: Var<CursorIcon>> UiNode for Cursor<T, C> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.render_cursor = *self.cursor.get(&ctx);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(cursor) = self.cursor.update(&ctx) {
            self.render_cursor = *cursor;
            ctx.push_frame();
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_cursor(self.render_cursor, &self.child);
    }
}

#[property(context_var)]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    Cursor {
        cursor: cursor.into_var(),
        child,
        render_cursor: CursorIcon::Default,
    }
}
