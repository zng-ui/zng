use crate::core2::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

struct OnEvent<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>)> {
    child: C,
    event: E,
    listener: EventListener<E::Args>,
    handler: F,
}

#[impl_ui_node_crate(child)]
impl<C: UiNode, E: Event, F: FnMut(&mut OnEventArgs<E::Args>) + 'static> UiNode for OnEvent<C, E, F> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.listener = ctx.listen::<E>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if ctx.try_get_visited::<StopPropagation<E>>().is_none() {
            for args in self.listener.updates(&ctx) {
                let mut args = OnEventArgs::new(args);
                (self.handler)(&mut args);
                if args.handled() {
                    ctx.set_visited::<StopPropagation<E>>(());
                    break;
                }
            }
        }
        self.child.update(ctx);
    }
}

#[property(event)]
pub fn on_key_down(child: impl UiNode, handler: impl FnMut(&mut OnEventArgs<KeyInputArgs>) + 'static ) -> impl UiNode {
    on_event::set(child, KeyDown, handler)
}

#[property(event)]
pub fn on_event<E: Event>(child: impl UiNode, event: E, handler: impl FnMut(&mut OnEventArgs<E::Args>) + 'static) -> impl UiNode {
    OnEvent {
        child,
        event,
        listener: EventListener::never(false),
        handler,
    }
}

pub struct StopPropagation<E: Event> {
    _e: std::marker::PhantomData<E>,
}

impl<E: Event> VisitedVar for StopPropagation<E> {
    type Type = ();
}

pub struct OnEventArgs<'a, A: EventArgs> {
    args: &'a A,
    stop_propagation: bool,
}

impl<'a, A: EventArgs> OnEventArgs<'a, A> {
    pub fn new(args: &'a A) -> Self {
        OnEventArgs {
            args,
            stop_propagation: false,
        }
    }

    pub fn args(&self) -> &'a A {
        self.args
    }

    pub fn stop_propagation(&mut self) {
        self.stop_propagation = true;
    }

    pub fn handled(self) -> bool {
        self.stop_propagation
    }
}
