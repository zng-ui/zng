//! Gesture events, [`on_click`](fn@on_click), [`on_shortcut`](fn@on_shortcut), [`on_context_click`](fn@on_context_click) and more.
//!
//! These events aggregate multiple lower-level events to represent a user interaction.
//! Prefer using these events over the events directly tied to an input device.

use super::event_property;
use crate::core::context::WidgetContext;
use crate::core::event::EventArgs;
use crate::core::gesture::*;
use crate::prelude::new_property::*;

event_property! {
    /// On widget click from any source and of any click count.
    ///
    /// This is the most general click handler, it raises for all possible sources of the [`ClickEvent`] and any number
    /// of consecutive clicks. Use [`click`](fn@click) to handle only primary button clicks or [`on_any_single_click`](fn@on_any_single_click)
    /// to not include double/triple clicks.
    pub fn any_click {
        event: ClickEvent,
        args: ClickArgs,
    }

    /// On widget click from any source but excluding double/triple clicks.
    ///
    /// This raises for all possible sources of [`ClickEvent`], but only when the click count is one. Use
    /// [`on_single_click`](fn@on_single_click) to handle only primary button clicks.
    pub fn any_single_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_single(),
    }

    /// On widget click from any source but exclusive double-clicks.
    ///
    /// This raises for all possible sources of [`ClickEvent`], but only when the click count is two. Use
    /// [`on_double_click`](fn@on_double_click) to handle only primary button clicks.
    pub fn any_double_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_double(),
    }

    /// On widget click from any source but exclusive triple-clicks.
    ///
    /// This raises for all possible sources of [`ClickEvent`], but only when the click count is three. Use
    /// [`on_triple_click`](fn@on_triple_click) to handle only primary button clicks.
    pub fn any_triple_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args|  args.concerns_widget(ctx) && args.is_triple(),
    }

    /// On widget click with the primary button and any click count.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary), but raises for any click count (double/triple clicks).
    /// Use [`on_any_click`](fn@on_any_click) to handle clicks from any button or [`on_single_click`](fn@on_single_click) to not include
    /// double/triple clicks.
    pub fn click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary(),
    }

    /// On widget click with the primary button, excluding double/triple clicks.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is one. Use
    /// [`on_any_single_click`](fn@on_any_single_click) to handle single clicks from any button.
    pub fn single_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary() && args.is_single(),
    }

    /// On widget click with the primary button and exclusive double-clicks.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is two. Use
    /// [`on_any_double_click`](fn@on_any_double_click) to handle double clicks from any button.
    pub fn double_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary() && args.is_double(),
    }

    /// On widget click with the primary button and exclusive triple-clicks.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is three. Use
    /// [`on_any_double_click`](fn@on_any_double_click) to handle double clicks from any button.
    pub fn triple_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_primary() && args.is_triple(),
    }

    /// On widget click with the secondary/context button.
    ///
    /// This raises only if the click [is context](ClickArgs::is_context).
    pub fn context_click {
        event: ClickEvent,
        args: ClickArgs,
        filter: |ctx, args| args.concerns_widget(ctx) && args.is_context(),
    }

    /// On keyboard shortcut press when the widget is focused.
    pub fn shortcut {
        event: ShortcutEvent,
        args: ShortcutArgs,
    }
}

/// Keyboard shortcuts that focus and clicks this widget.
///
/// When any of the `shortcuts` is pressed, focus and click this widget. The widget is only focused
/// if the parent window is focused and the widget is focusable.
#[property(context)]
pub fn click_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    ClickShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        shortcut_listener: ShortcutEvent::never(),
        kind: ShortcutClick::Primary,
    }
}
/// Keyboard shortcuts that focus and [context clicks](fn@on_context_click) this widget.
///
/// When any of the `shortcuts` is pressed, focus and context clicks this widget. The widget is only focused
/// if the parent window is focused and the widget is focusable.
#[property(context)]
pub fn context_click_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    ClickShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        shortcut_listener: ShortcutEvent::never(),
        kind: ShortcutClick::Context,
    }
}
struct ClickShortcutNode<C: UiNode, S: Var<Shortcuts>> {
    child: C,
    shortcuts: S,
    shortcut_listener: EventListener<ShortcutArgs>,
    kind: ShortcutClick,
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
                    // this request also focus the widget if the window is focused
                    // and the widget is focusable.
                    ctx.services
                        .req::<Gestures>()
                        .click_shortcut(ctx.path.window_id(), ctx.path.widget_id(), self.kind, args.clone());
                    break;
                }
            }
        }
    }
}
