use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use zng_ext_input::mouse::{ClickMode, MOUSE_HOVERED_EVENT, WidgetInfoBuilderMouseExt as _};
use zng_ext_window::WINDOW_Ext as _;
use zng_wgt::prelude::*;

pub use zng_view_api::window::CursorIcon;

pub use zng_ext_window::{CursorImg, CursorSource};

context_local! {
    static CHILD_SETS_CURSOR: AtomicBool = AtomicBool::new(false);
}

/// Sets the mouse pointer cursor displayed when hovering the widget.
///
/// You can set this property to a [`CursorIcon`] for a named platform dependent icon, [`CursorImg`] for a custom image,
/// or to `false` that converts to [`CursorSource::Hidden`].
#[property(CONTEXT, default(CursorIcon::Default))]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorSource>) -> impl UiNode {
    let cursor = cursor.into_var();
    let mut binding = None;
    let mut child_sets_ctx = None::<Arc<AtomicBool>>;

    match_node(child, move |c, op| {
        let mut unbind_restore = false;
        match op {
            UiNodeOp::Init => {
                WIDGET.sub_event(&MOUSE_HOVERED_EVENT);
            }
            UiNodeOp::Deinit => {
                unbind_restore = true;
                child_sets_ctx = None;
            }
            UiNodeOp::Event { update } => {
                if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                    let mut bind = false;
                    if args.is_over() {
                        if child_sets_ctx.is_none() {
                            child_sets_ctx = Some(Arc::new(AtomicBool::new(false)));
                        }

                        // if a child also sets cursor, it will flag our context.
                        CHILD_SETS_CURSOR.with_context(&mut child_sets_ctx, || c.event(update));

                        if !child_sets_ctx.as_ref().unwrap().swap(false, Ordering::Relaxed) {
                            // no descendant sets cursor, it is ours.
                            bind = true;
                        }

                        // flag parent context.
                        CHILD_SETS_CURSOR.get().store(true, Ordering::Relaxed);
                    }

                    if bind {
                        if binding.is_none() {
                            // we are not already set, setup binding.
                            binding = Some(cursor.set_bind(&WINDOW.vars().cursor()));
                        }
                    } else {
                        unbind_restore = true;
                    }
                }
            }
            _ => {}
        }

        // restore to default, if not set to other value already
        if unbind_restore && binding.is_some() {
            binding = None;
            let value = cursor.get();
            WINDOW.vars().cursor().modify(move |c| {
                if c.value() == &value {
                    **c = CursorIcon::Default.into();
                }
            });
        }
    })
}

/// Defines how click events are generated for the widget.
///
/// Setting this to `None` will cause the widget to inherit the parent mode, or [`ClickMode::default`] if
/// no parent sets the click mode.
///
/// [`ClickMode::default`]: zng_ext_input::mouse::ClickMode::default
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
