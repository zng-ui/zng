use crate::core::context::StateMapEntry;
use crate::core::mouse::{CaptureMode, MouseExt, MouseInputEvent};
use crate::prelude::new_property::*;

use std::cell::Cell;
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
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.mode).event(MouseInputEvent);

            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = MouseInputEvent.update(args) {
                if args.is_mouse_down() && args.concerns_widget(ctx) {
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
                    .find(ctx.path.widget_id())
                    .map(|w| w.allow_interaction())
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

/// Only allow interaction inside the widget and descendants.
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
        struct ModalWidget: Rc<Cell<WidgetId>>;
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, E: Var<bool>> UiNode for ModalNode<C, E> {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if self.enabled.copy(ctx) {
                let widget_id = ctx.path.widget_id();

                match ctx.update_state.entry(ModalWidget) {
                    StateMapEntry::Vacant(e) => {
                        let widget = Rc::new(Cell::new(widget_id));
                        e.insert(widget.clone());
                        info.push_interactive_filter(move |a| a.info.self_and_ancestors().any(|w| w.widget_id() == widget.get()));
                    }
                    StateMapEntry::Occupied(e) => {
                        e.get().set(widget_id);
                    }
                }
            }
            self.child.info(ctx, info);
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
