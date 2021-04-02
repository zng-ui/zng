//! Widget state properties, [`is_hovered`](fn@is_hovered), [`is_pressed`](fn@is_pressed), [`is_focused`](fn@is_focused) and more.

use std::time::Duration;

use crate::core::mouse::*;
use crate::core::window::{WindowDeactivatedEvent, WindowIsActiveArgs};
use crate::core::{focus::*, sync::TimeElapsed};
use crate::prelude::new_property::*;

/// If the mouse pointer is over the widget.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
///
/// This TODO cap, use [`is_cap_hovered`](fn@is_cap_hovered) for that.
#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
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

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.mouse_enter = MouseEnterEvent::never();
            self.mouse_leave = MouseEnterEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            let mut state = *self.state.get(ctx.vars);

            if IsEnabled::get(ctx.vars) {
                if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    state = false;
                }
                if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    state = true;
                }
            } else {
                state = false;
            }

            if state != *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, state);
            }
        }
    }
    IsHoveredNode {
        child,
        state,
        mouse_enter: MouseEnterEvent::never(),
        mouse_leave: MouseLeaveEvent::never(),
    }
}

/// If the mouse pointer is over the widget or captured by the widget.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
#[property(context)]
pub fn is_cap_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsCapHoveredNode<C: UiNode> {
        child: C,
        state: StateVar,
        is_hovered: bool,
        is_captured: bool,
        mouse_enter: EventListener<MouseHoverArgs>,
        mouse_leave: EventListener<MouseHoverArgs>,
        mouse_capture: EventListener<MouseCaptureArgs>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsCapHoveredNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.mouse_enter = ctx.events.listen::<MouseEnterEvent>();
            self.mouse_leave = ctx.events.listen::<MouseLeaveEvent>();
            self.mouse_capture = ctx.events.listen::<MouseCaptureEvent>();
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.is_hovered = false;
            self.is_captured = false;
            self.mouse_enter = MouseEnterEvent::never();
            self.mouse_leave = MouseEnterEvent::never();
            self.mouse_capture = MouseCaptureEvent::never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if IsEnabled::get(ctx.vars) {
                if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    self.is_hovered = false;
                }
                if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    self.is_hovered = true;
                }
                for a in self.mouse_capture.updates(ctx.events) {
                    if a.is_lost(ctx.path.widget_id()) {
                        self.is_captured = false;
                    } else if a.is_got(ctx.path.widget_id()) {
                        self.is_captured = true;
                    }
                }
            } else {
                self.is_hovered = false;
                self.is_captured = false;
            }

            let state = self.is_hovered || self.is_captured;
            if state != *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, state);
            }
        }
    }
    IsCapHoveredNode {
        child,
        state,
        is_hovered: false,
        is_captured: false,
        mouse_enter: MouseEnterEvent::never(),
        mouse_leave: MouseLeaveEvent::never(),
        mouse_capture: MouseCaptureEvent::never(),
    }
}

/// If the pointer is pressed in the widget.
///
/// This is `true` when the mouse primary button started pressing in the widget
/// and the pointer is over the widget and the primary button is still pressed.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
///
/// This TODO cap.
#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsPressedNode<C: UiNode> {
        child: C,
        state: StateVar,
        is_down: bool,
        is_over: bool,
        is_shortcut_press: bool,

        mouse_input: EventListener<MouseInputArgs>,
        click: EventListener<ClickArgs>,
        mouse_leave: EventListener<MouseHoverArgs>,
        mouse_enter: EventListener<MouseHoverArgs>,
        window_deactivated: EventListener<WindowIsActiveArgs>,
        shortcut_release: EventListener<TimeElapsed>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsPressedNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.mouse_input = ctx.events.listen::<MouseInputEvent>();
            self.click = ctx.events.listen::<ClickEvent>();
            self.mouse_enter = ctx.events.listen::<MouseEnterEvent>();
            self.mouse_leave = ctx.events.listen::<MouseLeaveEvent>();
            self.window_deactivated = ctx.events.listen::<WindowDeactivatedEvent>();
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.is_down = false;
            self.is_over = false;
            self.is_shortcut_press = false;
            self.mouse_input = MouseInputEvent::never();
            self.click = ClickEvent::never();
            self.mouse_enter = MouseEnterEvent::never();
            self.mouse_leave = MouseLeaveEvent::never();
            self.window_deactivated = WindowDeactivatedEvent::never();
            self.shortcut_release = EventListener::response_never();
            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if IsEnabled::get(ctx.vars) {
                for args in self.mouse_input.updates(ctx.events) {
                    if args.is_primary() {
                        match args.state {
                            ElementState::Pressed => {
                                if args.concerns_capture(ctx) {
                                    self.is_down = true;
                                }
                            }
                            ElementState::Released => {
                                self.is_down = false;
                            }
                        }
                    }
                }
                if self.mouse_leave.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    self.is_over = false;
                }
                if self.mouse_enter.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    self.is_over = true;
                }
                if self.window_deactivated.updates(ctx.events).iter().any(|a| a.concerns_widget(ctx)) {
                    self.is_down = false;
                }

                if self
                    .click
                    .updates(ctx.events)
                    .iter()
                    .any(|a| a.concerns_widget(ctx) && a.shortcut().is_some())
                {
                    if self.is_shortcut_press {
                        self.is_shortcut_press = false;
                        self.shortcut_release = EventListener::response_never();
                    } else {
                        let duration = ctx.services.req::<Gestures>().shortcut_pressed_duration;
                        if duration != Duration::default() {
                            self.is_shortcut_press = true;
                            self.shortcut_release = ctx.sync.update_after(duration);
                        }
                    }
                }

                if self.shortcut_release.has_updates(ctx.events) {
                    self.is_shortcut_press = false;
                }
            } else {
                self.is_down = false;
                self.is_over = false;
            }

            let state = (self.is_down && self.is_over) || self.is_shortcut_press;
            if state != *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, state);
            }
        }
    }
    IsPressedNode {
        child,
        state,
        is_down: false,
        is_over: false,
        is_shortcut_press: false,
        mouse_input: MouseInputEvent::never(),
        click: ClickEvent::never(),
        mouse_enter: MouseEnterEvent::never(),
        mouse_leave: MouseLeaveEvent::never(),
        window_deactivated: WindowDeactivatedEvent::never(),
        shortcut_release: EventListener::response_never(),
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
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
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
    }
    IsFocusedNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget or one of its descendants has keyboard focus.
///
/// To check if only the widget has keyboard focus use [`is_focused`].
#[property(context)]
pub fn is_focus_within(child: impl UiNode, state: StateVar) -> impl UiNode {
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
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
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
    }
    IsFocusWithinNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
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
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
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
    }
    IsFocusedHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget or one of its descendants has keyboard focus and focus highlighting is enabled.
///
/// To check if only the widget has keyboard focus use [`is_focused_hgl`].
///
/// Also see [`is_focus_within`] to check if the widget has focus within regardless of highlighting.
#[property(context)]
pub fn is_focus_within_hgl(child: impl UiNode, state: StateVar) -> impl UiNode {
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
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            if *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, false);
            }
            self.focus_changed = FocusChangedEvent::never();
            self.child.deinit(ctx);
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
    }
    IsFocusWithinHglNode {
        child,
        state,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// If the widget is focused when a parent scope is focused.
#[property(context)]
pub fn is_return_focus(child: impl UiNode, state: StateVar) -> impl UiNode {
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
            self.return_focus_changed = ReturnFocusChangedEvent::never();
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
    IsReturnFocusNode {
        child,
        state,
        return_focus_changed: ReturnFocusChangedEvent::never(),
    }
}

/// If the widget is hit-test visible.
///
/// This property is used only for probing the state. You can set the state using the
/// [`hit_testable`](crate::properties::hit_testable) property.
#[property(context)]
pub fn is_hit_testable(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsHitTestableNode<C: UiNode> {
        child: C,
        state: StateVar,
        //expected: bool,
    }
    impl<C: UiNode> IsHitTestableNode<C> {
        fn update_state(&self, ctx: &mut WidgetContext) {
            let hit_testable = IsHitTestable::get(ctx.vars) && ctx.widget_state.hit_testable();
            let is_state = hit_testable; // == self.expected;
            if is_state != *self.state.get(ctx.vars) {
                self.state.set(ctx.vars, is_state);
            }
        }
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsHitTestableNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.update_state(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);
            self.update_state(ctx);
        }
    }
    IsHitTestableNode {
        child,
        state,
        //expected: true
    }
}

struct IsEnabledNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: bool,
}
impl<C: UiNode> IsEnabledNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let enabled = IsEnabled::get(ctx.vars) && ctx.widget_state.enabled();
        let is_state = enabled == self.expected;
        if is_state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, is_state);
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsEnabledNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}
/// If the widget is enabled for receiving events.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`](crate::properties::enabled) property.
#[property(context)]
pub fn is_enabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: true,
    }
}
/// If the widget is disabled for receiving events.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`](crate::properties::enabled) property.
///
/// This is the same as `!self.is_enabled`.
#[property(context)]
pub fn is_disabled(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsEnabledNode {
        child,
        state,
        expected: false,
    }
}

use crate::core::widget_base::{Visibility, VisibilityContext, WidgetVisibilityExt};

use super::{IsHitTestable, WidgetHitTestableExt};

struct IsVisibilityNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: Visibility,
}
impl<C: UiNode> IsVisibilityNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let vis = VisibilityContext::get(ctx.vars) | ctx.widget_state.visibility();
        let is_state = vis == self.expected;
        if is_state != *self.state.get(ctx.vars) {
            self.state.set(ctx.vars, is_state);
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for IsVisibilityNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.update_state(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.update_state(ctx);
    }
}
/// If the widget [`visibility`](fn@crate::core::widget_base::visibility) is [`Visible`](crate::core::widget_base::Visibility::Visible).
#[property(context)]
pub fn is_visible(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Visible,
    }
}
/// If the widget [`visibility`](fn@crate::core::widget_base::visibility) is [`Hidden`](crate::core::widget_base::Visibility::Hidden).
#[property(context)]
pub fn is_hidden(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Hidden,
    }
}
/// If the widget [`visibility`](fn@crate::core::widget_base::visibility) is [`Collapsed`](crate::core::widget_base::Visibility::Collapsed).
#[property(context)]
pub fn is_collapsed(child: impl UiNode, state: StateVar) -> impl UiNode {
    IsVisibilityNode {
        child,
        state,
        expected: Visibility::Collapsed,
    }
}
