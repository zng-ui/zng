//! Keyboard focus properties, [`tab_index`](fn@tab_index), [`focusable`](fn@focusable) and more.

use crate::core::event::EventListener;
use crate::core::focus::*;
use crate::prelude::new_property::*;

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
#[property(context)]
pub fn focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: is_scope.into_local(),
        is_alt: false,
    }
}

/// Widget is the ALT focus scope.
///
/// ALT focus scopes are also, `TabIndex::SKIP`, `skip_directional_nav`, `TabNav::Cycle` and `DirectionalNav::Cycle` by default.
#[property(context)]
pub fn alt_focus_scope(child: impl UiNode, is_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScopeNode {
        child,
        is_focus_scope: is_scope.into_local(),
        is_alt: true,
    }
}

/// Behavior of a focus scope when it receives direct focus.
#[property(context)]
pub fn focus_scope_behavior(child: impl UiNode, behavior: impl IntoVar<FocusScopeOnFocus>) -> impl UiNode {
    FocusScopeBehaviorNode {
        child,
        behavior: behavior.into_local(),
    }
}

/// Tab navigation within this focus scope.
#[property(context)]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    TabNavNode {
        child,
        tab_nav: tab_nav.into_local(),
    }
}

/// Arrows navigation within this focus scope.
#[property(context)]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    DirectionalNavNode {
        child,
        directional_nav: directional_nav.into_local(),
    }
}

/// Keyboard shortcuts that focus this widget.
///
/// When any of the `shortcuts` is pressed, focus this widget the parent focusable widget.
#[property(context)]
pub fn focus_shortcut(child: impl UiNode, shortcuts: impl IntoVar<Shortcuts>) -> impl UiNode {
    FocusShortcutNode {
        child,
        shortcuts: shortcuts.into_var(),
        shortcut_listener: ShortcutEvent::never(),
    }
}

/// If directional navigation from outside this widget skips over it and its descendants.
///
/// Setting this to `true` is the directional navigation equivalent of setting `tab_index` to `SKIP`.
#[property(context)]
pub fn skip_directional(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    SkipDirectionalNode {
        child,
        enabled: enabled.into_local(),
    }
}

struct FocusableNode<C: UiNode, E: VarLocal<bool>> {
    child: C,
    is_focusable: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: VarLocal<bool>> UiNode for FocusableNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_focusable.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.is_focusable.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().focusable = Some(*self.is_focusable.get_local());
        self.child.render(frame);
    }
}

struct TabIndexNode<C: UiNode, T: VarLocal<TabIndex>> {
    child: C,
    tab_index: T,
}
#[impl_ui_node(child)]
impl<C, T> UiNode for TabIndexNode<C, T>
where
    C: UiNode,
    T: VarLocal<TabIndex>,
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.tab_index.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.tab_index.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().tab_index = Some(*self.tab_index.get_local());
        self.child.render(frame);
    }
}

struct SkipDirectionalNode<C: UiNode, E: VarLocal<bool>> {
    child: C,
    enabled: E,
}
#[impl_ui_node(child)]
impl<C, E> UiNode for SkipDirectionalNode<C, E>
where
    C: UiNode,
    E: VarLocal<bool>,
{
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.enabled.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.enabled.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().skip_directional = Some(*self.enabled.get_local());
        self.child.render(frame);
    }
}

struct FocusScopeNode<C: UiNode, E: VarLocal<bool>> {
    child: C,
    is_focus_scope: E,
    is_alt: bool,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: VarLocal<bool>> UiNode for FocusScopeNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_focus_scope.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.is_focus_scope.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let info = frame.meta().entry(FocusInfoKey).or_default();
        info.scope = Some(*self.is_focus_scope.get_local());
        if self.is_alt {
            info.alt_scope = true;

            if info.tab_index == None {
                info.tab_index = Some(TabIndex::SKIP);
            }
            if info.tab_nav == None {
                info.tab_nav = Some(TabNav::Cycle);
            }
            if info.directional_nav == None {
                info.directional_nav = Some(DirectionalNav::Cycle);
            }
            if info.skip_directional == None {
                info.skip_directional = Some(true);
            }
        }
        self.child.render(frame);
    }
}

struct TabNavNode<C: UiNode, E: VarLocal<TabNav>> {
    child: C,
    tab_nav: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: VarLocal<TabNav>> UiNode for TabNavNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.tab_nav.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.tab_nav.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().tab_nav = Some(*self.tab_nav.get_local());
        self.child.render(frame);
    }
}

struct DirectionalNavNode<C: UiNode, E: VarLocal<DirectionalNav>> {
    child: C,
    directional_nav: E,
}
#[impl_ui_node(child)]
impl<C: UiNode, E: VarLocal<DirectionalNav>> UiNode for DirectionalNavNode<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.directional_nav.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.directional_nav.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(FocusInfoKey).or_default().directional_nav = Some(*self.directional_nav.get_local());
        self.child.render(frame);
    }
}

struct FocusShortcutNode<C: UiNode, S: Var<Shortcuts>> {
    child: C,
    shortcuts: S,
    shortcut_listener: EventListener<ShortcutArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode, S: Var<Shortcuts>> UiNode for FocusShortcutNode<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.shortcut_listener = ctx.events.listen::<ShortcutEvent>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let shortcuts = self.shortcuts.get(ctx.vars);

        for args in self.shortcut_listener.updates(ctx.events) {
            if !args.stop_propagation_requested() && shortcuts.0.contains(&args.shortcut) {
                // focus on shortcut
                ctx.services.req::<Focus>().focus_widget_or_parent(ctx.path.widget_id(), true);

                args.stop_propagation();
                break;
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

struct FocusScopeBehaviorNode<C: UiNode, B: VarLocal<FocusScopeOnFocus>> {
    child: C,
    behavior: B,
}
#[impl_ui_node(child)]
impl<C: UiNode, B: VarLocal<FocusScopeOnFocus>> UiNode for FocusScopeBehaviorNode<C, B> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.behavior.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.behavior.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let info = frame.meta().entry(FocusInfoKey).or_default();
        info.on_focus = *self.behavior.get_local();
        if info.scope.is_none() {
            info.scope = Some(true);
        }
        self.child.render(frame);
    }
}
