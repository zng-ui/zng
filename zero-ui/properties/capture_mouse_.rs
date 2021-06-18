use crate::core::mouse::{CaptureMode, MouseDownEvent, MouseExt};
use crate::prelude::new_property::*;

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
#[property(context, default(false))]
pub fn capture_mouse(child: impl UiNode, mode: impl IntoVar<CaptureMode>) -> impl UiNode {
    struct CaptureMouseNode<C: UiNode, M: Var<CaptureMode>> {
        child: C,
        mode: M,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, M: Var<CaptureMode>> UiNode for CaptureMouseNode<C, M> {
        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = MouseDownEvent.update(args) {
                if IsEnabled::get(ctx) && args.concerns_widget(ctx) {
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
            if let Some(new_mode) = self.mode.copy_new(ctx) {
                if IsEnabled::get(ctx) {
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
