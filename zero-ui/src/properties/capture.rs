use crate::core::{
    mouse::{CaptureMode, MOUSE, MOUSE_INPUT_EVENT},
    task::parking_lot::Mutex,
    IdSet,
};
use crate::prelude::new_property::*;

use std::sync::Arc;

/// Capture mouse for the widget on mouse down.
///
/// The mouse is captured when the widget gets the first mouse down and the `mode` is [`Widget`] or [`Subtree`].
///
/// The capture is released back to window if the `mode` changes to [`Window`] when the mouse is captured for the widget.
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude::new_widget::*;
/// # use zero_ui::properties::capture_mouse;
/// #[widget($crate::button)]
/// pub mod button {
///     use super::*;
///     inherit!(container);
///     properties! {
///         /// Mouse does not interact with other widgets when pressed in a button.
///         capture_mouse = true; //true == CaptureMode::Widget;
///     }
/// }
/// ```
///
/// [`Widget`]: CaptureMode::Widget
/// [`Subtree`]: CaptureMode::Subtree
/// [`Window`]: CaptureMode::Window
#[property(CONTEXT, default(false))]
pub fn capture_mouse(child: impl UiNode, mode: impl IntoVar<CaptureMode>) -> impl UiNode {
    #[ui_node(struct CaptureMouseNode {
        child: impl UiNode,
        #[var] mode: impl Var<CaptureMode>,
    })]
    impl UiNode for CaptureMouseNode {
        fn init(&mut self) {
            WIDGET.sub_event(&MOUSE_INPUT_EVENT);
            self.auto_subs();
            self.child.init();
        }

        fn event(&mut self, update: &mut EventUpdate) {
            if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                if args.is_mouse_down() {
                    let widget_id = WIDGET.id();

                    match self.mode.get() {
                        CaptureMode::Widget => {
                            MOUSE.capture_widget(widget_id);
                        }
                        CaptureMode::Subtree => {
                            MOUSE.capture_subtree(widget_id);
                        }
                        CaptureMode::Window => (),
                    }
                }
            }
            self.child.event(update);
        }

        fn update(&mut self, updates: &mut WidgetUpdates) {
            if let Some(new_mode) = self.mode.get_new() {
                let tree = WINDOW.widget_tree();
                let widget_id = WIDGET.id();
                if tree.get(widget_id).map(|w| w.interactivity().is_enabled()).unwrap_or(false) {
                    if let Some((current, _)) = MOUSE.current_capture() {
                        if current.widget_id() == widget_id {
                            // If mode updated and we are capturing the mouse:
                            match new_mode {
                                CaptureMode::Widget => MOUSE.capture_widget(widget_id),
                                CaptureMode::Subtree => MOUSE.capture_subtree(widget_id),
                                CaptureMode::Window => MOUSE.release_capture(),
                            }
                        }
                    }
                }
            }
            self.child.update(updates);
        }
    }
    CaptureMouseNode {
        child,
        mode: mode.into_var(),
    }
}

/// Only allow interaction inside the widget, descendants and ancestors.
///
/// When modal mode is enabled in a widget only it and widget descendants [allows interaction], all other widgets behave as if disabled, but
/// without the visual indication of disabled. This property is a building block for modal overlay widgets.
///
/// Only one widget can be the modal at a time, if multiple widgets set `modal = true` only the last one by traversal order is modal, this
/// is by design to support dialog overlays that open another dialog overlay.
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

    #[ui_node(struct ModalNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,
    })]
    impl UiNode for ModalNode {
        fn info(&self, info: &mut WidgetInfoBuilder) {
            let mws = WINDOW.req_state(&MODAL_WIDGETS);

            if self.enabled.get() {
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

                    info.push_interactivity_filter(clone_move!(mws, |a| {
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

            self.child.info(info);
        }

        fn init(&mut self) {
            self.auto_subs();
            WINDOW.init_state_default(&MODAL_WIDGETS); // insert window state
            self.child.init();
        }

        fn deinit(&mut self) {
            {
                let mws = WINDOW.req_state(&MODAL_WIDGETS);

                // maybe unregister.
                let mut mws = mws.lock();
                let widget_id = WIDGET.id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }
            self.child.deinit()
        }

        fn update(&mut self, updates: &mut WidgetUpdates) {
            if self.enabled.is_new() {
                WIDGET.update_info();
            }

            self.child.update(updates);
        }
    }
    ModalNode {
        child,
        enabled: enabled.into_var(),
    }
}
