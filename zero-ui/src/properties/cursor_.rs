use crate::core::{
    mouse::MOUSE_HOVERED_EVENT,
    window::{CursorIcon, WindowVars},
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
///     child = text("Mouse over this text shows the hand cursor");
/// }
/// # ;
/// ```
///
/// [`CursorIcon`]: crate::core::window::CursorIcon
#[property(context, default(CursorIcon::Default))]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
    #[ui_node(struct CursorNode {
        child: impl UiNode,
        #[var] cursor: impl Var<Option<CursorIcon>>,
        hovered_binding: Option<VarHandle>,
    })]
    impl UiNode for CursorNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.init_handles(ctx);
            ctx.sub_event(&MOUSE_HOVERED_EVENT);
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.hovered_binding = None;
            self.child.deinit(ctx);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);
            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                let state = *ctx.update_state.entry(&CURSOR_STATE_ID).or_default();
                match state {
                    CursorState::Default => {
                        // child did not set

                        if args.target.as_ref().map(|t| t.contains(ctx.path.widget_id())).unwrap_or(false) {
                            // we can set

                            if self.hovered_binding.is_none() {
                                // we are not already set, setup binding.

                                let cursor = WindowVars::req(&ctx.window_state).cursor();
                                cursor.set_ne(ctx.vars, self.cursor.get());
                                self.hovered_binding = Some(self.cursor.bind(cursor));
                            }

                            // flag parent
                            ctx.update_state.set(&CURSOR_STATE_ID, CursorState::Set);
                        } else if self.hovered_binding.is_some() {
                            // we are set, unbind.

                            self.hovered_binding = None;
                            WindowVars::req(&ctx.window_state)
                                .cursor()
                                .set_ne(ctx.vars, Some(CursorIcon::Default));
                        }
                    }
                    CursorState::Set => {
                        // child did set, unbind if we were bound.

                        self.hovered_binding = None;
                    }
                }
            }
        }
    }
    CursorNode {
        child,
        cursor: cursor.into_var(),
        hovered_binding: None,
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

static CURSOR_STATE_ID: StaticStateId<CursorState> = StaticStateId::new_unique();
