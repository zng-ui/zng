use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct Cursor<T: UiNode, C: LocalVar<CursorIcon>> {
    cursor: C,
    child: T,
}

#[impl_ui_node(child)]
impl<T: UiNode, C: LocalVar<CursorIcon>> UiNode for Cursor<T, C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.cursor.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.cursor.update_local(&ctx.vars).is_some() {
            ctx.updates.push_render();
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
        cursor: cursor.into_local(),
        child,
    }
}
