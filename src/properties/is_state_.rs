use crate::core::context::*;
use crate::core::event::*;
use crate::core::focus::{FocusChanged, FocusChangedArgs};
use crate::core::mouse::*;
use crate::core::var::{ObjVar, StateVar, VarIsReadOnly};
use crate::core::UiNode;
use crate::core::{impl_ui_node, property};

struct IsHovered<C: UiNode> {
    child: C,
    state: StateVar,
    mouse_enter: EventListener<MouseHoverArgs>,
    mouse_leave: EventListener<MouseHoverArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsHovered<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.mouse_enter = ctx.events.listen::<MouseEnter>();
        self.mouse_leave = ctx.events.listen::<MouseLeave>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if *self.state.get(ctx.vars) {
            if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                ctx.updates.push_set(&self.state, false, ctx.vars).expect("is_hovered");
            }
        } else if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            ctx.updates.push_set(&self.state, true, ctx.vars).expect("is_hovered");
        }
    }
}

#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsHovered {
        child,
        state,
        mouse_enter: EventListener::never(false),
        mouse_leave: EventListener::never(false),
    }
}

struct IsPressed<C: UiNode> {
    child: C,
    state: StateVar,
    mouse_down: EventListener<MouseInputArgs>,
    mouse_up: EventListener<MouseInputArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsPressed<C> {
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
                ctx.updates.push_set(&self.state, false, ctx.vars).expect("is_pressed");
            }
        } else if self.mouse_down.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            // if not pressed and mouse down inside.
            ctx.updates.push_set(&self.state, true, ctx.vars).expect("is_pressed");
        }
    }
}

#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsPressed {
        child,
        state,
        mouse_down: EventListener::never(false),
        mouse_up: EventListener::never(false),
    }
}

struct IsFocused<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocused<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChanged>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused = *self.state.get(ctx.vars);
            let is_focused = u.new_focus.as_ref().map(|p| p.widget_id() == ctx.widget_id).unwrap_or_default();

            if was_focused != is_focused {
                self.state.push_set(is_focused, ctx.vars, ctx.updates).expect("is_focused");
            }
        }
        self.child.update(ctx);
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// You can use [`is_focus_within`] to check if the focused widget is within this one.
#[property(context)]
pub fn is_focused(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocused {
        child,
        state,
        focus_changed: EventListener::never(false),
    }
}

struct IsFocusWithin<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithin<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChanged>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused = *self.state.get(ctx.vars);
            let is_focused = u.new_focus.as_ref().map(|p| p.contains(ctx.widget_id)).unwrap_or_default();

            if was_focused != is_focused {
                self.state.push_set(is_focused, ctx.vars, ctx.updates).expect("is_focus_within");
            }
        }
        self.child.update(ctx);
    }
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`](is_focused).
#[property(context)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusWithin {
        child,
        state,
        focus_changed: EventListener::never(false),
    }
}
