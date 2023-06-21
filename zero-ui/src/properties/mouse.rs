use crate::core::{
    mouse::{ClickMode, WidgetInfoBuilderMouseExt, MOUSE_HOVERED_EVENT},
    window::CursorIcon,
};
use crate::prelude::new_property::*;

/// Widget property that sets the [`CursorIcon`] displayed when hovering the widget.
///
/// # Examples
///
/// ```
/// # use zero_ui::prelude::*;
/// Container! {
///     cursor = CursorIcon::Hand;
///     child = Text!("Mouse over this text shows the hand cursor");
/// }
/// # ;
/// ```
///
/// [`CursorIcon`]: crate::core::window::CursorIcon
#[property(CONTEXT, default(CursorIcon::Default))]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
    let cursor = cursor.into_var();
    let mut hovered_binding = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&MOUSE_HOVERED_EVENT);
        }
        UiNodeOp::Deinit => {
            hovered_binding = None;
        }
        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                let is_over = args.target.as_ref().map(|t| t.as_path().contains(WIDGET.id())).unwrap_or(false);
                if is_over {
                    if hovered_binding.is_none() {
                        // we are not already set, setup binding.

                        let c = WINDOW.vars().cursor();
                        c.set_from(&cursor);
                        hovered_binding = Some(cursor.bind(&c));
                    }
                } else {
                    // restore to default, if not set to other value already
                    if hovered_binding.is_some() {
                        hovered_binding = None;
                        let value = cursor.get();
                        WINDOW.vars().cursor().modify(move |c| {
                            if c.as_ref() == &value {
                                *c.to_mut() = Some(CursorIcon::Default);
                            }
                        });
                    }
                }
            }
        }
        _ => {}
    })
}

/// Defines how click events are generated for the widget.
///
/// Setting this to `None` will cause the widget to inherit the parent mode, or [`ClickMode::Default`] if
/// no parent sets the click mode.
#[property(CONTEXT, default(None))]
pub fn click_mode(child: impl UiNode, mode: impl IntoVar<Option<ClickMode>>) -> impl UiNode {
    let mode = mode.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&mode);
        }
        UiNodeOp::Info { info } => {
            info.set_click_mode(mode.get());
        }
        _ => {}
    })
}
