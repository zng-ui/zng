use crate::core::{
    mouse::MOUSE_INPUT_EVENT,
    pointer_capture::{CaptureMode, POINTER_CAPTURE},
    task::parking_lot::Mutex,
    IdSet,
};
use crate::prelude::new_property::*;

use std::sync::Arc;

/// Capture mouse and touch for the widget on press.
///
/// The capture happens only if any mouse button or touch is pressed on the window and the `mode` is [`Widget`] or [`Subtree`].
///
/// Captures are released when all mouse buttons and touch contacts stop being pressed on the window.
/// The capture is also released back to window if the `mode` changes to [`Window`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude::new_widget::*;
/// # use zero_ui::properties::capture_pointer;
/// #[widget($crate::Button)]
/// pub struct Button(Container);
/// impl Button {
///     fn widget_intrinsic(&mut self) {
///         widget_set! {
///             self;
///             // Mouse does not interact with other widgets when pressed in the widget.
///             capture_pointer = true; //true == CaptureMode::Widget;
///         }
///     }
/// }
/// ```
///
/// [`Widget`]: CaptureMode::Widget
/// [`Subtree`]: CaptureMode::Subtree
/// [`Window`]: CaptureMode::Window
#[property(CONTEXT, default(false))]
pub fn capture_pointer(child: impl UiNode, mode: impl IntoVar<CaptureMode>) -> impl UiNode {
    let mode = mode.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&MOUSE_INPUT_EVENT).sub_var(&mode);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                if args.is_mouse_down() {
                    let widget_id = WIDGET.id();

                    match mode.get() {
                        CaptureMode::Widget => {
                            POINTER_CAPTURE.capture_widget(widget_id);
                        }
                        CaptureMode::Subtree => {
                            POINTER_CAPTURE.capture_subtree(widget_id);
                        }
                        CaptureMode::Window => (),
                    }
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(new_mode) = mode.get_new() {
                let tree = WINDOW.info();
                let widget_id = WIDGET.id();
                if tree.get(widget_id).map(|w| w.interactivity().is_enabled()).unwrap_or(false) {
                    if let Some(current) = POINTER_CAPTURE.current_capture().get() {
                        if current.target.widget_id() == widget_id {
                            // If mode updated and we are capturing the mouse:
                            match new_mode {
                                CaptureMode::Widget => POINTER_CAPTURE.capture_widget(widget_id),
                                CaptureMode::Subtree => POINTER_CAPTURE.capture_subtree(widget_id),
                                CaptureMode::Window => POINTER_CAPTURE.release_capture(),
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    })
}

/// Capture mouse and touch for the widget on init.
///
/// The capture happens only if any mouse button or touch is pressed on the window and the `mode` is [`Widget`] or [`Subtree`].
///
/// Mouse captures are released when all mouse buttons stop being pressed on the window.
/// The capture is also released back to window if the `mode` changes to [`Window`] when the mouse is captured for the widget.
///
/// [`Widget`]: CaptureMode::Widget
/// [`Subtree`]: CaptureMode::Subtree
/// [`Window`]: CaptureMode::Window
#[property(CONTEXT, default(false))]
pub fn capture_pointer_on_init(child: impl UiNode, mode: impl IntoVar<CaptureMode>) -> impl UiNode {
    let mode = mode.into_var();
    let mut capture = true;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&mode);
            capture = true; // wait for info
        }
        UiNodeOp::Info { .. } => {
            if std::mem::take(&mut capture) {
                let widget_id = WIDGET.id();

                match mode.get() {
                    CaptureMode::Widget => {
                        POINTER_CAPTURE.capture_widget(widget_id);
                    }
                    CaptureMode::Subtree => {
                        POINTER_CAPTURE.capture_subtree(widget_id);
                    }
                    CaptureMode::Window => (),
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(new_mode) = mode.get_new() {
                let tree = WINDOW.info();
                let widget_id = WIDGET.id();
                if tree.get(widget_id).map(|w| w.interactivity().is_enabled()).unwrap_or(false) {
                    if let Some(current) = POINTER_CAPTURE.current_capture().get() {
                        if current.target.widget_id() == widget_id {
                            // If mode updated and we are capturing the mouse:
                            match new_mode {
                                CaptureMode::Widget => POINTER_CAPTURE.capture_widget(widget_id),
                                CaptureMode::Subtree => POINTER_CAPTURE.capture_subtree(widget_id),
                                CaptureMode::Window => POINTER_CAPTURE.release_capture(),
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    })
}

/// Only allow interaction inside the widget, descendants and ancestors.
///
/// When modal mode is enabled in a widget only it and widget descendants [allows interaction], all other widgets behave as if disabled, but
/// without the visual indication of disabled. This property is a building block for modal overlay widgets.
///
/// Only one widget can be the modal at a time, if multiple widgets set `modal = true` only the last one by traversal order is modal.
///
/// [allows interaction]: crate::core::widget_info::WidgetInfo::interactivity
#[property(CONTEXT, default(false))]
pub fn modal(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    static MODAL_WIDGETS: StaticStateId<Arc<Mutex<ModalWidgetsData>>> = StaticStateId::new_unique();
    #[derive(Default)]
    struct ModalWidgetsData {
        widgets: IdSet<WidgetId>,
        last_in_tree: Option<WidgetId>,
    }
    let enabled = enabled.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&enabled);
            WINDOW.init_state_default(&MODAL_WIDGETS); // insert window state
        }
        UiNodeOp::Deinit => {
            let mws = WINDOW.req_state(&MODAL_WIDGETS);

            // maybe unregister.
            let mut mws = mws.lock();
            let widget_id = WIDGET.id();
            if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                mws.last_in_tree = None;
            }
        }
        UiNodeOp::Info { info } => {
            let mws = WINDOW.req_state(&MODAL_WIDGETS);

            if enabled.get() {
                let insert_filter = {
                    let mut mws = mws.lock();
                    if mws.widgets.insert(WIDGET.id()) {
                        mws.last_in_tree = None;
                        mws.widgets.len() == 1
                    } else {
                        false
                    }
                };
                if insert_filter {
                    // just registered and we are the first, insert the filter:

                    info.push_interactivity_filter(clmv!(mws, |a| {
                        let mut mws = mws.lock();

                        // caches the top-most modal.
                        if mws.last_in_tree.is_none() {
                            match mws.widgets.len() {
                                0 => unreachable!(),
                                1 => {
                                    // only one modal
                                    mws.last_in_tree = mws.widgets.iter().next().copied();
                                }
                                _ => {
                                    // multiple modals, find the *top* one.
                                    let mut found = 0;
                                    for info in a.info.root().self_and_descendants() {
                                        if mws.widgets.contains(&info.id()) {
                                            mws.last_in_tree = Some(info.id());
                                            found += 1;
                                            if found == mws.widgets.len() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            };
                        }

                        // filter, only allows inside self inclusive, and ancestors.
                        let modal = mws.last_in_tree.unwrap();
                        if a.info.self_and_ancestors().any(|w| w.id() == modal) || a.info.self_and_descendants().any(|w| w.id() == modal) {
                            Interactivity::ENABLED
                        } else {
                            Interactivity::BLOCKED
                        }
                    }));
                }
            } else {
                // maybe unregister.
                let mut mws = mws.lock();
                let widget_id = WIDGET.id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }
        }
        _ => {}
    })
}
