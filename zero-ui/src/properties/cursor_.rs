use crate::core::{
    mouse::MouseHoveredEvent,
    window::{CursorIcon, WindowVarsKey},
};
use crate::prelude::new_property::*;

/// Widget property that sets the [`CursorIcon`] displayed when hovering the widget.
///
/// # Examples
/// 
/// ```
/// # use zero_ui::prelude::*;
/// container! {
///     cursor = CursorIcon::Hand;
///     content = text("Mouse over this text shows the hand cursor");
/// }
/// # ;
/// ```
///
/// [`CursorIcon`]: crate::core::window::CursorIcon
#[property(context, default(CursorIcon::Default))]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
    struct CursorNode<T, C> {
        cursor: C,
        child: T,
        hovered_binding: VarBindingHandle,
    }
    #[impl_ui_node(child)]
    impl<T, C> UiNode for CursorNode<T, C>
    where
        T: UiNode,
        C: Var<Option<CursorIcon>>,
    {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.cursor).event(MouseHoveredEvent);
            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = MouseHoveredEvent.update(args) {
                self.child.event(ctx, args);

                let state = *ctx.update_state.entry(CursorStateKey).or_default();
                match state {
                    CursorState::Default => {
                        // child did not set

                        if args.target.as_ref().map(|t| t.contains(ctx.path.widget_id())).unwrap_or(false) {
                            // we can set

                            if self.hovered_binding.is_unbound() {
                                // we are not already set, setup binding.

                                let cursor = ctx.window_state.req(WindowVarsKey).cursor();
                                cursor.set_ne(ctx.vars, self.cursor.copy(ctx.vars));
                                self.hovered_binding = self.cursor.bind_into(ctx.vars, cursor);
                            }

                            // flag parent
                            ctx.update_state.set(CursorStateKey, CursorState::Set);
                        } else if !self.hovered_binding.is_unbound() {
                            // we are set, unbind.

                            self.hovered_binding = VarBindingHandle::dummy();
                            ctx.window_state
                                .req(WindowVarsKey)
                                .cursor()
                                .set_ne(ctx.vars, Some(CursorIcon::Default));
                        }
                    }
                    CursorState::Set => {
                        // child did set, unbind if we were bound.

                        self.hovered_binding = VarBindingHandle::dummy();
                    }
                }
            } else {
                self.child.event(ctx, args);
            }
        }
    }
    CursorNode {
        cursor: cursor.into_var(),
        child,
        hovered_binding: VarBindingHandle::dummy(),
    }
}

#[derive(Clone, Copy)]
enum CursorState {
    /// Restore to default.
    Default,

    /// Cursor already set by child.
    Set,
}

impl Default for CursorState {
    fn default() -> Self {
        CursorState::Default
    }
}

state_key! {
    struct CursorStateKey: CursorState;
}
