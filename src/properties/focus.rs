use super::StopPropagation;
use crate::core::context::*;
use crate::core::focus::*;
use crate::core::render::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{
    core::{
        event::{Event, EventListener},
        gesture::KeyShortcut,
        impl_ui_node,
        keyboard::{KeyDownEvent, KeyInputArgs},
        property,
        types::WidgetId,
    },
    prelude::Windows,
};

/// Enables a widget to receive focus.
#[property(context)]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    FocusableNode {
        child,
        is_focusable: focusable.into_local(),
    }
}

/// Customizes the widget order during TAB navigation.
#[property(context)]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {
    TabIndexNode {
        child,
        tab_index: tab_index.into_local(),
    }
}

/// Widget is a focus scope.
///
/// Focus scopes are also [`focusable`] by default.
#[property(context)]
pub fn focus_scope(child: impl UiNode, focus_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: focus_scope.into_local(),
    }
}

/// Focus scope returns focus to last focused.
///
/// The last focused widget within the scope is remembered. Focus is restored when the scope receives direct focus.
/// If the focus cannot be restored, behaves like [`focus_first`].
///
/// This property does nothing if not set in a [`focus_scope`] widget.
///
/// See also [`is_return_focus`](crate::properties::is_return_focus).
#[property(context)]
pub fn remember_last_focus(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    RememberLastFocusNode {
        child,
        enabled: enabled.into_local(),
        return_focus: None,
        is_in_scope: None,
        return_focus_version: 0,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// Focus scope moves focus to first child.
///
/// When the focus scope receives direct focus it moves the focus to its first descendent, considering tab indexes.
///
/// This property does nothing if not set in a [`focus_scope`] widget.
pub fn focus_first(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    FocusFirstNode {
        child,
        enabled: enabled.into_local(),
        is_in_scope: None,
        focus_changed: FocusChangedEvent::never(),
    }
}

/// Tab navigation within this widget.
#[property(context)]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    TabNavNode {
        child,
        tab_nav: tab_nav.into_local(),
    }
}

/// Arrows navigation within this widget.
#[property(context)]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    DirectionalNavNode {
        child,
        directional_nav: directional_nav.into_local(),
    }
}

/// Keyboard shortcut that focus this widget.
#[property(context)]
pub fn focus_shortcut(child: impl UiNode, shortcut: impl IntoVar<KeyShortcut>) -> impl UiNode {
    FocusShortcutNode {
        child,
        shortcut: shortcut.into_var(),
        key_down: KeyDownEvent::never(),
    }
}

struct FocusableNode<C: UiNode, E: LocalVar<bool>> {
    child: C,
    is_focusable: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for FocusableNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_focusable.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.is_focusable.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().focusable = Some(*self.is_focusable.get_local());
        self.child.render(frame);
    }
}

struct TabIndexNode<C: UiNode, T: LocalVar<TabIndex>> {
    child: C,
    tab_index: T,
}
#[impl_ui_node(child)]
impl<C, T> UiNode for TabIndexNode<C, T>
where
    C: UiNode,
    T: LocalVar<TabIndex>,
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.tab_index.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.tab_index.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().tab_index = Some(*self.tab_index.get_local());
        self.child.render(frame);
    }
}

struct FocusScopeNode<C: UiNode, E: LocalVar<bool>> {
    child: C,
    is_focus_scope: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for FocusScopeNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_focus_scope.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.is_focus_scope.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().scope = Some(*self.is_focus_scope.get_local());
        self.child.render(frame);
    }
}

struct TabNavNode<C: UiNode, E: LocalVar<TabNav>> {
    child: C,
    tab_nav: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<TabNav>> UiNode for TabNavNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.tab_nav.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.tab_nav.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().tab_nav = Some(*self.tab_nav.get_local());
        self.child.render(frame);
    }
}

struct DirectionalNavNode<C: UiNode, E: LocalVar<DirectionalNav>> {
    child: C,
    directional_nav: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<DirectionalNav>> UiNode for DirectionalNavNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.directional_nav.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.directional_nav.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().directional_nav = Some(*self.directional_nav.get_local());
        self.child.render(frame);
    }
}

struct FocusShortcutNode<C: UiNode, S: Var<KeyShortcut>> {
    child: C,
    shortcut: S,
    key_down: EventListener<KeyInputArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode, S: Var<KeyShortcut>> UiNode for FocusShortcutNode<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.key_down = ctx.events.listen::<KeyDownEvent>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let handled_key = StopPropagation::<KeyDownEvent>::key();
        if !ctx.event_state.flagged(handled_key) {
            let shortcut = Some(*self.shortcut.get(ctx.vars));
            for update in self.key_down.updates(ctx.events) {
                if update.shortcut() == shortcut {
                    ctx.services.req::<Focus>().focus_widget(ctx.widget_id, true);
                    ctx.event_state.flag(handled_key);
                    break;
                }
            }
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        self.child.render(frame);

        let focus = frame.meta().entry(FocusInfoKey).or_default();
        if focus.focusable.is_none() {
            focus.focusable = Some(true);
        }
    }
}

struct RememberLastFocusNode<C: UiNode, E: LocalVar<bool>> {
    child: C,
    enabled: E,
    is_in_scope: Option<bool>,
    return_focus: Option<WidgetId>,
    return_focus_version: u32,
    focus_changed: EventListener<FocusChangedArgs>,
}
context_var! {
    /// The widget to which the focus returns to in the current focus scope.
    pub struct ReturnFocusVar: Option<WidgetId> = return &None;
}
impl<C: UiNode, E: LocalVar<bool>> RememberLastFocusNode<C, E> {
    fn with_return_focus_context(&mut self, is_new: bool, ctx: &mut WidgetContext, action: impl FnOnce(&mut C, &mut WidgetContext)) {
        if self.is_scope(ctx) {
            let value = self.return_focus;
            let version = self.return_focus_version;
            let child = &mut self.child;
            ctx.vars
                .with_context(ReturnFocusVar, &value, is_new, version, || action(child, ctx));
        } else {
            action(&mut self.child, ctx);
        }
    }

    fn is_scope(&mut self, ctx: &mut WidgetContext) -> bool {
        if let Some(is) = self.is_in_scope {
            is
        } else {
            let is = current_is_scope(ctx);
            if !is {
                warn_println!("`remember_last_focus` only works in a focus scope");
            }
            self.is_in_scope = Some(is);
            is
        }
    }

    fn set_return_focus(&mut self, return_focus: Option<WidgetId>) -> bool {
        let is_new = self.return_focus != return_focus;
        if is_new {
            self.return_focus_version = self.return_focus_version.wrapping_add(1);
        }
        is_new
    }

    fn return_focus(&self, scope_path: &WidgetPath, ctx: &mut WidgetContext) -> Option<WidgetId> {
        ctx.services.req::<Windows>().window(ctx.window_id).ok().and_then(|w| {
            let frame = w.frame_info();
            if let Some(return_) = self.return_focus {
                if let Some(node) = frame.find(return_).and_then(|n| n.as_focusable()) {
                    if node.ancestors().any(|d| d.info.widget_id() == scope_path.widget_id()) {
                        // return focus is still valid.
                        return Some(return_);
                    }
                }
            }

            // fallback to first child
            frame
                .get(scope_path)
                .and_then(|s| s.as_focusable())
                .and_then(|s| s.first_descendant_sorted().map(|f| f.info.widget_id()))
        })
    }
}
#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for RememberLastFocusNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.enabled.init_local(ctx.vars);
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.is_in_scope = None;
        self.child.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let mut return_focus = self.return_focus;

        if self.is_scope(ctx) && *self.enabled.get(ctx.vars) {
            if let Some(update) = self.focus_changed.updates(ctx.events).last() {
                if let Some(path) = &update.new_focus {
                    if path.widget_id() == ctx.widget_id {
                        // we got direct focus, return focus or search first child.
                        if let Some(child) = self.return_focus(path, ctx) {
                            ctx.services.req::<Focus>().focus_widget(child, update.highlight);
                        }
                    } else if path.contains(ctx.widget_id) {
                        // focus inside scope, remember
                        return_focus = Some(path.widget_id());
                    }
                }
            }
        } else {
            return_focus = None;
        }

        let is_new = self.set_return_focus(return_focus);
        self.with_return_focus_context(is_new, ctx, |child, ctx| child.update(ctx));
    }
}

struct FocusFirstNode<C: UiNode, E: LocalVar<bool>> {
    child: C,
    enabled: E,
    is_in_scope: Option<bool>,
    focus_changed: EventListener<FocusChangedArgs>,
}

impl<C: UiNode, E: LocalVar<bool>> FocusFirstNode<C, E> {
    fn is_scope(&mut self, ctx: &mut WidgetContext) -> bool {
        if let Some(is) = self.is_in_scope {
            is
        } else {
            let is = current_is_scope(ctx);
            if !is {
                warn_println!("`focus_first` only works in a focus scope");
            }
            self.is_in_scope = Some(is);
            is
        }
    }
}
#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for FocusFirstNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.enabled.init_local(ctx.vars);
        self.focus_changed = ctx.events.listen::<FocusChangedEvent>();
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.is_in_scope = None;
        self.child.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.is_scope(ctx) && *self.enabled.get(ctx.vars) {
            if let Some(update) = self.focus_changed.updates(ctx.events).last() {
                if let Some(path) = &update.new_focus {
                    if path.widget_id() == ctx.widget_id {
                        // we got direct focus, search first child.
                        let first_child = ctx
                            .services
                            .req::<Windows>()
                            .window(path.window_id())
                            .ok()
                            .and_then(|w| w.frame_info().get(path))
                            .and_then(|s| s.as_focusable())
                            .and_then(|s| s.first_descendant_sorted())
                            .map(|f| f.info.widget_id());

                        if let Some(first) = first_child {
                            ctx.services.req::<Focus>().focus_widget(first, update.highlight);
                        }
                    }
                }
            }
        }
        self.child.update(ctx);
    }
}

fn current_is_scope(ctx: &mut WidgetContext) -> bool {
    let widget_id = ctx.widget_id;
    ctx.services
        .req::<Windows>()
        .window(ctx.window_id)
        .ok()
        .and_then(|w| w.frame_info().find(widget_id))
        .map(|wid| wid.as_focus_info().is_scope())
        .unwrap_or_default()
}
