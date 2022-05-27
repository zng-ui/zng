//! Widget state properties, [`is_hovered`](fn@is_hovered), [`is_pressed`](fn@is_pressed) and more.

use std::time::Duration;

use crate::core::{
    mouse::*,
    timer::DeadlineVar,
    window::{WidgetInfoChangedEvent, WindowFocusChangedEvent},
};
use crate::prelude::new_property::*;

/// If the mouse pointer is over the widget or an widget descendant.
///
/// This state property does not consider mouse capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_hovered`] to include the captured state.
///
/// The value is always `false` when the widget does not allow interaction, such as when it is disabled.
///
/// [`is_cap_hovered`]: fn@is_cap_hovered
#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsHoveredNode<C: UiNode> {
        child: C,
        state: StateVar,
        is_hovered: bool,
        allow_interaction: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsHoveredNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(MouseHoveredEvent).event(WidgetInfoChangedEvent);
            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.allow_interaction = ctx
                .info_tree
                .find(ctx.path.widget_id())
                .map(|w| w.allow_interaction())
                .unwrap_or(false);

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_hovered = false;
            self.allow_interaction = false;
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut update = false;
            if let Some(args) = MouseHoveredEvent.update(args) {
                if args.is_mouse_enter(ctx.path) {
                    self.is_hovered = true;
                    update = true;
                } else if args.is_mouse_leave(ctx.path) {
                    self.is_hovered = false;
                    update = true;
                }
                self.child.event(ctx, args);
            } else if let Some(args) = WidgetInfoChangedEvent.update(args) {
                if args.concerns_widget(ctx) {
                    self.allow_interaction = ctx
                        .info_tree
                        .find(ctx.path.widget_id())
                        .map(|w| w.allow_interaction())
                        .unwrap_or(false);
                    update = true;
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if update {
                self.state.set_ne(ctx, self.is_hovered && self.allow_interaction);
            }
        }
    }
    IsHoveredNode {
        child,
        state,
        is_hovered: false,
        allow_interaction: false,
    }
}

/// If the mouse pointer is over the widget or an widget descendant or captured by the widget.
///
/// The value is always `false` when the widget does not allow interaction, such as when it is disabled.
#[property(context)]
pub fn is_cap_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsCapHoveredNode<C> {
        child: C,
        state: StateVar,
        is_hovered: bool,
        is_captured: bool,
        allow_interaction: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsCapHoveredNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(MouseHoveredEvent).event(MouseCaptureEvent).event(WidgetInfoChangedEvent);

            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.allow_interaction = ctx
                .info_tree
                .find(ctx.path.widget_id())
                .map(|w| w.allow_interaction())
                .unwrap_or(false);

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.state.set_ne(ctx.vars, false);
            self.is_hovered = false;
            self.is_captured = false;
            self.allow_interaction = false;
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut update = false;

            // self.is_hovered:
            if let Some(args) = MouseHoveredEvent.update(args) {
                if args.is_mouse_enter(ctx.path) {
                    self.is_hovered = true;
                    update = true;
                } else if args.is_mouse_leave(ctx.path) {
                    self.is_hovered = false;
                    update = true;
                }
                self.child.event(ctx, args);
            }
            // self.is_captured:
            else if let Some(args) = MouseCaptureEvent.update(args) {
                if args.concerns_widget(ctx) {
                    if args.is_got(ctx.path.widget_id()) {
                        self.is_captured = true;
                        update = true;
                    } else if args.is_lost(ctx.path.widget_id()) {
                        self.is_captured = false;
                        update = true;
                    }
                }
                self.child.event(ctx, args);
            }
            // self.allow_interaction
            else if let Some(args) = WidgetInfoChangedEvent.update(args) {
                if args.concerns_widget(ctx) {
                    self.allow_interaction = ctx
                        .info_tree
                        .find(ctx.path.widget_id())
                        .map(|w| w.allow_interaction())
                        .unwrap_or(false);
                    update = true;
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if update {
                self.state
                    .set_ne(ctx.vars, (self.is_hovered || self.is_captured) && self.allow_interaction);
            }
        }
    }
    IsCapHoveredNode {
        child,
        state,
        is_hovered: false,
        is_captured: false,
        allow_interaction: false,
    }
}

/// If the pointer is pressed in the widget.
///
/// This is `true` when the mouse primary button started pressing in the widget
/// and the pointer is over the widget and the primary button is still pressed and
/// the widget allows interaction (is enabled).
///
/// The value is always `false` when the widget is disabled, and
/// if the widget is pressed, disabled and re-enabled the state remains `false`.
///
/// A keyboard shortcut press causes this to be `true` for the [`shortcut_pressed_duration`].
///
/// This state property does not consider mouse capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_pressed`] to
/// include the captured state.
///
/// [`shortcut_pressed_duration`]: Gestures::shortcut_pressed_duration
/// [`is_cap_pressed`]: fn@is_cap_pressed
#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsPressedNode<C> {
        child: C,
        state: StateVar,
        is_down: bool,
        is_over: bool,
        allow_interaction: bool,
        shortcut_press: Option<DeadlineVar>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsPressedNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(MouseHoveredEvent)
                .event(MouseInputEvent)
                .event(ClickEvent)
                .event(WindowFocusChangedEvent)
                .event(WidgetInfoChangedEvent);

            if let Some(s) = &self.shortcut_press {
                subs.var(ctx, s);
            }

            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.allow_interaction = ctx
                .info_tree
                .find(ctx.path.widget_id())
                .map(|w| w.allow_interaction())
                .unwrap_or(false);

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_down = false;
            self.is_over = false;
            self.allow_interaction = false;
            self.shortcut_press = None;
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut update = false;

            // self.is_over:
            if let Some(args) = MouseHoveredEvent.update(args) {
                if args.is_mouse_enter(ctx.path) {
                    self.is_over = true;
                    update = true;
                } else if args.is_mouse_leave(ctx.path) {
                    self.is_over = false;
                    update = true;
                }
                self.child.event(ctx, args);
            }
            // self.is_down:
            else if let Some(args) = MouseInputEvent.update(args) {
                if args.is_primary() {
                    match args.state {
                        ButtonState::Pressed => {
                            if args.concerns_capture(ctx) {
                                self.is_down = true;
                                update = true;
                            }
                        }
                        ButtonState::Released => {
                            self.is_down = false;
                            update = true;
                        }
                    }
                }
                self.child.event(ctx, args);
            }
            // self.shortcut_press:
            else if let Some(args) = ClickEvent.update(args) {
                if args.shortcut().is_some() && args.concerns_widget(ctx) {
                    // if a shortcut click happened, we show pressed for the duration of `shortcut_pressed_duration`
                    // unless we where already doing that, then we just stop showing pressed, this causes
                    // a flickering effect when rapid clicks are happening.
                    if self.shortcut_press.take().is_none() {
                        let duration = ctx.services.gestures().shortcut_pressed_duration;
                        if duration != Duration::default() {
                            self.shortcut_press = Some(ctx.timers.timeout(duration));
                            update = true;
                            ctx.updates.subscriptions();
                        }
                    } else {
                        update = true;
                        ctx.updates.subscriptions();
                    }
                }
                self.child.event(ctx, args);
            }
            // self.is_down = false;
            else if let Some(args) = WindowFocusChangedEvent.update(args) {
                if !args.focused && args.concerns_widget(ctx) {
                    self.is_down = false;
                    update = true;
                }
                self.child.event(ctx, args);
            }
            // self.allow_interaction
            else if let Some(args) = WidgetInfoChangedEvent.update(args) {
                if args.concerns_widget(ctx) {
                    self.allow_interaction = ctx
                        .info_tree
                        .find(ctx.path.widget_id())
                        .map(|w| w.allow_interaction())
                        .unwrap_or(false);

                    if !self.allow_interaction {
                        self.is_down = false;
                        self.shortcut_press = None;
                    }

                    update = true;
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if update {
                self.state.set_ne(
                    ctx.vars,
                    self.allow_interaction && (self.is_down && self.is_over) || self.shortcut_press.is_some(),
                );
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(timer) = &self.shortcut_press {
                if timer.is_new(ctx) {
                    self.shortcut_press = None;
                    ctx.updates.subscriptions();
                    self.state.set_ne(ctx.vars, self.is_down && self.is_over);
                }
            }
        }
    }
    IsPressedNode {
        child,
        state,
        is_down: false,
        is_over: false,
        allow_interaction: false,
        shortcut_press: None,
    }
}

/// If the pointer is pressed in the widget or was captured during press.
///
/// This is `true` when [`is_pressed`] is `true` but also when
/// the pointer is not over the widget but the widget captured the pointer during the press.
///
/// [`is_pressed`]: fn@is_pressed
#[property(context)]
pub fn is_cap_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsCapPressedNode<C> {
        child: C,
        state: StateVar,
        is_down: bool,
        is_captured: bool,
        allow_interaction: bool,
        shortcut_press: Option<DeadlineVar>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsCapPressedNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(MouseInputEvent)
                .event(MouseCaptureEvent)
                .event(ClickEvent)
                .event(WindowFocusChangedEvent)
                .event(WidgetInfoChangedEvent);

            if let Some(s) = &self.shortcut_press {
                subs.var(ctx, s);
            }
            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.allow_interaction = ctx
                .info_tree
                .find(ctx.path.widget_id())
                .map(|w| w.allow_interaction())
                .unwrap_or(false);

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_down = false;
            self.is_captured = false;
            self.allow_interaction = false;
            self.shortcut_press = None;
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut update = false;

            // self.is_down:
            if let Some(args) = MouseInputEvent.update(args) {
                if args.is_primary() {
                    match args.state {
                        ButtonState::Pressed => {
                            if args.concerns_capture(ctx) {
                                self.is_down = true;
                                update = true;
                            }
                        }
                        ButtonState::Released => {
                            self.is_down = false;
                            update = true;
                        }
                    }
                }
                self.child.event(ctx, args);
            }
            // self.is_captured:
            else if let Some(args) = MouseCaptureEvent.update(args) {
                if args.concerns_widget(ctx) {
                    self.is_captured = args.is_got(ctx.path.widget_id());
                    update = true;
                }
                self.child.event(ctx, args);
            }
            // self.is_down = false;
            else if let Some(args) = WindowFocusChangedEvent.update(args) {
                if !args.focused && args.concerns_widget(ctx) {
                    self.is_down = false;
                    update = true;
                }
                self.child.event(ctx, args);
            }
            // self.shortcut_press:
            else if let Some(args) = ClickEvent.update(args) {
                if args.concerns_widget(ctx) && args.shortcut().is_some() {
                    // see `is_pressed` for details of what is happening here.
                    if self.shortcut_press.take().is_none() {
                        let duration = ctx.services.gestures().shortcut_pressed_duration;
                        if duration != Duration::default() {
                            self.shortcut_press = Some(ctx.timers.timeout(duration));
                            update = true;
                            ctx.updates.subscriptions();
                        }
                    } else {
                        update = true;
                        ctx.updates.subscriptions();
                    }
                    self.child.event(ctx, args);
                }
            }
            // self.allow_interaction
            else if let Some(args) = WidgetInfoChangedEvent.update(args) {
                if args.concerns_widget(ctx) {
                    self.allow_interaction = ctx
                        .info_tree
                        .find(ctx.path.widget_id())
                        .map(|w| w.allow_interaction())
                        .unwrap_or(false);

                    if !self.allow_interaction {
                        self.is_down = false;
                        self.shortcut_press = None;
                    }

                    update = true;
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if update {
                self.state
                    .set_ne(ctx.vars, self.is_down || self.is_captured || self.shortcut_press.is_some());
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(timer) = &self.shortcut_press {
                if timer.is_new(ctx) {
                    self.shortcut_press = None;
                    ctx.updates.subscriptions();
                    self.state.set_ne(ctx.vars, self.is_down || self.is_captured);
                }
            }
        }
    }
    IsCapPressedNode {
        child,
        state,
        is_down: false,
        is_captured: false,
        allow_interaction: false,
        shortcut_press: None,
    }
}

#[doc(no_inline)]
pub use crate::core::widget_base::{is_collapsed, is_disabled, is_enabled, is_hidden, is_hit_testable, is_visible};
