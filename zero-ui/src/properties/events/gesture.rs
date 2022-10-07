//! Gesture events and control, [`on_click`](fn@on_click), [`click_shortcut`](fn@click_shortcut) and more.
//!
//! These events aggregate multiple lower-level events to represent a user interaction.
//! Prefer using these events over the events directly tied to an input device.

use super::event_property;
use crate::core::gesture::*;
use crate::prelude::new_property::*;

event_property! {
    /// On widget click from any source and of any click count and the widget is enabled.
    ///
    /// This is the most general click handler, it raises for all possible sources of the [`CLICK_EVENT`] and any number
    /// of consecutive clicks. Use [`click`](fn@click) to handle only primary button clicks or [`on_any_single_click`](fn@on_any_single_click)
    /// to not include double/triple clicks.
    pub fn any_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click from any source and of any click count and the widget is disabled.
    pub fn disabled_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_disabled(ctx.path.widget_id()),
    }

    /// On widget click from any source but excluding double/triple clicks and the widget is enabled.
    ///
    /// This raises for all possible sources of [`CLICK_EVENT`], but only when the click count is one. Use
    /// [`on_single_click`](fn@on_single_click) to handle only primary button clicks.
    pub fn any_single_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_single() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click from any source but exclusive double-clicks and the widget is enabled.
    ///
    /// This raises for all possible sources of [`CLICK_EVENT`], but only when the click count is two. Use
    /// [`on_double_click`](fn@on_double_click) to handle only primary button clicks.
    pub fn any_double_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_double() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click from any source but exclusive triple-clicks and the widget is enabled.
    ///
    /// This raises for all possible sources of [`CLICK_EVENT`], but only when the click count is three. Use
    /// [`on_triple_click`](fn@on_triple_click) to handle only primary button clicks.
    pub fn any_triple_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_triple() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click with the primary button and any click count and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary), but raises for any click count (double/triple clicks).
    /// Use [`on_any_click`](fn@on_any_click) to handle clicks from any button or [`on_single_click`](fn@on_single_click) to not include
    /// double/triple clicks.
    pub fn click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click with the primary button, excluding double/triple clicks and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is one. Use
    /// [`on_any_single_click`](fn@on_any_single_click) to handle single clicks from any button.
    pub fn single_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_single() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click with the primary button and exclusive double-clicks and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is two. Use
    /// [`on_any_double_click`](fn@on_any_double_click) to handle double clicks from any button.
    pub fn double_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_double() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click with the primary button and exclusive triple-clicks and the widget is enabled.
    ///
    /// This raises only if the click [is primary](ClickArgs::is_primary) and the click count is three. Use
    /// [`on_any_double_click`](fn@on_any_double_click) to handle double clicks from any button.
    pub fn triple_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_primary() && args.is_triple() && args.is_enabled(ctx.path.widget_id()),
    }

    /// On widget click with the secondary/context button and the widget is enabled.
    ///
    /// This raises only if the click [is context](ClickArgs::is_context).
    pub fn context_click {
        event: CLICK_EVENT,
        args: ClickArgs,
        filter: |ctx, args| args.is_context() && args.is_enabled(ctx.path.widget_id()),
    }
}

/// Keyboard shortcuts that focus and clicks this widget.
///
/// When any of the `shortcuts` is pressed, focus and click this widget.
#[property(context)]
pub fn click_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    ClickShortcutNode {
        child,
        var_shortcuts: shortcuts.into_var(),
        kind: ShortcutClick::Primary,
        handle: None,
    }
}
/// Keyboard shortcuts that focus and [context clicks](fn@on_context_click) this widget.
///
/// When any of the `shortcuts` is pressed, focus and context clicks this widget.
#[property(context)]
pub fn context_click_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    ClickShortcutNode {
        child,
        var_shortcuts: shortcuts.into_var(),
        kind: ShortcutClick::Context,
        handle: None,
    }
}
#[impl_ui_node(struct ClickShortcutNode {
    child: impl UiNode,
    var_shortcuts: impl Var<Shortcuts>,
    kind: ShortcutClick,
    handle: Option<ShortcutsHandle>,
})]
impl UiNode for ClickShortcutNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        let s = self.var_shortcuts.get();
        self.handle = Some(Gestures::req(ctx.services).click_shortcut(s, self.kind, ctx.path.widget_id()));
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        self.child.update(ctx, updates);

        if let Some(s) = self.var_shortcuts.get_new(ctx) {
            self.handle = Some(Gestures::req(ctx.services).click_shortcut(s, self.kind, ctx.path.widget_id()));
        }
    }
}
