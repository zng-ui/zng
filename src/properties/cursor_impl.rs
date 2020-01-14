use crate::core2::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;


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

/// Cursor property that sets the cursor.
///
/// # Arguments
/// * `cursor`: The cursor to use for `child`, can be a direct [value](CursorIcon) or a [variable](zero_ui::core::Var).
///
/// # Example
/// ```
/// # use zero_ui::properties::{text, cursor};
/// # use zero_ui::core::CursorIcon;
/// # use zero_ui::ui;
/// ui! {
///     cursor: CursorIcon::Hand;
///     => text("Mouse over this text shows the hand cursor")
/// }
/// # ;
/// ```
#[property(context_var)]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    Cursor {
        cursor: cursor.into_var(),
        child,
        render_cursor: CursorIcon::Default,
    }
}
