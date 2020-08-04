use crate::core::context::*;
use crate::core::event::*;
use crate::core::gesture::*;
use crate::core::keyboard::*;
use crate::core::mouse::*;
use crate::core::profiler::profile_scope;
use crate::core::render::FrameBuilder;
use crate::core::types::{LayoutSize, PixelGrid};
use crate::core::UiNode;
use crate::core::{impl_ui_node, property};

struct OnEvent<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>)> {
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    handler: F,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>) + 'static> OnEvent<C, E, F> {
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
/// use zero_ui::core::{UiNode, keyboard::{KeyDown, KeyInputArgs}, property};
///
/// /// Sets an event listener for the [`KeyDown`](KeyDown) event.
/// #[property(event)]
/// pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
///     on_event(child, KeyDown, handler)
/// }
/// ```
#[inline]
pub fn on_event<E: Event>(child: impl UiNode, event: E, handler: impl FnMut(&mut OnEventArgs<E::Args>) + 'static) -> impl UiNode {
    OnEvent {
        child,
        _event: event,
        listener: EventListener::never(false),
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

pub struct StopPropagation<E: Event> {
    _e: std::marker::PhantomData<E>,
}

impl<E: Event> Default for StopPropagation<E> {
    fn default() -> Self {
        StopPropagation {
            _e: std::marker::PhantomData,
        }
    }
}

impl<E: Event> StateKey for StopPropagation<E> {
    type Type = ();
}

#[property(event)]
pub fn on_key_input(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event(child, KeyInputEvent, handler)
}

/// Sets an event listener for the [`KeyDown`](KeyDown) event.
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
    struct OnInit { fn init } fn on_init;
    struct OnDeinit { fn deinit } fn on_denit;
    struct OnUpdate { fn update } fn on_update;
    struct OnUpdateHp { fn update_hp } fn on_update_hp;
}

struct OnRender<C: UiNode, F: Fn(&mut FrameBuilder)> {
    child: C,
    handler: F,
}

#[impl_ui_node(child)]
impl<C: UiNode, F: Fn(&mut FrameBuilder) + 'static> UiNode for OnRender<C, F> {
    fn render(&self, frame: &mut FrameBuilder) {
        self.child.render(frame);
        (self.handler)(frame);
    }
}

#[property(event)]
pub fn on_render(child: impl UiNode, handler: impl Fn(&mut FrameBuilder) + 'static) -> impl UiNode {
    OnRender { child, handler }
}

#[derive(Debug)]
pub struct OnArrangeArgs {
    pub final_size: LayoutSize,
    pub pixel_grid: PixelGrid,
}

struct OnArrange<C: UiNode, F: FnMut(OnArrangeArgs)> {
    child: C,
    handler: F,
}

#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(OnArrangeArgs) + 'static> UiNode for OnArrange<C, F> {
    fn arrange(&mut self, final_size: LayoutSize, pixel_grid: PixelGrid) {
        self.child.arrange(final_size, pixel_grid);
        (self.handler)(OnArrangeArgs { final_size, pixel_grid });
    }
}

#[property(event)]
pub fn on_arrange(child: impl UiNode, handler: impl FnMut(OnArrangeArgs) + 'static) -> impl UiNode {
    OnArrange { child, handler }
}

#[derive(Debug)]
pub struct OnMeasureArgs {
    pub available_size: LayoutSize,
    pub desired_size: LayoutSize,
    pub pixel_grid: PixelGrid,
}

struct OnMeasure<C: UiNode, F: FnMut(OnMeasureArgs) -> LayoutSize> {
    child: C,
    handler: F,
}

#[impl_ui_node(child)]
impl<C: UiNode, F: FnMut(OnMeasureArgs) -> LayoutSize + 'static> UiNode for OnMeasure<C, F> {
    fn measure(&mut self, available_size: LayoutSize, pixel_grid: PixelGrid) -> LayoutSize {
        let mut args = OnMeasureArgs {
            available_size,
            desired_size: LayoutSize::zero(),
            pixel_grid,
        };

        args.desired_size = self.child.measure(available_size, pixel_grid);

        (self.handler)(args)
    }
}

#[property(event)]
pub fn on_measure(child: impl UiNode, handler: impl FnMut(OnMeasureArgs) -> LayoutSize + 'static) -> impl UiNode {
    OnMeasure { child, handler }
}
