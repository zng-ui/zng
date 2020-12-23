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
                    profile_scope!("on_pre_event::<{}>", std::any::type_name::<E>());
                    (self.handler)(ctx, &args);
                }
            }
        }
    }
}

/// Declare one or more event properties.
///
/// Each declaration expands to a pair of properties `on_$event` and `on_pre_$event`. The preview property
/// calls [`on_pre_event_filtered`](zero_ui::properties::events::on_pre_event_filtered),
/// the main event property calls [`on_event_filtered`](zero_ui::properties::events::on_event_filtered).
///
/// # Example
///
/// ```
/// # fn main() { }
/// # use zero_ui::properties::events::event_property;
/// # use zero_ui::core::event::EventArgs;
/// # use zero_ui::core::keyboard::*;
/// event_property! {
///     /// on_key_down docs.
///     pub fn key_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // default filter is |ctx, args| args.concerns_widget(ctx)
///     }
///
///     pub(crate) fn space_down {
///         event: KeyDownEvent,
///         args: KeyInputArgs,
///         // optional filter:
///         filter: |ctx, args| args.concerns_widget(ctx) && args.key == Some(Key::Space),
///     }
/// }
/// ```
///
/// # Filter
///
/// App events can be listened from any `UiNode`. An event property must call the event handler only
/// in contexts where the event is relevant. Some event properties can also specialize further on top
/// of a more general app event. To implement this you can use a filter predicate.
///
/// First [`on_event_filtered`](zero_ui::properties::events::on_event_filtered) filters event that
/// have [stop propagation requested](EventArgs::stop_propagation_requested)
/// requested and widgets context that are [disabled](IsEnabled). After this the filter predicate is called.
///
/// If you don't provide a filter predicate the default [`args.concerns_widget(ctx)`](EventArgs::concerns_widget) is used.
/// So if you want to extend the filter and not fully replace it you must call `args.concerns_widget(ctx)` in your custom filter.
pub use zero_ui_macros::event_property;

/// Helper for declaring event properties.
///
/// # Route
///
/// The event is raised after the [preview](on_pre_event) version. If the event targets a path the target
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
/// use zero_ui::properties::events::on_pre_event;
/// use zero_ui::core::{UiNode, keyboard::{KeyDownEvent, KeyInputArgs}, property};
/// use zero_ui::core::context::WidgetContext;
///
/// /// Sets an event listener for the [`KeyDownEvent`].
/// #[property(event)]
/// pub fn on_pre_key_down(
///    child: impl UiNode,
///    handler: impl FnMut(&mut WidgetContext, &KeyInputArgs) + 'static
/// ) -> impl UiNode {
///     on_pre_event(child, KeyDownEvent, handler)
/// }
/// ```
#[inline]
pub fn on_pre_event<E: Event>(child: impl UiNode, event: E, handler: impl FnMut(&mut WidgetContext, &E::Args) + 'static) -> impl UiNode {
    on_pre_event_filtered(child, event, |ctx, args| args.concerns_widget(ctx), handler)
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
/// The event route is similar to [`on_pre_event`], parent widgets get first chance of handling the event.
/// In-fact if you use the filter `|ctx, args| args.concerns_widget(ctx)` it will behave exactly the same.
pub fn on_pre_event_filtered<E: Event>(
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

event_property! {
    /// Event fired when a keyboard key is pressed or released.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
    /// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press/release generates a key input event, including keys that don't map
    /// to any virtual key, see [`KeyInputArgs`] for more details. To take text input use [`on_char_input`] instead.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KeyInputEvent`] that is included in the default app.
    pub fn key_input {
        event: KeyInputEvent,
        args: KeyInputArgs,
    }

    /// Event fired when a keyboard key is pressed.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
    /// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key press generates a key down event, including keys that don't map to any virtual key, see [`KeyInputArgs`]
    /// for more details. To take text input use [`on_char_input`] instead.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KeyDownEvent`] that is included in the default app.
    pub fn key_down {
        event: KeyDownEvent,
        args: KeyInputArgs,
    }

    /// Event fired when a keyboard key is released.
    ///
    /// # Route
    ///
    /// The event is raised in the [keyboard focused](crate::properties::is_focused)
    /// widget and then each parent up to the root. If [`stop_propagation`](EventArgs::stop_propagation)
    /// is requested the event is not notified further. If the widget is [disabled](IsEnabled) the event is not notified.
    ///
    /// This route is also called *bubbling*.
    ///
    /// # Keys
    ///
    /// Any key release generates a key up event, including keys that don't map to any virtual key, see [`KeyInputArgs`]
    /// for more details. To take text input use [`on_char_input`] instead.
    /// For key combinations consider using [`on_shortcut`].
    ///
    /// # Underlying Event
    ///
    /// This event property uses the [`KeyUpEvent`] that is included in the default app.
    pub fn key_up {
        event: KeyUpEvent,
        args: KeyInputArgs,
    }

    pub fn char_input {
        event: CharInputEvent,
        args: CharInputArgs,
    }
}

event_property! {
    pub fn mouse_move {
        event: MouseMoveEvent,
        args: MouseMoveArgs,
    }

    pub fn mouse_input {
        event: MouseInputEvent,
        args: MouseInputArgs,
    }

    pub fn mouse_down {
        event: MouseDownEvent,
        args: MouseInputArgs,
    }

    pub fn mouse_up {
        event: MouseUpEvent,
        args: MouseInputArgs,
    }

    pub fn mouse_any_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_any_single_click {
        event: MouseSingleClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_any_double_click {
        event: MouseDoubleClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_any_triple_click {
        event: MouseTripleClickEvent,
        args: MouseClickArgs,
    }

    pub fn mouse_click {
        event: MouseClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_single_click {
        event: MouseSingleClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_double_click {
        event: MouseDoubleClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_triple_click {
        event: MouseTripleClickEvent,
        args: MouseClickArgs,
        filter: mouse_primary_filter,
    }

    pub fn mouse_enter {
        event: MouseEnterEvent,
        args: MouseHoverArgs,
    }

    pub fn mouse_leave {
        event: MouseLeaveEvent,
        args: MouseHoverArgs,
    }

    pub fn got_mouse_capture {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
        filter: |ctx, args| args.is_got(ctx.path.widget_id()),
    }

    pub fn lost_mouse_capture {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
        filter: |ctx, args| args.is_lost(ctx.path.widget_id()),
    }

    pub fn mouse_capture_changed {
        event: MouseCaptureEvent,
        args: MouseCaptureArgs,
    }
}
// filter used in mouse_click, mouse_single_click, mouse_double_click and mouse_triple_click.
fn mouse_primary_filter(ctx: &mut WidgetContext, args: &MouseClickArgs) -> bool {
    args.concerns_widget(ctx) && args.is_primary()
}

event_property! {
    /// Adds a handler for clicks in the widget from any mouse button.
    pub fn any_click {
        event: ClickEvent,
        args: ClickArgs,
    }

    /// Adds a handler for clicks in the widget from the left mouse button.
    pub fn click {
        event: ClickEvent,
        args: ClickArgs,
        filter: primary_filter,
    }

    pub fn any_single_click {
        event: SingleClickEvent,
        args: ClickArgs,
    }

    pub fn single_click {
        event: SingleClickEvent,
        args: ClickArgs,
        filter: primary_filter,
    }

    pub fn any_double_click {
        event: DoubleClickEvent,
        args: ClickArgs,
    }

    pub fn double_click {
        event: DoubleClickEvent,
        args: ClickArgs,
        filter: primary_filter,
    }

    pub fn any_triple_click {
        event: TripleClickEvent,
        args: ClickArgs,
    }

    pub fn triple_click {
        event: TripleClickEvent,
        args: ClickArgs,
        filter: primary_filter,
    }

    pub fn context_click {
        event: SingleClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_context(),
    }

    pub fn shortcut {
        event: ShortcutEvent,
        args: ShortcutArgs,
    }
}
// filter used in click, single_click, double_click and triple_click.
fn primary_filter(ctx: &mut WidgetContext, args: &ClickArgs) -> bool {
    args.concerns_widget(ctx) && args.is_primary()
}

event_property! {
    /// Focus changed in the widget or its descendants.
    pub fn focus_changed {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
    }

    /// Widget got direct keyboard focus.
    pub fn focus {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus(ctx.path.widget_id()),
    }

    /// Widget lost direct keyboard focus.
    pub fn blur {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_blur(ctx.path.widget_id()),
    }

    /// Widget or one of its descendants got focus.
    pub fn focus_enter {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus_enter(ctx.path.widget_id())
    }

    /// Widget or one of its descendants lost focus.
    pub fn focus_leave {
        event: FocusChangedEvent,
        args: FocusChangedArgs,
        filter: |ctx, args| args.is_focus_leave(ctx.path.widget_id())
    }
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

struct OnPreviewRenderNode<C: UiNode, F: Fn(&mut FrameBuilder)> {
    child: C,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameBuilder) + 'static> UiNode for OnPreviewRenderNode<C, F> {
    fn render(&self, frame: &mut FrameBuilder) {
        (self.handler)(frame);
        self.child.render(frame);
    }
}

/// The `handler` is called even when the widget is [disabled](IsEnabled).
#[property(event)]
pub fn on_pre_render(child: impl UiNode, handler: impl Fn(&mut FrameBuilder) + 'static) -> impl UiNode {
    OnPreviewRenderNode { child, handler }
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
