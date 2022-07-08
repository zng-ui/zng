use crate::core::mouse::{CaptureMode, MouseExt, MouseInputEvent};
use crate::prelude::new_property::*;

use std::cell::RefCell;
use std::rc::Rc;

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
#[property(context, default(false))]
pub fn capture_mouse(child: impl UiNode, mode: impl IntoVar<CaptureMode>) -> impl UiNode {
    struct CaptureMouseNode<C: UiNode, M: Var<CaptureMode>> {
        child: C,
        mode: M,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, M: Var<CaptureMode>> UiNode for CaptureMouseNode<C, M> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.mode).event(MouseInputEvent);

            self.child.subscriptions(ctx, subs);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = MouseInputEvent.update(args) {
                if args.is_mouse_down() {
                    let mouse = ctx.services.mouse();
                    let widget_id = ctx.path.widget_id();

                    match self.mode.copy(ctx.vars) {
                        CaptureMode::Widget => {
                            mouse.capture_widget(widget_id);
                        }
                        CaptureMode::Subtree => {
                            mouse.capture_subtree(widget_id);
                        }
                        CaptureMode::Window => (),
                    }
                }

                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(new_mode) = self.mode.copy_new(ctx.vars) {
                if ctx
                    .info_tree
                    .get(ctx.path.widget_id())
                    .map(|w| w.interactivity().is_enabled())
                    .unwrap_or(false)
                {
                    let mouse = ctx.services.mouse();
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
            self.child.update(ctx);
        }
    }
    CaptureMouseNode {
        child,
        mode: mode.into_var(),
    }
}

/// Only allow interaction inside the widget, descendants and ancestors.
///
/// When modal mode is enabled in a widget only it and widget descendants [`allow_interaction`], all other widgets behave as if disabled, but
/// without the visual indication of disabled. This property is a building block for modal overlay widgets.
///
/// Only one widget can be the modal at a time, if multiple widgets set `modal = true` only the last one by traversal order is modal, this
/// is by design to support dialog overlays that open another dialog overlay.
///
/// [`allow_interaction`]: crate::core::widget_info::WidgetInfo::allow_interaction
#[property(context, default(false))]
pub fn modal(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    struct ModalNode<C, E> {
        child: C,
        enabled: E,
    }
    state_key! {
        struct ModalWidgets: Rc<RefCell<ModalWidgetsData>>;
    }
    #[derive(Default)]
    struct ModalWidgetsData {
        widgets: linear_map::set::LinearSet<WidgetId>,
        last_in_tree: Option<WidgetId>,
    }

    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for ModalNode<C, E> {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            let mws = ctx.window_state.get(ModalWidgets).unwrap();

            if self.enabled.copy(ctx) {
                let insert_filter = {
                    let mut mws = mws.borrow_mut();
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
                        let mut mws = mws.borrow_mut();

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
                            || a.info.self_and_descendants().any(|w| (w.widget_id() == modal).into())
                        {
                            Interactivity::ENABLED
                        } else {
                            Interactivity::BLOCKED
                        }
                    }));
                }
            } else {
                // maybe unregister.
                let mut mws = mws.borrow_mut();
                let widget_id = ctx.path.widget_id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }

            self.child.info(ctx, info);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.window_state.entry(ModalWidgets).or_default(); // insert window state
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            {
                let mws = ctx.window_state.get(ModalWidgets).unwrap();

                // maybe unregister.
                let mut mws = mws.borrow_mut();
                let widget_id = ctx.path.widget_id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }
            self.child.deinit(ctx)
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.enabled.is_new(ctx) {
                ctx.updates.info();
            }

            self.child.update(ctx);
        }
    }
    ModalNode {
        child,
        enabled: enabled.into_var(),
    }
}
