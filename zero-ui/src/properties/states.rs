//! Widget state properties, [`is_hovered`](fn@is_hovered), [`is_pressed`](fn@is_pressed) and more.

use std::time::Duration;

use crate::core::{mouse::*, timer::DeadlineVar, window::WindowFocusChangedEvent};
use crate::prelude::new_property::*;

/// If the mouse pointer is over the widget or an widget descendant.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
///
/// This state property does not consider mouse capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_hovered`](fn@is_cap_hovered) to
/// include the captured state.
#[property(context)]
pub fn is_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsHoveredNode<C: UiNode> {
        child: C,
        state: StateVar,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsHoveredNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.event(MouseHoveredEvent).updates(&IsEnabled::update_mask(ctx));
            self.child.subscriptions(ctx, subscriptions);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            if let Some(args) = MouseHoveredEvent.update(args) {
                if IsEnabled::get(ctx) {
                    if args.is_mouse_enter(ctx.path) {
                        self.state.set_ne(ctx, true);
                    } else if args.is_mouse_leave(ctx.path) {
                        self.state.set_ne(ctx, false);
                    }
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(false) = IsEnabled::get_new(ctx) {
                self.state.set_ne(ctx.vars, false);
            }
            self.child.update(ctx);
        }
    }
    IsHoveredNode { child, state }
}

/// If the mouse pointer is over the widget or an widget descendant or captured by the widget.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
#[property(context)]
pub fn is_cap_hovered(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsCapHoveredNode<C> {
        child: C,
        state: StateVar,
        is_hovered: bool,
        is_captured: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsCapHoveredNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .event(MouseHoveredEvent)
                .event(MouseCaptureEvent)
                .updates(&IsEnabled::update_mask(ctx));

            self.child.subscriptions(ctx, subscriptions);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.state.set_ne(ctx.vars, false);
            self.is_hovered = false;
            self.is_captured = false;
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut updated = false;

            // self.is_hovered:
            if let Some(args) = MouseHoveredEvent.update(args) {
                if IsEnabled::get(ctx) {
                    if args.is_mouse_enter(ctx.path) {
                        self.is_hovered = true;
                        updated = true;
                    } else if args.is_mouse_leave(ctx.path) {
                        self.is_hovered = false;
                        updated = true;
                    }
                }
                self.child.event(ctx, args);
            }
            // self.is_captured:
            else if let Some(args) = MouseCaptureEvent.update(args) {
                if IsEnabled::get(ctx) && args.concerns_widget(ctx) {
                    if args.is_got(ctx.path.widget_id()) {
                        self.is_captured = true;
                        updated = true;
                    } else if args.is_lost(ctx.path.widget_id()) {
                        self.is_captured = false;
                        updated = true;
                    }
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if updated {
                self.state.set_ne(ctx.vars, self.is_hovered || self.is_captured);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(false) = IsEnabled::get_new(ctx) {
                self.is_hovered = false;
                self.is_captured = false;
                self.state.set_ne(ctx.vars, false);
            }

            self.child.update(ctx);
        }
    }
    IsCapHoveredNode {
        child,
        state,
        is_hovered: false,
        is_captured: false,
    }
}

/// If the pointer is pressed in the widget.
///
/// This is `true` when the mouse primary button started pressing in the widget
/// and the pointer is over the widget and the primary button is still pressed.
///
/// This is always `false` when the widget is [disabled](IsEnabled).
///
/// A keyboard shortcut press causes this to be `true` true for the [specified time period](Gestures::shortcut_pressed_duration).
///
/// This state property does not consider mouse capture, if the pointer is captured by the widget
/// but is not actually over the widget this is `false`, use [`is_cap_pressed`](fn@is_cap_pressed) to
/// include the captured state.
#[property(context)]
pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsPressedNode<C> {
        child: C,
        state: StateVar,
        is_down: bool,
        is_over: bool,
        shortcut_press: Option<DeadlineVar>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsPressedNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .event(MouseHoveredEvent)
                .event(MouseInputEvent)
                .event(ClickEvent)
                .event(WindowFocusChangedEvent)
                .updates(&IsEnabled::update_mask(ctx));

            if let Some(s) = &self.shortcut_press {
                subscriptions.var(ctx, s);
            }

            self.child.subscriptions(ctx, subscriptions);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_down = false;
            self.is_over = false;
            self.shortcut_press = None;
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut updated = false;

            // self.is_over:
            if let Some(args) = MouseHoveredEvent.update(args) {
                if IsEnabled::get(ctx) {
                    if args.is_mouse_enter(ctx.path) {
                        self.is_over = true;
                        updated = true;
                    } else if args.is_mouse_leave(ctx.path) {
                        self.is_over = false;
                        updated = true;
                    }
                }
                self.child.event(ctx, args);
            }
            // self.is_down:
            else if let Some(args) = MouseInputEvent.update(args) {
                if IsEnabled::get(ctx) && args.is_primary() {
                    match args.state {
                        ButtonState::Pressed => {
                            if args.concerns_capture(ctx) {
                                self.is_down = true;
                                updated = true;
                            }
                        }
                        ButtonState::Released => {
                            self.is_down = false;
                            updated = true;
                        }
                    }
                }
                self.child.event(ctx, args);
            }
            // self.shortcut_press:
            else if let Some(args) = ClickEvent.update(args) {
                if IsEnabled::get(ctx) && args.concerns_widget(ctx) && args.shortcut().is_some() {
                    // if a shortcut click happened, we show pressed for the duration of `shortcut_press`
                    // unless we where already doing that, then we just stop showing pressed, this causes
                    // a flickering effect when rapid clicks are happening.
                    if self.shortcut_press.take().is_none() {
                        let duration = ctx.services.gestures().shortcut_pressed_duration;
                        if duration != Duration::default() {
                            self.shortcut_press = Some(ctx.timers.timeout(duration));
                            updated = true;
                            ctx.updates.subscriptions();
                        }
                    } else {
                        updated = true;
                        ctx.updates.subscriptions();
                    }
                }
                self.child.event(ctx, args);
            }
            // self.is_down = false;
            else if let Some(args) = WindowFocusChangedEvent.update(args) {
                if !args.focused && IsEnabled::get(ctx) && args.concerns_widget(ctx) {
                    self.is_down = false;
                    updated = false;
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if updated {
                self.state
                    .set_ne(ctx.vars, (self.is_down && self.is_over) || self.shortcut_press.is_some());
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(false) = IsEnabled::get_new(ctx) {
                self.is_down = false;
                self.is_over = false;
                if self.shortcut_press.take().is_some() {
                    ctx.updates.subscriptions();
                }

                self.state.set_ne(ctx.vars, false);
            } else if let Some(timer) = &self.shortcut_press {
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
        shortcut_press: None,
    }
}

/// If the pointer is pressed in the widget or was captured during press.
///
/// This is `true` when [`is_pressed`](fn@is_pressed) is `true` but also when
/// the pointer is not over the widget but the widget captured the pointer during the press.
#[property(context)]
pub fn is_cap_pressed(child: impl UiNode, state: StateVar) -> impl UiNode {
    struct IsCapPressedNode<C> {
        child: C,
        state: StateVar,
        is_down: bool,
        is_captured: bool,
        shortcut_press: Option<DeadlineVar>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsCapPressedNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .event(MouseInputEvent)
                .event(MouseCaptureEvent)
                .event(ClickEvent)
                .event(WindowFocusChangedEvent)
                .updates(&IsEnabled::update_mask(ctx));

            if let Some(s) = &self.shortcut_press {
                subscriptions.var(ctx, s);
            }
            self.child.subscriptions(ctx, subscriptions);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_down = false;
            self.is_captured = false;
            self.shortcut_press = None;
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
            let mut updated = false;

            // self.is_down:
            if let Some(args) = MouseInputEvent.update(args) {
                if IsEnabled::get(ctx) && args.is_primary() {
                    match args.state {
                        ButtonState::Pressed => {
                            if args.concerns_capture(ctx) {
                                self.is_down = true;
                                updated = true;
                            }
                        }
                        ButtonState::Released => {
                            self.is_down = false;
                            updated = true;
                        }
                    }
                }
                self.child.event(ctx, args);
            }
            // self.is_captured:
            else if let Some(args) = MouseCaptureEvent.update(args) {
                if IsEnabled::get(ctx) && args.concerns_widget(ctx) {
                    self.is_captured = args.is_got(ctx.path.widget_id());
                    updated = true;
                }
                self.child.event(ctx, args);
            }
            // self.is_down = false;
            else if let Some(args) = WindowFocusChangedEvent.update(args) {
                if !args.focused && IsEnabled::get(ctx) && args.concerns_widget(ctx) {
                    self.is_down = false;
                    updated = true;
                }
                self.child.event(ctx, args);
            }
            // self.shortcut_press:
            else if let Some(args) = ClickEvent.update(args) {
                if IsEnabled::get(ctx) && args.concerns_widget(ctx) && args.shortcut().is_some() {
                    // see `is_pressed` for details of what is happening here.
                    if self.shortcut_press.take().is_none() {
                        let duration = ctx.services.gestures().shortcut_pressed_duration;
                        if duration != Duration::default() {
                            self.shortcut_press = Some(ctx.timers.timeout(duration));
                            updated = true;
                            ctx.updates.subscriptions();
                        }
                    } else {
                        updated = true;
                        ctx.updates.subscriptions();
                    }
                    self.child.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }

            if updated {
                self.state
                    .set_ne(ctx.vars, self.is_down || self.is_captured || self.shortcut_press.is_some());
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(false) = IsEnabled::get_new(ctx) {
                self.is_down = false;
                self.is_captured = false;
                if self.shortcut_press.take().is_some() {
                    ctx.updates.subscriptions();
                }
                self.state.set_ne(ctx.vars, false);
            } else if let Some(timer) = &self.shortcut_press {
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
        shortcut_press: None,
    }
}

pub use crate::core::widget_base::{is_collapsed, is_disabled, is_enabled, is_hidden, is_visible};
