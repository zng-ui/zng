use crate::core2::*;
use crate::{impl_ui_node, property};

struct OnEvent<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>)> {
    child: C,
    _event: E,
    listener: EventListener<E::Args>,
    handler: F,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>) + 'static> UiNode for OnEvent<C, E, F> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.listener = ctx.events.listen::<E>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if ctx.event_state.flagged(StopPropagation::<E>::default()) {
            for args in self.listener.updates(&ctx.events) {
                let mut args = OnEventArgs::new(ctx, args);
                (self.handler)(&mut args);
                if args.handled() {
                    ctx.event_state.flag(StopPropagation::<E>::default());
                    break;
                }
            }
        }
        self.child.update(ctx);
    }
}

#[property(event)]
pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static) -> impl UiNode {
    on_event::set(child, KeyDown, handler)
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

impl<E: Event> context::StateKey for StopPropagation<E> {
    type Type = ();
}

/// Event arguments.
pub struct OnEventArgs<'c, 'a, 'v, 'sa, 'sw, 'sx, 'e, 's, 'u, A: EventArgs> {
    ctx: &'c mut WidgetContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u>,
    args: &'a A,
    stop_propagation: bool,
}

impl<'c, 'a, 'v, 'sa, 'sw, 'sx, 'e, 's, 'u, A: EventArgs> OnEventArgs<'c, 'a, 'v, 'sa, 'sw, 'sx, 'e, 's, 'u, A> {
    pub fn new(ctx: &'c mut WidgetContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u>, args: &'a A) -> Self {
        OnEventArgs {
            ctx,
            args,
            stop_propagation: false,
        }
    }

    /// Widget context.
    pub fn ctx(&mut self) -> &mut WidgetContext<'v, 'sa, 'sw, 'sx, 'e, 's, 'u> {
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
