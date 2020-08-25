use super::StopPropagation;
use crate::core::context::*;
use crate::core::focus::*;
use crate::core::render::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::core::{
    event::{Event, EventListener},
    gesture::{Shortcut, ShortcutArgs, ShortcutEvent},
    impl_ui_node, property,
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
/// ALT focus scopes are also, `TabIndex::SKIP`, `TabNav::Cycle` and `DirectionalNav::Cycle` by default.
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
///
/// When `shortcut` is pressed focus this widget if focusable or the parent focusable widget.
#[property(context)]
pub fn focus_shortcut(child: impl UiNode, shortcut: impl IntoVar<Shortcut>) -> impl UiNode {
    FocusShortcutNode {
        child,
        shortcut: shortcut.into_var(),
        shortcut_listener: ShortcutEvent::never(),
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
    is_alt: bool,
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
        }
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

struct FocusShortcutNode<C: UiNode, S: Var<Shortcut>> {
    child: C,
    shortcut: S,
    shortcut_listener: EventListener<ShortcutArgs>,
}
#[impl_ui_node(child)]
impl<C: UiNode, S: Var<Shortcut>> UiNode for FocusShortcutNode<C, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.shortcut_listener = ctx.events.listen::<ShortcutEvent>();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let handled_key = StopPropagation::<ShortcutEvent>::key();
        if !ctx.event_state.flagged(handled_key) {
            // if shortcut not handled

            let shortcut = *self.shortcut.get(ctx.vars);

            for update in self.shortcut_listener.updates(ctx.events) {
                if update.shortcut == shortcut {
                    // focus on shortcut

                    ctx.services.req::<Focus>().focus_widget_or_parent(ctx.widget_id, true);
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

struct FocusScopeBehaviorNode<C: UiNode, B: LocalVar<FocusScopeOnFocus>> {
    child: C,
    behavior: B,
}
#[impl_ui_node(child)]
impl<C: UiNode, B: LocalVar<FocusScopeOnFocus>> UiNode for FocusScopeBehaviorNode<C, B> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.behavior.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.behavior.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
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
