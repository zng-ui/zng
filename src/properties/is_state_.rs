use crate::core::context::*;
use crate::core::event::*;
use crate::core::mouse::*;
use crate::core::var::Var;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct IsHovered<C: UiNode, S: Var<bool>> {
    child: C,
    state: S,
    mouse_enter: EventListener<MouseHoverArgs>,
    mouse_leave: EventListener<MouseHoverArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, S: Var<bool>> UiNode for IsHovered<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.mouse_enter = ctx.events.listen::<MouseEnter>();
        self.mouse_leave = ctx.events.listen::<MouseLeave>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if *self.state.get(ctx.vars) {
            if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                let _ = ctx.updates.push_set(&self.state, false);
            }
        } else if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            let _ = ctx.updates.push_set(&self.state, true);
        }
    }
}

#[property(context)]
pub fn is_hovered(child: impl UiNode, state: impl Var<bool>) -> impl UiNode {
    IsHovered {
        child,
        state,
        mouse_enter: EventListener::never(false),
        mouse_leave: EventListener::never(false),
    }
}

struct IsPressed<C: UiNode, S: Var<bool>> {
    child: C,
    state: S,
    mouse_down: EventListener<MouseInputArgs>,
    mouse_up: EventListener<MouseInputArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, S: Var<bool>> UiNode for IsPressed<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.mouse_down = ctx.events.listen::<MouseDown>();
        self.mouse_up = ctx.events.listen::<MouseUp>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if *self.state.get(ctx.vars) {
            if self.mouse_up.has_updates(ctx.events) {
                // if mouse_up in any place.
                let _ = ctx.updates.push_set(&self.state, false);
            }
        } else if self.mouse_down.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            // if not pressed and mouse down inside.
            let _ = ctx.updates.push_set(&self.state, true);
        }
    }
}

#[property(context)]
pub fn is_pressed(child: impl UiNode, state: impl Var<bool>) -> impl UiNode {
    IsPressed {
        child,
        state,
        mouse_down: EventListener::never(false),
        mouse_up: EventListener::never(false),
    }
}
