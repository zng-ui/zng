use crate::core::context::*;
use crate::core::event::*;
use crate::core::focus::{FocusChangedArgs, FocusChangedEvent};
use crate::core::mouse::*;
use crate::core::var::{ObjVar, StateVar};
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
        self.mouse_enter = ctx.events.listen::<MouseEnterEvent>();
        self.mouse_leave = ctx.events.listen::<MouseLeaveEvent>();
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

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.push_set(false, ctx.vars, ctx.updates).expect("is_hovered");
        }
        self.child.deinit(ctx);
    }
}

/// If the mouse pointer is over the widget.
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
        self.mouse_down = ctx.events.listen::<MouseDownEvent>();
        self.mouse_up = ctx.events.listen::<MouseUpEvent>();
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

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.push_set(false, ctx.vars, ctx.updates).expect("is_pressed");
        }
        self.child.deinit(ctx);
    }
}

/// If the mouse pointer is pressed in the widget.
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
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
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

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.push_set(false, ctx.vars, ctx.updates).expect("is_focused");
        }
        self.child.deinit(ctx);
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// You can use [`is_focus_within`](is_focus_within) to check if the focused widget is within this one.
///
/// # Highlighting
///
/// TODO
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
    focus_changed: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithin<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
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

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.push_set(false, ctx.vars, ctx.updates).expect("is_focus_within");
        }
        self.child.deinit(ctx);
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

struct IsFocusedHighlighted<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusedHighlighted<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused_hgl = *self.state.get(ctx.vars);
            let is_focused_hgl = u.highlight && u.new_focus.as_ref().map(|p| p.widget_id() == ctx.widget_id).unwrap_or_default();
            if was_focused_hgl != is_focused_hgl {
                self.state.push_set(is_focused_hgl, ctx.vars, ctx.updates).expect("is_focused_hgl");
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.push_set(false, ctx.vars, ctx.updates).expect("is_focused_hgl");
        }
        self.child.deinit(ctx);
    }
}

/// If the widget has keyboard focus and focus highlighting is enabled.
///
/// This is only `true` if the widget itself is focused and focus highlighting is enabled.
/// You can use [`is_focus_within_hgl`](is_focus_within_hgl) to check if the focused widget is within this one.
///
/// Also see [`is_focused`](is_focused) to check if the widget is focused regardless of highlighting.
#[property(context)]
pub fn is_focused_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusedHighlighted {
        child,
        state,
        focus_changed: EventListener::never(false),
    }
}

struct IsFocusWithinHighlighted<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithinHighlighted<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused_hgl = *self.state.get(ctx.vars);
            let is_focused_hgl = u.highlight && u.new_focus.as_ref().map(|p| p.contains(ctx.widget_id)).unwrap_or_default();

            if was_focused_hgl != is_focused_hgl {
                self.state
                    .push_set(is_focused_hgl, ctx.vars, ctx.updates)
                    .expect("is_focus_within_hgl");
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.push_set(false, ctx.vars, ctx.updates).expect("is_focus_within_hgl");
        }
        self.child.deinit(ctx);
    }
}

/// If the widget or one of its descendants has keyboard focus and focus highlighting is enabled.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`](is_focused_hgl).
///
/// Also see [`is_focus_within`](is_focus_within) to check if the widget has focus within regardless of highlighting.
#[property(context)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusWithinHighlighted {
        child,
        state,
        focus_changed: EventListener::never(false),
    }
}
