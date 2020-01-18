use crate::core2::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

struct Cursor<T: UiNode, C: LocalVar<CursorIcon>> {
    cursor: C,
    child: T,
}

#[impl_ui_node_crate]
impl<T: UiNode, C: LocalVar<CursorIcon>> UiNode for Cursor<T, C> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.cursor.init_local(ctx);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if self.cursor.update_local(&ctx).is_some() {
            ctx.push_frame();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_cursor(*self.cursor.get_local(), &self.child);
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
        cursor: cursor.into_var().as_local(),
        child,
    }
}
