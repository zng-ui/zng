//! Event handler properties, [`on_click`], [`on_key_down`], [`on_focus`] and more.

use crate::core::context::*;
use crate::core::event::*;
use crate::core::focus::*;
use crate::core::gesture::*;
use crate::core::keyboard::*;
use crate::core::mouse::*;
use crate::core::profiler::profile_scope;
use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::core::{impl_ui_node, property};
use crate::properties::IsEnabled;

struct OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> OnEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.listener = E::never();
        self.child.deinit(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if !E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.child.update_hp(ctx);

        if E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }
    }

    fn do_update(&mut self, ctx: &mut WidgetContext) {
        if self.listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            for args in self.listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    profile_scope!("on_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

struct OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool,
    H: FnMut(&mut WidgetContext, &E::Args),
{
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    filter: F,
    handler: H,
}
#[impl_ui_node(child)]
impl<C, E, F, H> OnPreviewEventNode<C, E, F, H>
where
    C: UiNode,
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    H: FnMut(&mut WidgetContext, &E::Args) + 'static,
{
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.listener = E::never();
        self.child.deinit(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if !E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }

        self.child.update(ctx);
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        if E::IS_HIGH_PRESSURE {
            self.do_update(ctx)
        }

        self.child.update_hp(ctx);
    }

    fn do_update(&mut self, ctx: &mut WidgetContext) {
        if self.listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            for args in self.listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && (self.filter)(ctx, args) {
                    profile_scope!("on_preview_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

/// Helper for declaring event properties.
///
/// # Route
///
/// The event is raised after the [preview](on_preview_event) version. If the event targets a path the target
/// widget is notified first followed by every parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
/// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
///
/// This route is also called *bubbling*.
///
/// # Example
/// ```
/// # fn main() { }
/// use zero_ui::properties::events::on_event;
/// use zero_ui::core::{UiNode, keyboard::{KeyDownEvent, KeyInputArgs}, property};
/// use zero_ui::core::context::WidgetContext;
///
/// /// Sets an event listener for the [`KeyDownEvent`].
/// #[property(event)]
/// pub fn on_key_down(
///    child: impl UiNode,
///    handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static
/// ) -> impl UiNode {
///     on_event(child, KeyDownEvent, handler)
/// }
/// ```
#[inline]
pub fn on_event<E: Event>(child: impl UiNode, event: E, handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static) -> impl UiNode {
    on_event_filtered(child, event, |ctx, args| args.concerns_widget(ctx), handler)
}

/// Helper for declaring event properties with a custom event filter.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested. It
/// must return `true` if the event arguments are relevant in the context of the widget. If it returns `true`
/// the `handler` closure is called. If the widget is [disabled](IsEnabled) the event is not notified.
///
/// # Route
///
/// The event route is similar to [`on_event`], child widgets get first chance of handling the event. In-fact
/// if you use the filter `|ctx, args| args.concerns_widget(ctx)` it will behave exactly the same.
#[inline]
pub fn on_event_filtered<E: Event>(
    child: impl UiNode,
    event: E,
    filter: impl FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    OnEventNode {
        child,
        _event: event,
        listener: E::never(),
        filter,
        handler,
    }
}

/// Helper for declaring preview event properties.
///
/// # Preview
///
/// Preview events are fired before the main event ([`on_event`]). If the event targets a path the root parent
/// is notified first, followed by every parent down to the target. If [`stop_propagation`](EventArgs::stop_propagation) is
/// requested the event is not notified further and the main event handlers are also not notified.
///  If the widget is [disabled](IsEnabled) the event is not notified.
///
/// This route is also called *tunneling* or *capturing*.
///
/// # Example
/// ```
/// # fn main() { }
/// use zero_ui::properties::events::on_preview_event;
/// use zero_ui::core::{UiNode, keyboard::{KeyDownEvent, KeyInputArgs}, property};
/// use zero_ui::core::context::WidgetContext;
///
/// /// Sets an event listener for the [`KeyDownEvent`].
/// #[property(event)]
/// pub fn on_preview_key_down(
///    child: impl UiNode,
///    handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static
/// ) -> impl UiNode {
///     on_preview_event(child, KeyDownEvent, handler)
/// }
/// ```
#[inline]
pub fn on_preview_event<E: Event>(
    child: impl UiNode,
    event: E,

    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    on_preview_event_filtered(child, event, |ctx, args| args.concerns_widget(ctx), handler)
}

/// Helper for declaring preview event properties with a custom filter.
///
/// # Filter
///
/// The `filter` predicate is called if [`stop_propagation`](EventArgs::stop_propagation) is not requested. It
/// must return `true` if the event arguments are relevant in the context of the widget. If it returns `true`
/// the `handler` closure is called.  If the widget is [disabled](IsEnabled) the event is not notified.
///
/// # Route
///
/// The event route is similar to [`on_preview_event`], parent widgets get first chance of handling the event.
/// In-fact if you use the filter `|ctx, args| args.concerns_widget(ctx)` it will behave exactly the same.
pub fn on_preview_event_filtered<E: Event>(
    child: impl UiNode,
    event: E,
    filter: impl FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static,
) -> impl UiNode {
    OnPreviewEventNode {
        child,
        _event: event,
        listener: E::never(),
        filter,
        handler,
    }
}

#[property(event)]
pub fn on_key_input(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static) -> impl UiNode {
    on_event(child, KeyInputEvent, handler)
}
#[property(event)]
pub fn on_preview_key_input(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, KeyInputEvent, handler)
}

#[property(event)]
pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static) -> impl UiNode {
    on_event(child, KeyDownEvent, handler)
}
#[property(event)]
pub fn on_preview_key_down(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, KeyDownEvent, handler)
}

#[property(event)]
pub fn on_key_up(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static) -> impl UiNode {
    on_event(child, KeyUpEvent, handler)
}
#[property(event)]
pub fn on_preview_key_up(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, KeyUpEvent, handler)
}

#[property(event)]
pub fn on_char_input(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &CharInputArgs) + 'static) -> impl UiNode {
    on_event(child, CharInputEvent, handler)
}
#[property(event)]
pub fn on_preview_char_input(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &CharInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, CharInputEvent, handler)
}

#[property(event)]
pub fn on_mouse_move(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseMoveArgs) + 'static) -> impl UiNode {
    on_event(child, MouseMoveEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_move(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseMoveArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseMoveEvent, handler)
}

#[property(event)]
pub fn on_mouse_input(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseInputArgs) + 'static) -> impl UiNode {
    on_event(child, MouseInputEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_input(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseInputEvent, handler)
}

#[property(event)]
pub fn on_mouse_down(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseInputArgs) + 'static) -> impl UiNode {
    on_event(child, MouseDownEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_down(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseDownEvent, handler)
}

#[property(event)]
pub fn on_mouse_up(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseInputArgs) + 'static) -> impl UiNode {
    on_event(child, MouseUpEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_up(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseInputArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseUpEvent, handler)
}

#[property(event)]
pub fn on_mouse_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static) -> impl UiNode {
    on_event(child, MouseClickEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_single_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static) -> impl UiNode {
    on_event(child, MouseDoubleClickEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_single_click(
    child: impl UiNode,
    handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static,
) -> impl UiNode {
    on_preview_event(child, MouseDoubleClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_double_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static) -> impl UiNode {
    on_event(child, MouseSingleClickEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_double_click(
    child: impl UiNode,
    handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static,
) -> impl UiNode {
    on_preview_event(child, MouseSingleClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_triple_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static) -> impl UiNode {
    on_event(child, MouseTripleClickEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_triple_click(
    child: impl UiNode,
    handler: impl FnMut(&mut WidgetContext, &MouseClickArgs) + 'static,
) -> impl UiNode {
    on_preview_event(child, MouseTripleClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_enter(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseHoverArgs) + 'static) -> impl UiNode {
    on_event(child, MouseEnterEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_enter(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseHoverArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseEnterEvent, handler)
}

#[property(event)]
pub fn on_mouse_leave(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseHoverArgs) + 'static) -> impl UiNode {
    on_event(child, MouseLeaveEvent, handler)
}
#[property(event)]
pub fn on_preview_mouse_leave(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseHoverArgs) + 'static) -> impl UiNode {
    on_preview_event(child, MouseLeaveEvent, handler)
}

/// Adds a handler for clicks in the widget from any mouse button.
#[property(event)]
pub fn on_any_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event(child, ClickEvent, handler)
}
#[property(event)]
pub fn on_preview_any_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event(child, ClickEvent, handler)
}

/// Adds a handler for clicks in the widget from the left mouse button.
#[property(event)]
pub fn on_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, ClickEvent, is_primary_predicate, handler)
}
#[property(event)]
pub fn on_preview_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, ClickEvent, is_primary_predicate, handler)
}
fn is_primary_predicate(ctx: &mut WidgetContext, args: &ClickArgs) -> bool {
    args.concerns_widget(ctx) && args.is_primary()
}

#[property(event)]
pub fn on_any_single_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event(child, SingleClickEvent, handler)
}
#[property(event)]
pub fn on_preview_any_single_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event(child, SingleClickEvent, handler)
}
#[property(event)]
pub fn on_single_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, SingleClickEvent, is_primary_predicate, handler)
}
#[property(event)]
pub fn on_preview_single_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, SingleClickEvent, is_primary_predicate, handler)
}

#[property(event)]
pub fn on_any_double_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event(child, DoubleClickEvent, handler)
}
#[property(event)]
pub fn on_preview_any_double_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event(child, DoubleClickEvent, handler)
}
#[property(event)]
pub fn on_double_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, DoubleClickEvent, is_primary_predicate, handler)
}
#[property(event)]
pub fn on_preview_double_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, DoubleClickEvent, is_primary_predicate, handler)
}

#[property(event)]
pub fn on_any_triple_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event(child, TripleClickEvent, handler)
}
#[property(event)]
pub fn on_preview_any_triple_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event(child, TripleClickEvent, handler)
}
#[property(event)]
pub fn on_triple_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, TripleClickEvent, is_primary_predicate, handler)
}
#[property(event)]
pub fn on_preview_triple_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, TripleClickEvent, is_primary_predicate, handler)
}

#[property(event)]
pub fn on_context_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, SingleClickEvent, is_context_click_predicate, handler)
}
#[property(event)]
pub fn on_preview_context_click(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ClickArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, SingleClickEvent, is_context_click_predicate, handler)
}
fn is_context_click_predicate(ctx: &mut WidgetContext, args: &ClickArgs) -> bool {
    args.concerns_widget(ctx) && args.is_context()
}

#[property(event)]
pub fn on_shortcut(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ShortcutArgs) + 'static) -> impl UiNode {
    on_event(child, ShortcutEvent, handler)
}
#[property(event)]
pub fn on_preview_shortcut(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &ShortcutArgs) + 'static) -> impl UiNode {
    on_preview_event(child, ShortcutEvent, handler)
}

/// Focus changed in the widget or its descendants.
#[property(event)]
pub fn on_focus_changed(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_event(child, FocusChangedEvent, handler)
}
#[property(event)]
pub fn on_preview_focus_changed(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_preview_event(child, FocusChangedEvent, handler)
}

/// Widget got direct keyboard focus.
#[property(event)]
pub fn on_focus(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, FocusChangedEvent, focus_predicate, handler)
}
#[property(event)]
pub fn on_preview_focus(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, FocusChangedEvent, focus_predicate, handler)
}
fn focus_predicate(ctx: &mut WidgetContext, args: &FocusChangedArgs) -> bool {
    args.new_focus
        .as_ref()
        .map(|p| p.widget_id() == ctx.path.widget_id())
        .unwrap_or_default()
}

/// Widget lost direct keyboard focus.
#[property(event)]
pub fn on_blur(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, FocusChangedEvent, blur_predicate, handler)
}
#[property(event)]
pub fn on_preview_blur(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, FocusChangedEvent, blur_predicate, handler)
}
fn blur_predicate(ctx: &mut WidgetContext, args: &FocusChangedArgs) -> bool {
    args.prev_focus
        .as_ref()
        .map(|p| p.widget_id() == ctx.path.widget_id())
        .unwrap_or_default()
}

/// Widget or one of its descendants got focus.
#[property(event)]
pub fn on_focus_enter(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, FocusChangedEvent, focus_enter_predicate, handler)
}
#[property(event)]
pub fn on_preview_focus_enter(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, FocusChangedEvent, focus_enter_predicate, handler)
}
fn focus_enter_predicate(ctx: &mut WidgetContext, args: &FocusChangedArgs) -> bool {
    // if we are in `new_focus` and are not in `prev_focus`
    args.new_focus
        .as_ref()
        .map(|p| p.contains(ctx.path.widget_id()))
        .unwrap_or_default()
        && args.prev_focus.as_ref().map(|p| !p.contains(ctx.path.widget_id())).unwrap_or(true)
}

/// Widget or one of its descendants lost focus.
#[property(event)]
pub fn on_focus_leave(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, FocusChangedEvent, focus_leave_predicate, handler)
}
#[property(event)]
pub fn on_preview_focus_leave(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &FocusChangedArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, FocusChangedEvent, focus_leave_predicate, handler)
}
fn focus_leave_predicate(ctx: &mut WidgetContext, args: &FocusChangedArgs) -> bool {
    // if we are in `prev_focus` and are not in `new_focus`
    args.prev_focus
        .as_ref()
        .map(|p| p.contains(ctx.path.widget_id()))
        .unwrap_or_default()
        && args.new_focus.as_ref().map(|p| !p.contains(ctx.path.widget_id())).unwrap_or(true)
}

/// Widget got mouse capture.
#[property(event)]
pub fn on_got_capture(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseCaptureArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, MouseCaptureEvent, got_capture_predicate, handler)
}
/// Preview, widget got mouse capture.
#[property(event)]
pub fn on_preview_got_capture(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseCaptureArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, MouseCaptureEvent, got_capture_predicate, handler)
}
fn got_capture_predicate(ctx: &mut WidgetContext, args: &MouseCaptureArgs) -> bool {
    args.is_got(ctx.path.widget_id())
}

/// Widget got mouse capture.
#[property(event)]
pub fn on_lost_capture(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseCaptureArgs) + 'static) -> impl UiNode {
    on_event_filtered(child, MouseCaptureEvent, lost_capture_predicate, handler)
}
/// Preview, widget got mouse capture.
#[property(event)]
pub fn on_preview_lost_capture(child: impl UiNode, handler: impl FnMut(&mut WidgetContext, &MouseCaptureArgs) + 'static) -> impl UiNode {
    on_preview_event_filtered(child, MouseCaptureEvent, lost_capture_predicate, handler)
}
fn lost_capture_predicate(ctx: &mut WidgetContext, args: &MouseCaptureArgs) -> bool {
    args.is_lost(ctx.path.widget_id())
}

macro_rules! on_ctx_mtd {
    ($( $(#[$outer:meta])* struct $OnCtxMtd:ident { fn $mtd:ident } fn $on_mtd:ident;)+) => {$(
        struct $OnCtxMtd<C: UiNode, F: FnMut(&mut WidgetContext)> {
            child: C,
            handler: F
        }

        #[impl_ui_node(child)]
        impl<C: UiNode, F: FnMut(&mut WidgetContext) + 'static> UiNode for $OnCtxMtd<C, F> {
            fn $mtd(&mut self, ctx: &mut WidgetContext) {
                self.child.$mtd(ctx);
                (self.handler)(ctx);
            }
        }

        $(#[$outer])*
        ///
        /// The `handler` is called even when the widget is [disabled](IsEnabled).
        #[property(event)]
        pub fn $on_mtd(child: impl UiNode, handler: impl FnMut(&mut WidgetContext) + 'static) -> impl UiNode {
            $OnCtxMtd {
                child,
                handler
            }
        }
    )+};
}

on_ctx_mtd! {
    /// Called when the widget is initialized.
    struct OnInitNode { fn init } fn on_init;
    struct OnDeinitNode { fn deinit } fn on_denit;
    struct OnUpdateNode { fn update } fn on_update;
    struct OnUpdateHpNode { fn update_hp } fn on_update_hp;
}

struct OnRenderNode<C: UiNode, F: Fn(&mut FrameBuilder)> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameBuilder) + 'static> UiNode for OnRenderNode<C, F> {
    fn render(&self, frame: &mut FrameBuilder) {
        self.child.render(frame);
        (self.handler)(frame);
    }
}

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_render(child: impl UiNode, handler: impl Fn(&mut FrameBuilder) + 'static) -> impl UiNode {
    OnRenderNode { child, handler }
}

#[derive(Debug)]
pub struct OnArrangeArgs<'c> {
    pub final_size: LayoutSize,
    pub ctx: &'c mut LayoutContext,
}

struct OnArrangeNode<C: UiNode, F: FnMut(OnArrangeArgs)> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(OnArrangeArgs) + 'static> UiNode for OnArrangeNode<C, F> {
    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.child.arrange(final_size, ctx);
        (self.handler)(OnArrangeArgs { final_size, ctx });
    }
}

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_arrange(child: impl UiNode, handler: impl FnMut(OnArrangeArgs) + 'static) -> impl UiNode {
    OnArrangeNode { child, handler }
}

#[derive(Debug)]
pub struct OnMeasureArgs<'c> {
    pub available_size: LayoutSize,
    pub desired_size: LayoutSize,
    pub ctx: &'c mut LayoutContext,
}

struct OnMeasureNode<C: UiNode, F: FnMut(OnMeasureArgs) -> LayoutSize> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(OnMeasureArgs) -> LayoutSize + 'static> UiNode for OnMeasureNode<C, F> {
    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let desired_size = self.child.measure(available_size, ctx);

        (self.handler)(OnMeasureArgs {
            available_size,
            desired_size,
            ctx,
        })
    }
}

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_measure(child: impl UiNode, handler: impl FnMut(OnMeasureArgs) -> LayoutSize + 'static) -> impl UiNode {
    OnMeasureNode { child, handler }
}

struct ClickShortcutNode<C: UiNode, S: Var<Shortcuts>> {
    child: C,
    shortcuts: S,
    shortcut_listener: EventListener<ShortcutArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode, S: Var<Shortcuts>> UiNode for ClickShortcutNode<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.shortcut_listener = ctx.events.listen::<ShortcutEvent>();
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.child.deinit(ctx);
        self.shortcut_listener = ShortcutEvent::never();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if self.shortcut_listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            let shortcuts = self.shortcuts.get(ctx.vars);

            for args in self.shortcut_listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && shortcuts.0.contains(&args.shortcut) {
                    // focus on shortcut, if focusable
                    ctx.services
                        .req::<Gestures>()
                        .click_shortcut(ctx.path.window_id(), ctx.path.widget_id(), args.clone());
                    break;
                }
            }
        }
    }
}

/// Keyboard shortcuts that focus and clicks this widget.
///
/// When any of the `shortcuts` is pressed, focus and click this widget.
#[property(context)]
pub fn click_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    ClickShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        shortcut_listener: ShortcutEvent::never(),
    }
}
