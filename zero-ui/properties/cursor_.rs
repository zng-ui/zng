use crate::core::window::CursorIcon;
use crate::prelude::new_property::*;

/// Widget property that sets the [`CursorIcon`](crate::core::types::CursorIcon) displayed when hovering the widget.
///
/// # Example
/// ```
/// # use zero_ui::prelude::*;
/// container! {
///     cursor = CursorIcon::Hand;
///     content = text("Mouse over this text shows the hand cursor");
/// }
/// # ;
/// ```
#[property(context)]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    struct CursorNode<T, C> {
        cursor: C,
        child: T,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, C: Var<CursorIcon>> UiNode for CursorNode<T, C> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.cursor.is_new(&ctx.vars) {
                ctx.updates.render();
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_cursor(*self.cursor.get(ctx.vars), |frame| self.child.render(ctx, frame));
        }
    }
    CursorNode {
        cursor: cursor.into_var(),
        child,
    }
}
