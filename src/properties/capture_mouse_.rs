use crate::core::mouse::{CaptureMode, Mouse, MouseDownEvent, MouseInputArgs};
use crate::prelude::new_property::*;

struct CaptureMouseNode<C: UiNode, M: Var<CaptureMode>> {
    child: C,
    mode: M,
    mouse_down: EventListener<MouseInputArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode, M: Var<CaptureMode>> UiNode for CaptureMouseNode<C, M> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.mouse_down = ctx.events.listen::<MouseDownEvent>();
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.mouse_down = MouseDownEvent::never();
        self.child.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.mouse_down.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            let mouse = ctx.services.req::<Mouse>();
            let widget_id = ctx.path.widget_id();

            match *self.mode.get(ctx.vars) {
                CaptureMode::Widget => {
                    mouse.capture_widget(widget_id);
                }
                CaptureMode::Subtree => {
                    mouse.capture_subtree(widget_id);
                }
                CaptureMode::Window => (),
            }
        } else if let Some(&new_mode) = self.mode.get_new(ctx.vars) {
            let mouse = ctx.services.req::<Mouse>();
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
        self.child.update(ctx);
    }
}

/// Capture mouse for the widget on mouse down.
///
/// The mouse is captured when the widget gets the first mouse down and the `mode` is
/// [`Widget`](CaptureMode::Widget) or [`Subtree`](CaptureMode::Subtree).
///
/// The capture is released back to window if the `mode` changes to [`Window`](CaptureMode::Window) when
/// the mouse is captured for the widget.
///
/// # Example
///
/// ```
/// # use zero_ui::prelude::new_widget::*;
/// # use zero_ui::properties::capture_mouse;
/// widget! {
///     pub button: container;
///
///     default {
///         /// Mouse does not interact with other widgets when pressed in a button.
///         capture_mouse: true; //true == CaptureMode::Widget;
///     }
/// }
/// ```
#[property(context)]
pub fn capture_mouse(child: impl UiNode, mode: impl IntoVar<CaptureMode>) -> impl UiNode {
    CaptureMouseNode {
        child,
        mode: mode.into_var(),
        mouse_down: MouseDownEvent::never(),
    }
}
