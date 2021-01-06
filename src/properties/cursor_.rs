use crate::core::window::CursorIcon;
use crate::prelude::new_property::*;

struct CursorNode<T: UiNode, C: VarLocal<CursorIcon>> {
    cursor: C,
    child: T,
}

#[impl_ui_node(child)]
impl<T: UiNode, C: VarLocal<CursorIcon>> UiNode for CursorNode<T, C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.cursor.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.cursor.update_local(&ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_cursor(*self.cursor.get_local(), |frame| self.child.render(frame));
    }
}

/// Widget property that sets the [`CursorIcon`](crate::core::types::CursorIcon) displayed when hovering the widget.
///
/// # Example
/// ```
/// # use crate::prelude::*;
/// container! {
///     cursor: CursorIcon::Hand;
///     content: text("Mouse over this text shows the hand cursor");
/// }
/// # ;
/// ```
#[property(context)]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    CursorNode {
        cursor: cursor.into_local(),
        child,
    }
}
