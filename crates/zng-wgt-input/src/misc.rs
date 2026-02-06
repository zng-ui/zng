use std::sync::atomic::AtomicBool;

use zng_ext_input::mouse::{ClickMode, MOUSE_HOVERED_EVENT, WidgetInfoBuilderMouseExt as _};
use zng_ext_window::WINDOWS;
use zng_wgt::prelude::*;

pub use zng_view_api::window::CursorIcon;

pub use zng_ext_window::CursorSource;

#[cfg(feature = "image")]
pub use zng_ext_window::CursorImg;

context_local! {
    static CHILD_SETS_CURSOR: AtomicBool = AtomicBool::new(false);
}

static_id! {
    // set on info metadata
    static ref WIDGET_CURSOR_ID: StateId<Var<CursorSource>>;
    static ref WINDOW_CURSOR_HANDLER_ID: StateId<WeakVarHandle>;
}

/// Sets the mouse pointer cursor displayed when hovering the widget.
///
/// You can set this property to a [`CursorIcon`] for a named platform dependent icon, [`CursorImg`] for a custom image,
/// or to `false` that converts to [`CursorSource::Hidden`].
///
/// [`CursorImg`]: zng_ext_window::CursorImg
#[property(CONTEXT, default(CursorIcon::Default))]
pub fn cursor(child: impl IntoUiNode, cursor: impl IntoVar<CursorSource>) -> UiNode {
    let cursor = cursor.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            // setup a single handler for the window, each widget
            // that sets cursor holds the handle
            let id = WINDOW.id();
            let h = WINDOW.with_state_mut(|mut s| match s.entry(*WINDOW_CURSOR_HANDLER_ID) {
                state_map::StateMapEntry::Occupied(mut e) => {
                    if let Some(h) = e.get().upgrade() {
                        h
                    } else {
                        let h = cursor_impl(id);
                        e.insert(h.downgrade());
                        h
                    }
                }
                state_map::StateMapEntry::Vacant(e) => {
                    let h = cursor_impl(id);
                    e.insert(h.downgrade());
                    h
                }
            });
            WIDGET.push_var_handle(h);
        }
        UiNodeOp::Info { info } => {
            info.set_meta(*WIDGET_CURSOR_ID, cursor.current_context());
        }
        _ => {}
    })
}
fn cursor_impl(id: WindowId) -> VarHandle {
    let mut _binding = VarHandle::dummy();
    let mut current_top = None;
    MOUSE_HOVERED_EVENT.hook(move |args| {
        if let Some(t) = &args.target
            && t.window_id() == id
            && let Some(info) = WINDOWS.widget_tree(id).unwrap().get(t.widget_id())
        {
            let mut vars = None;
            macro_rules! vars {
                () => {
                    vars.get_or_insert_with(|| WINDOWS.vars(id).unwrap())
                };
            }
            for info in info.self_and_ancestors() {
                if let Some(cap) = &args.capture
                    && !cap.allows((id, info.id()))
                {
                    continue;
                }
                if let Some(cursor) = info.meta().get(*WIDGET_CURSOR_ID) {
                    let top = Some(info.id());
                    if current_top != top {
                        current_top = top;

                        _binding = cursor.set_bind(&vars!().cursor());
                    }
                    return true;
                }
            }

            if current_top.is_some() {
                _binding = VarHandle::dummy();
                vars!().cursor().set(CursorIcon::Default);
            }
        }
        true
    })
}

/// Defines how click events are generated for the widget.
///
/// Setting this to `None` will cause the widget to inherit the parent mode, or [`ClickMode::default`] if
/// no parent sets the click mode.
///
/// [`ClickMode::default`]: zng_ext_input::mouse::ClickMode::default
#[property(CONTEXT, default(None))]
pub fn click_mode(child: impl IntoUiNode, mode: impl IntoVar<Option<ClickMode>>) -> UiNode {
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
