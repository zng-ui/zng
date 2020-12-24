//! Gesture events, [`on_click`], [`on_shortcut`], [`on_context_click`] and more.
//!
//! These events aggregate multiple lower-level events to represent a user interaction.
//! Prefer using these events over the events directly tied to an input device.

use super::event_property;
use crate::core::context::WidgetContext;
use crate::core::event::EventArgs;
use crate::core::gesture::*;
use crate::prelude::new_property::*;

event_property! {
    /// Adds a handler for clicks in the widget from any mouse button.
    pub fn any_click {
        event: ClickEvent,
        args: ClickArgs,
    }

    pub fn any_single_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_single(),
    }

    pub fn any_double_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_double(),
    }

    pub fn any_triple_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_triple(),
    }

    /// Adds a handler for clicks in the widget from the left mouse button.
    pub fn click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary(),
    }

    pub fn single_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary() && args.is_single(),
    }

    pub fn double_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary() && args.is_double(),
    }

    pub fn triple_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary() && args.is_triple(),
    }

    pub fn context_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_context(),
    }

    pub fn shortcut {
        event: ShortcutEvent,
        args: ShortcutArgs,
    }
}

struct ClickShortcutNode<C: UiNode, S: Var<Shortcuts>> {
    child: C,
    shortcuts: S,
    shortcut_listener: EventListener<ShortcutArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode, S: Var<Shortcuts>> UiNode for ClickShortcutNode<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.shortcut_listener = ctx.events.listen::<ShortcutEvent>();
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.child.deinit(ctx);
        self.shortcut_listener = ShortcutEvent::never();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if self.shortcut_listener.has_updates(ctx.events) && IsEnabled::get(ctx.vars) {
            let shortcuts = self.shortcuts.get(ctx.vars);

            for args in self.shortcut_listener.updates(ctx.events) {
                if !args.stop_propagation_requested() && shortcuts.0.contains(&args.shortcut) {
                    // focus on shortcut, if focusable
                    ctx.services
                        .req::<Gestures>()
                        .click_shortcut(ctx.path.window_id(), ctx.path.widget_id(), args.clone());
                    break;
                }
            }
        }
    }
}

/// Keyboard shortcuts that focus and clicks this widget.
///
/// When any of the `shortcuts` is pressed, focus and click this widget.
#[property(context)]
pub fn click_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    ClickShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        shortcut_listener: ShortcutEvent::never(),
    }
}
