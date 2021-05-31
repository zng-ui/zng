//! Widget state properties, [`is_hovered`](fn@is_hovered), [`is_pressed`](fn@is_pressed) and more.

use std::time::Duration;

use zero_ui_core::widget_base::{IsHitTestable, WidgetHitTestableExt};

use crate::core::mouse::*;
use crate::core::sync::TimeElapsed;
use crate::core::window::WindowBlurEvent;
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
        is_hovered: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsHoveredNode<C> {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.is_hovered = false;
            self.state.set_ne(ctx.vars, false);
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args) {
            if let Some(args) = update.is::<MouseEnterEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_hovered = true;
                }
                self.child.event(ctx, MouseEnterEvent, args);
            } else if let Some(args) = update.is::<MouseLeaveEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_hovered = false;
                }
                self.child.event(ctx, MouseLeaveEvent, args);
            } else {
                self.child.event(ctx, update, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(false) = IsEnabled::get_new(ctx.vars) {
                self.is_hovered = false;
            }
            self.state.set_ne(ctx.vars, self.is_hovered);
            self.child.update(ctx);
        }
    }
    IsHoveredNode {
        child,
        state,
        is_hovered: false,
    }
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
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.state.set_ne(ctx.vars, false);
            self.is_hovered = false;
            self.is_captured = false;
        }

        fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args) {
            if let Some(args) = update.is::<MouseEnterEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_hovered = true;
                }
                self.child.event(ctx, MouseEnterEvent, args);
            } else if let Some(args) = update.is::<MouseLeaveEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_hovered = false;
                }
                self.child.event(ctx, MouseLeaveEvent, args);
            } else if let Some(args) = update.is::<MouseCaptureEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    if args.is_got(ctx.path.widget_id()) {
                        self.is_captured = true;
                    } else if args.is_lost(ctx.path.widget_id()) {
                        self.is_captured = false;
                    }
                }
                self.child.event(ctx, MouseCaptureEvent, args);
            } else {
                self.child.event(ctx, update, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(false) = IsEnabled::get_new(ctx.vars) {
                self.is_hovered = false;
                self.is_captured = false;
            }
            self.state.set_ne(ctx.vars, self.is_hovered || self.is_captured);
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
        is_shortcut_press: bool,

        shortcut_release: EventListener<TimeElapsed>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsPressedNode<C> {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_down = false;
            self.is_over = false;
            if self.is_shortcut_press {
                self.is_shortcut_press = false;
                self.shortcut_release = EventListener::response_never();
            }
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args) {
            if let Some(args) = update.is::<MouseEnterEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_over = true;
                }
                self.child.event(ctx, MouseEnterEvent, args);
            } else if let Some(args) = update.is::<MouseLeaveEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_over = false;
                }
                self.child.event(ctx, MouseLeaveEvent, args);
            } else if let Some(args) = update.is::<MouseInputEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.is_primary() {
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
            } else if let Some(args) = update.is::<ClickEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) && args.shortcut().is_some() {
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
                self.child.event(ctx, ClickEvent, args);
            } else if let Some(args) = update.is::<WindowBlurEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_down = false;
                }
                self.child.event(ctx, WindowBlurEvent, args);
            } else {
                self.child.event(ctx, update, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(false) = IsEnabled::get_new(ctx.vars) {
                self.is_down = false;
                self.is_over = false;
                if self.is_shortcut_press {
                    self.is_shortcut_press = false;
                    self.shortcut_release = EventListener::response_never();
                }
            } else if self.shortcut_release.has_updates(ctx.events) {
                self.is_shortcut_press = false;
            }
            self.state
                .set_ne(ctx.vars, (self.is_down && self.is_over) || self.is_shortcut_press);
        }
    }
    IsPressedNode {
        child,
        state,
        is_down: false,
        is_over: false,
        is_shortcut_press: false,
        shortcut_release: EventListener::response_never(),
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
        is_shortcut_press: bool,

        shortcut_release: EventListener<TimeElapsed>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for IsCapPressedNode<C> {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, false);
            self.is_down = false;
            self.is_captured = false;
            if self.is_shortcut_press {
                self.is_shortcut_press = false;
                self.shortcut_release = EventListener::response_never();
            }
            self.child.deinit(ctx);
        }

        fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args) {
            if let Some(args) = update.is::<MouseInputEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.is_primary() {
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
                self.child.event(ctx, MouseInputEvent, args);
            } else if let Some(args) = update.is::<MouseCaptureEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_captured = args.is_got(ctx.path.widget_id());
                }
                self.child.event(ctx, MouseCaptureEvent, args);
            } else if let Some(args) = update.is::<WindowBlurEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) {
                    self.is_down = false;
                }
                self.child.event(ctx, WindowBlurEvent, args);
            } else if let Some(args) = update.is::<ClickEvent>(args) {
                if IsEnabled::get(ctx.vars) && args.concerns_widget(ctx) && args.shortcut().is_some() {
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
                    self.child.event(ctx, ClickEvent, args);
                }
            } else {
                self.child.event(ctx, update, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(false) = IsEnabled::get_new(ctx.vars) {
                self.is_down = false;
                self.is_captured = false;
                if self.is_shortcut_press {
                    self.is_shortcut_press = false;
                    self.shortcut_release = EventListener::response_never();
                }
            } else if self.shortcut_release.has_updates(ctx.events) {
                self.is_shortcut_press = false;
            }
            self.state
                .set_ne(ctx.vars, self.is_down || self.is_captured || self.is_shortcut_press);
        }
    }
    IsCapPressedNode {
        child,
        state,
        is_down: false,
        is_captured: false,
        is_shortcut_press: false,

        shortcut_release: EventListener::response_never(),
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
    }
    impl<C: UiNode> IsHitTestableNode<C> {
        fn update_state(&self, ctx: &mut WidgetContext) {
            let hit_testable = IsHitTestable::get(ctx.vars) && ctx.widget_state.hit_testable();
            self.state.set_ne(ctx.vars, hit_testable);
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
    IsHitTestableNode { child, state }
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
        self.state.set_ne(ctx.vars, is_state);
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

struct IsVisibilityNode<C: UiNode> {
    child: C,
    state: StateVar,
    expected: Visibility,
}
impl<C: UiNode> IsVisibilityNode<C> {
    fn update_state(&self, ctx: &mut WidgetContext) {
        let vis = VisibilityContext::get(ctx.vars) | ctx.widget_state.visibility();
        let is_state = vis == self.expected;
        self.state.set_ne(ctx.vars, is_state);
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
