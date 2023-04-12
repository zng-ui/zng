use crate::core::{
    mouse::{ClickMode, WidgetInfoBuilderMouseExt, MOUSE_HOVERED_EVENT},
    window::{CursorIcon, WINDOW_CTRL},
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
///     child = Text!("Mouse over this text shows the hand cursor");
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

        fn event(&mut self, update: &EventUpdate) {
            self.child.event(update);
            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                let is_over = args.target.as_ref().map(|t| t.as_path().contains(WIDGET.id())).unwrap_or(false);
                if is_over {
                    if self.hovered_binding.is_none() {
                        // we are not already set, setup binding.

                        let cursor = WINDOW_CTRL.vars().cursor();
                        cursor.set_ne(self.cursor.get());
                        self.hovered_binding = Some(self.cursor.bind(&cursor));
                    }
                } else {
                    // restore to default, if not set to other value already
                    if self.hovered_binding.is_some() {
                        self.hovered_binding = None;
                        let value = self.cursor.get();
                        WINDOW_CTRL.vars().cursor().modify(move |c| {
                            if c.as_ref() == &value {
                                *c.to_mut() = Some(CursorIcon::Default);
                            }
                        });
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

/// Defines how click events are generated for the widget.
///
/// Setting this to `None` will cause the widget to inherit the parent mode, or [`ClickMode::Default`] if
/// no parent sets the click mode.
#[property(CONTEXT, default(None))]
pub fn click_mode(child: impl UiNode, mode: impl IntoVar<Option<ClickMode>>) -> impl UiNode {
    #[ui_node(struct ClickModeNode {
        child: impl UiNode,
        #[var] mode: impl Var<Option<ClickMode>>,
    })]
    impl UiNode for ClickModeNode {
        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);

            if self.mode.is_new() {
                WIDGET.update_info();
            }
        }

        fn info(&self, info: &mut WidgetInfoBuilder) {
            info.set_click_mode(self.mode.get());

            self.child.info(info);
        }
    }
    ClickModeNode {
        child,
        mode: mode.into_var(),
    }
}
