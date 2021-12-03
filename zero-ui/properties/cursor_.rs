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
#[property(context, default(CursorIcon::Default))]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    struct CursorNode<T, C> {
        cursor: C,
        child: T,
    }
    #[impl_ui_node(child)]
    impl<T: UiNode, C: Var<CursorIcon>> UiNode for CursorNode<T, C> {
        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget);
            widget.subscriptions().var(ctx, &self.cursor);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.cursor.is_new(ctx) {
                ctx.updates.render(); // TODO reduce this to a metadata render_update.
            }
            self.child.update(ctx);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_cursor(self.cursor.copy(ctx), |frame| self.child.render(ctx, frame));
        }
    }
    CursorNode {
        cursor: cursor.into_var(),
        child,
    }
}
