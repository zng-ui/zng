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
///     child = text!("Mouse over this text shows the hand cursor");
/// }
/// # ;
/// ```
///
/// [`CursorIcon`]: crate::core::window::CursorIcon
#[property(CONTEXT, default(CursorIcon::Default))]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
    #[ui_node(struct CursorNode {
        child: impl UiNode,
        #[var] cursor: impl Var<Option<CursorIcon>>,
        hovered_binding: Option<VarHandle>,
    })]
    impl UiNode for CursorNode {
        fn init(&mut self) {
            self.auto_subs();
            WIDGET.sub_event(&MOUSE_HOVERED_EVENT);
            self.child.init();
        }

        fn deinit(&mut self) {
            self.hovered_binding = None;
            self.child.deinit();
        }

        fn event(&mut self, update: &mut EventUpdate) {
            self.child.event(update);
            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if args.is_mouse_enter() {
                    if self.hovered_binding.is_none() {
                        // we are not already set, setup binding.

                        let cursor = WindowVars::req().cursor();
                        cursor.set_ne(self.cursor.get());
                        self.hovered_binding = Some(self.cursor.bind(&cursor));
                    }
                } else if args.is_mouse_leave() {
                    self.hovered_binding = None;
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
