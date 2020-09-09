use crate::core::context::*;
use crate::core::event::*;
use crate::core::focus::*;
use crate::core::gesture::*;
use crate::core::keyboard::*;
use crate::core::mouse::*;
use crate::core::profiler::profile_scope;
use crate::core::render::FrameBuilder;
use crate::core::units::*;
use crate::core::UiNode;
use crate::core::{impl_ui_node, property};
use std::fmt;

struct OnEventNode<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>)> {
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    handler: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>) + 'static> OnEventNode<C, E, F> {
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
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
        if !ctx.event_state.flagged(StopPropagation::<E>::default()) {
            for args in self.listener.updates(&ctx.events) {
                if args.concerns_widget(ctx) {
                    profile_scope!("on_event::<{}>", std::any::type_name::<E>());

                    let mut args = OnEventArgs::new(ctx, args);
                    (self.handler)(&mut args);
                    if args.handled() {
                        ctx.event_state.flag(StopPropagation::<E>::default());
                        break;
                    }
                }
            }
        }
    }
}

/// Helper for declaring properties that set a listener to an event.
///
/// # Example
/// ```
/// # fn main() { }
/// use zero_ui::properties::{on_event, OnEventArgs};
/// use zero_ui::core::{UiNode, keyboard::{KeyDownEvent, KeyInputArgs}, property};
///
/// /// Sets an event listener for the [`KeyDown`] event.
/// #[property(event)]
/// pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
///     on_event(child, KeyDownEvent, handler)
/// }
/// ```
#[inline]
pub fn on_event<E: Event>(child: impl UiNode, event: E, handler: impl FnMut(&mut OnEventArgs<E::Args>) + 'static) -> impl UiNode {
    OnEventNode {
        child,
        _event: event,
        listener: E::never(),
        handler,
    }
}
/// Event arguments.
pub struct OnEventArgs<'c, 'a, A: EventArgs> {
    ctx: &'a mut WidgetContext<'c>,
    args: &'a A,
    stop_propagation: bool,
}

impl<'c, 'a, A: EventArgs> OnEventArgs<'c, 'a, A> {
    pub fn new(ctx: &'a mut WidgetContext<'c>, args: &'a A) -> Self {
        OnEventArgs {
            ctx,
            args,
            stop_propagation: false,
        }
    }

    /// Widget context.
    pub fn ctx(&mut self) -> &mut WidgetContext<'c> {
        &mut self.ctx
    }

    /// Event arguments.
    pub fn args(&self) -> &'a A {
        self.args
    }

    /// Stops this event from being raised in other widgets.
    pub fn stop_propagation(&mut self) {
        self.stop_propagation = true;
    }

    /// Finished call to handler, returns if should [`stop_propagation`](OnEventArgs::stop_propagation).
    pub fn handled(self) -> bool {
        self.stop_propagation
    }
}

/// Event state flag that indicate the event is "handled".
pub struct StopPropagation<E: Event> {
    _e: std::marker::PhantomData<E>,
}
impl<E: Event> StopPropagation<E> {
    pub fn key() -> Self {
        StopPropagation {
            _e: std::marker::PhantomData,
        }
    }
}
impl<E: Event> Default for StopPropagation<E> {
    fn default() -> Self {
        Self::key()
    }
}
impl<E: Event> StateKey for StopPropagation<E> {
    type Type = ();
}
impl<E: Event> fmt::Debug for StopPropagation<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StopPropagation<{}>", std::any::type_name::<E>())
    }
}
impl<E: Event> Clone for StopPropagation<E> {
    fn clone(&self) -> Self {
        Self::key()
    }
}
impl<E: Event> Copy for StopPropagation<E> {}

#[property(event)]
pub fn on_key_input(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event(child, KeyInputEvent, handler)
}

/// Sets an event listener for the [`KeyDown`] event.
#[property(event)]
pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event(child, KeyDownEvent, handler)
}

#[property(event)]
pub fn on_key_up(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event(child, KeyUpEvent, handler)
}

#[property(event)]
pub fn on_mouse_move(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseMoveArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseMoveEvent, handler)
}

#[property(event)]
pub fn on_mouse_input(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseInputArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseInputEvent, handler)
}

#[property(event)]
pub fn on_mouse_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseInputArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseDownEvent, handler)
}

#[property(event)]
pub fn on_mouse_up(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseInputArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseUpEvent, handler)
}

#[property(event)]
pub fn on_mouse_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseClickArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_single_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseClickArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseDoubleClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_double_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseClickArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseSingleClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_triple_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseClickArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseTripleClickEvent, handler)
}

#[property(event)]
pub fn on_mouse_enter(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseHoverArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseEnterEvent, handler)
}

#[property(event)]
pub fn on_mouse_leave(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseHoverArgs>) + 'static) -> impl UiNode {
    on_event(child, MouseLeaveEvent, handler)
}

/// Adds a handler for clicks in the widget.
#[property(event)]
pub fn on_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<ClickArgs>) + 'static) -> impl UiNode {
    on_event(child, ClickEvent, handler)
}

#[property(event)]
pub fn on_single_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<ClickArgs>) + 'static) -> impl UiNode {
    on_event(child, SingleClickEvent, handler)
}

#[property(event)]
pub fn on_double_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<ClickArgs>) + 'static) -> impl UiNode {
    on_event(child, DoubleClickEvent, handler)
}

#[property(event)]
pub fn on_triple_click(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<ClickArgs>) + 'static) -> impl UiNode {
    on_event(child, TripleClickEvent, handler)
}

#[property(event)]
pub fn on_shortcut(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<ShortcutArgs>) + 'static) -> impl UiNode {
    on_event(child, ShortcutEvent, handler)
}

/// Focus changed in the widget or its descendants.
#[property(event)]
pub fn on_focus_changed(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static) -> impl UiNode {
    on_event(child, FocusChangedEvent, handler)
}

/// Widget got direct keyboard focus.
#[property(event)]
pub fn on_focus(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static) -> impl UiNode {
    OnFocusNode {
        child,
        handler,
        listener: FocusChangedEvent::never(),
    }
}

/// Widget lost direct keyboard focus.
#[property(event)]
pub fn on_blur(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static) -> impl UiNode {
    OnBlurNode {
        child,
        handler,
        listener: FocusChangedEvent::never(),
    }
}

/// Widget or one of its descendants got focus.
#[property(event)]
pub fn on_focus_enter(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static) -> impl UiNode {
    OnFocusEnterNode {
        child,
        handler,
        listener: FocusChangedEvent::never(),
    }
}

/// Widget or one of its descendants lost focus.
#[property(event)]
pub fn on_focus_leave(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static) -> impl UiNode {
    OnFocusLeaveNode {
        child,
        handler,
        listener: FocusChangedEvent::never(),
    }
}

struct OnFocusNode<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> {
    child: C,
    handler: H,
    listener: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> UiNode for OnFocusNode<C, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        for args in self.listener.updates(ctx.events) {
            if args
                .new_focus
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default()
            {
                (self.handler)(&mut OnEventArgs::new(ctx, args));
            }
        }
    }
}

struct OnBlurNode<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> {
    child: C,
    handler: H,
    listener: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> UiNode for OnBlurNode<C, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        for args in self.listener.updates(ctx.events) {
            if args
                .prev_focus
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default()
            {
                (self.handler)(&mut OnEventArgs::new(ctx, args));
            }
        }
    }
}

struct OnFocusEnterNode<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> {
    child: C,
    handler: H,
    listener: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> UiNode for OnFocusEnterNode<C, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        for args in self.listener.updates(ctx.events) {
            if args
                .new_focus
                .as_ref()
                .map(|p| p.contains(ctx.path.widget_id()))
                .unwrap_or_default()
                && args.prev_focus.as_ref().map(|p| !p.contains(ctx.path.widget_id())).unwrap_or(true)
            {
                // if we are in `new_focus` and are not in `prev_focus`
                (self.handler)(&mut OnEventArgs::new(ctx, args));
            }
        }
    }
}

struct OnFocusLeaveNode<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> {
    child: C,
    handler: H,
    listener: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, H: FnMut(&mut OnEventArgs<FocusChangedArgs>) + 'static> UiNode for OnFocusLeaveNode<C, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        for args in self.listener.updates(ctx.events) {
            if args
                .prev_focus
                .as_ref()
                .map(|p| p.contains(ctx.path.widget_id()))
                .unwrap_or_default()
                && args.new_focus.as_ref().map(|p| !p.contains(ctx.path.widget_id())).unwrap_or(true)
            {
                // if we are in `prev_focus` and are not in `new_focus`
                (self.handler)(&mut OnEventArgs::new(ctx, args));
            }
        }
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

#[property(event)]
pub fn on_measure(child: impl UiNode, handler: impl FnMut(OnMeasureArgs) -> LayoutSize + 'static) -> impl UiNode {
    OnMeasureNode { child, handler }
}
