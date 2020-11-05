//! Widget state properties, [`is_hovered`], [`is_pressed`], [`is_focused`] and more.

use crate::core::focus::*;
use crate::core::mouse::*;
use crate::prelude::new_property::*;

struct IsHoveredNode<C: UiNode> {
    child: C,
    state: StateVar,
    mouse_enter: EventListener<MouseHoverArgs>,
    mouse_leave: EventListener<MouseHoverArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsHoveredNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.mouse_enter = ctx.events.listen::<MouseEnterEvent>();
        self.mouse_leave = ctx.events.listen::<MouseLeaveEvent>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let mut new_state = *self.state.get(ctx.vars);
        if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            new_state = false;
        }
        if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            new_state = true;
        }

        if new_state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, new_state);
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the mouse pointer is over the widget.
#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsHoveredNode {
        child,
        state,
        mouse_enter: MouseEnterEvent::never(),
        mouse_leave: MouseLeaveEvent::never(),
    }
}

struct IsPressedNode<C: UiNode> {
    child: C,
    state: StateVar,
    mouse_down: EventListener<MouseInputArgs>,
    mouse_up: EventListener<MouseInputArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsPressedNode<C> {
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
                self.state.set(ctx.vars, false);
            }
        } else if self.mouse_down.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
            // if not pressed and mouse down inside.
            self.state.set(ctx.vars, true);
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the mouse pointer is pressed in the widget.
#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsPressedNode {
        child,
        state,
        mouse_down: MouseDownEvent::never(),
        mouse_up: MouseUpEvent::never(),
    }
}

struct IsFocusedNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusedNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused = *self.state.get(ctx.vars);
            let is_focused = u
                .new_focus
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default();
            if was_focused != is_focused {
                self.state.set(ctx.vars, is_focused);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget has keyboard focus.
///
/// This is only `true` if the widget itself is focused.
/// You can use [`is_focus_within`] to check if the focused widget is within this one.
///
/// # Highlighting
///
/// TODO
#[property(context)]
pub fn is_focused(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusedNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsFocusWithinNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithinNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused = *self.state.get(ctx.vars);
            let is_focused = u.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or_default();

            if was_focused != is_focused {
                self.state.set(ctx.vars, is_focused);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`].
#[property(context)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusWithinNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsFocusedHglNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusedHglNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused_hgl = *self.state.get(ctx.vars);
            let is_focused_hgl = u.highlight
                && u.new_focus
                    .as_ref()
                    .map(|p| p.widget_id() == ctx.path.widget_id())
                    .unwrap_or_default();
            if was_focused_hgl != is_focused_hgl {
                self.state.set(ctx.vars, is_focused_hgl);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget has keyboard focus and focus highlighting is enabled.
///
/// This is only `true` if the widget itself is focused and focus highlighting is enabled.
/// You can use [`is_focus_within_hgl`] to check if the focused widget is within this one.
///
/// Also see [`is_focused`] to check if the widget is focused regardless of highlighting.
#[property(context)]
pub fn is_focused_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusedHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsFocusWithinHglNode<C: UiNode> {
    child: C,
    state: StateVar,
    focus_changed: EventListener<FocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsFocusWithinHglNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(u) = self.focus_changed.updates(ctx.events).last() {
            let was_focused_hgl = *self.state.get(ctx.vars);
            let is_focused_hgl = u.highlight && u.new_focus.as_ref().map(|p| p.contains(ctx.path.widget_id())).unwrap_or_default();

            if was_focused_hgl != is_focused_hgl {
                self.state.set(ctx.vars, is_focused_hgl);
            }
        }
        self.child.update(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }
}

/// If the widget or one of its descendants has keyboard focus and focus highlighting is enabled.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`].
///
/// Also see [`is_focus_within`] to check if the widget has focus within regardless of highlighting.
#[property(context)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsFocusWithinHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

struct IsReturnFocusNode<C: UiNode> {
    child: C,
    state: StateVar,
    return_focus_changed: EventListener<ReturnFocusChangedArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsReturnFocusNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.return_focus_changed = ctx.events.listen::<ReturnFocusChangedEvent>();
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, false);
        }
        self.child.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let state = *self.state.get(ctx.vars);
        let mut new_state = state;
        for args in self.return_focus_changed.updates(ctx.events) {
            if args
                .prev_return
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default()
            {
                new_state = false;
            }
            if args
                .new_return
                .as_ref()
                .map(|p| p.widget_id() == ctx.path.widget_id())
                .unwrap_or_default()
            {
                new_state = true;
            }
        }

        if new_state != state {
            self.state.set(ctx.vars, new_state);
        }
    }
}

/// If the widget is focused when a parent scope is focused.
#[property(context)]
pub fn is_return_focus(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsReturnFocusNode {
        child,
        state,
        return_focus_changed: ReturnFocusChangedEvent::never(),
    }
}
