use crate::core::mouse::{CaptureMode, Mouse, MOUSE_INPUT_EVENT};
use crate::prelude::new_property::*;

use crate::core::task::Mutex;
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
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_event(&MOUSE_INPUT_EVENT);
            self.init_handles(ctx);
            self.child.init(ctx);
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                if args.is_mouse_down() {
                    let mouse = Mouse::req(ctx.services);
                    let widget_id = ctx.path.widget_id();

                    match self.mode.get() {
                        CaptureMode::Widget => {
                            mouse.capture_widget(widget_id);
                        }
                        CaptureMode::Subtree => {
                            mouse.capture_subtree(widget_id);
                        }
                        CaptureMode::Window => (),
                    }
                }
            }
            self.child.event(ctx, update);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(new_mode) = self.mode.get_new(ctx) {
                if ctx
                    .info_tree
                    .get(ctx.path.widget_id())
                    .map(|w| w.interactivity().is_enabled())
                    .unwrap_or(false)
                {
                    let mouse = Mouse::req(ctx.services);
                    let widget_id = ctx.path.widget_id();
                    if let Some((current, _)) = mouse.current_capture() {
                        if current.widget_id() == widget_id {
                            // If mode updated and we are capturing the mouse:
                            match new_mode {
                                CaptureMode::Widget => mouse.capture_widget(widget_id),
                                CaptureMode::Subtree => mouse.capture_subtree(widget_id),
                                CaptureMode::Window => mouse.release_capture(),
                            }
                        }
                    }
                }
            }
            self.child.update(ctx, updates);
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
        widgets: linear_map::set::LinearSet<WidgetId>,
        last_in_tree: Option<WidgetId>,
    }

    #[ui_node(struct ModalNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,
    })]
    impl UiNode for ModalNode {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            let mws = ctx.window_state.get(&MODAL_WIDGETS).unwrap();

            if self.enabled.get() {
                let insert_filter = {
                    let mut mws = mws.lock();
                    if mws.widgets.insert(ctx.path.widget_id()) {
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
                                        if mws.widgets.contains(&info.widget_id()) {
                                            mws.last_in_tree = Some(info.widget_id());
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
                        if a.info.self_and_ancestors().any(|w| w.widget_id() == modal)
                            || a.info.self_and_descendants().any(|w| w.widget_id() == modal)
                        {
                            Interactivity::ENABLED
                        } else {
                            Interactivity::BLOCKED
                        }
                    }));
                }
            } else {
                // maybe unregister.
                let mut mws = mws.lock();
                let widget_id = ctx.path.widget_id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }

            self.child.info(ctx, info);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.window_state.entry(&MODAL_WIDGETS).or_default(); // insert window state
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            {
                let mws = ctx.window_state.get(&MODAL_WIDGETS).unwrap();

                // maybe unregister.
                let mut mws = mws.lock();
                let widget_id = ctx.path.widget_id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }
            self.child.deinit(ctx)
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.enabled.is_new(ctx) {
                ctx.updates.info();
            }

            self.child.update(ctx, updates);
        }
    }
    ModalNode {
        child,
        enabled: enabled.into_var(),
    }
}
