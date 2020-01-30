use crate::core::context::*;
use crate::core::event::*;
use crate::core::events::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

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
        if E::valid_in_widget(ctx) && ctx.event_state.flagged(StopPropagation::<E>::default()) {
            for args in self.listener.updates(&ctx.events) {
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

#[property(event)]
pub fn on_event<E: Event>(
    child: impl UiNode,
    event: E,
    handler: impl FnMut(&mut OnEventArgs<E::Args>) + 'static,
) -> impl UiNode {
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

    /// Finished call to handler, returns if should [stop_propagation].
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
    on_event::set(child, KeyInput, handler)
}

#[property(event)]
pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event::set(child, KeyDown, handler)
}

#[property(event)]
pub fn on_key_up(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event::set(child, KeyUp, handler)
}

#[property(event)]
pub fn on_mouse_move(
    child: impl UiNode,
    handler: impl FnMut(&mut OnEventArgs<MouseMoveArgs>) + 'static,
) -> impl UiNode {
    on_event::set(child, MouseMove, handler)
}

#[property(event)]
pub fn on_mouse_input(
    child: impl UiNode,
    handler: impl FnMut(&mut OnEventArgs<MouseInputArgs>) + 'static,
) -> impl UiNode {
    on_event::set(child, MouseInput, handler)
}

#[property(event)]
pub fn on_mouse_down(
    child: impl UiNode,
    handler: impl FnMut(&mut OnEventArgs<MouseInputArgs>) + 'static,
) -> impl UiNode {
    on_event::set(child, MouseDown, handler)
}

#[property(event)]
pub fn on_mouse_up(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<MouseInputArgs>) + 'static) -> impl UiNode {
    on_event::set(child, MouseUp, handler)
}

#[property(event)]
pub fn on_mouse_click(
    child: impl UiNode,
    handler: impl FnMut(&mut OnEventArgs<MouseClickArgs>) + 'static,
) -> impl UiNode {
    on_event::set(child, MouseClick, handler)
}
