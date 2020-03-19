use crate::core::context::*;
use crate::core::event::*;
use crate::core::mouse::*;
use crate::core::var::Var;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct IsState<C: UiNode, E: Event, L: Event, S: Var<bool>> {
    child: C,
    _enter: E,
    _leave: L,
    state: S,
    enter_listener: EventListener<E::Args>,
    leave_listener: EventListener<L::Args>,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: Event, L: Event, S: Var<bool>> IsState<C, E, L, S> {
    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.enter_listener = ctx.events.listen::<E>();
        self.leave_listener = ctx.events.listen::<L>();
        self.child.init(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if !E::IS_HIGH_PRESSURE {
            self.do_enter(ctx)
        }
        if !L::IS_HIGH_PRESSURE {
            self.do_leave(ctx)
        }
    }

    #[UiNode]
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.child.update_hp(ctx);

        if E::IS_HIGH_PRESSURE {
            self.do_enter(ctx)
        }
        if L::IS_HIGH_PRESSURE {
            self.do_leave(ctx)
        }
    }

    fn do_enter(&mut self, ctx: &mut WidgetContext) {
        if self.enter_listener.has_updates(ctx.events) {
            let _ = ctx.updates.push_set(&self.state, true);
        }
    }

    fn do_leave(&mut self, ctx: &mut WidgetContext) {
        if self.leave_listener.has_updates(ctx.events) {
            let _ = ctx.updates.push_set(&self.state, false);
        }
    }
}

/// Helper for declaring properties that set a state variable
#[inline]
pub fn is_state(child: impl UiNode, enter: impl Event, leave: impl Event, state: impl Var<bool>) -> impl UiNode {
    IsState {
        child,
        _enter: enter,
        _leave: leave,
        state,
        enter_listener: EventListener::never(false),
        leave_listener: EventListener::never(false),
    }
}

#[property(context)]
pub fn is_hovered(child: impl UiNode, state: impl Var<bool>) -> impl UiNode{
    is_state(child, MouseEnter, MouseLeave, state)
}
