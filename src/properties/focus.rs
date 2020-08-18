use super::StopPropagation;
use crate::core::context::*;
use crate::core::focus::*;
use crate::core::render::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::core::{
    event::{Event, EventListener},
    gesture::KeyShortcut,
    impl_ui_node,
    keyboard::{KeyDownEvent, KeyInputArgs},
    property,
};

/// Enables a widget to receive focus.
#[property(context)]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    Focusable {
        child,
        is_focusable: focusable.into_local(),
    }
}

/// Customizes the widget order during TAB navigation.
#[property(context)]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {
    SetTabIndex {
        child,
        tab_index: tab_index.into_local(),
    }
}

/// If this widget is a focus scope.
///
/// Focus scopes are also [`focusable`] by default.
#[property(context)]
pub fn focus_scope(child: impl UiNode, focus_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScope {
        child,
        is_focus_scope: focus_scope.into_local(),
    }
}

/// Tab navigation within this widget.
#[property(context)]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    SetTabNav {
        child,
        tab_nav: tab_nav.into_local(),
    }
}

/// Arrows navigation within this widget.
#[property(context)]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    SetDirectionalNav {
        child,
        directional_nav: directional_nav.into_local(),
    }
}

/// Keyboard shortcut that focus this widget.
#[property(context)]
pub fn focus_shortcut(child: impl UiNode, shortcut: impl IntoVar<KeyShortcut>) -> impl UiNode {
    FocusShortcut {
        child,
        shortcut: shortcut.into_var(),
        key_down: KeyDownEvent::never(),
    }
}

struct Focusable<C: UiNode, E: LocalVar<bool>> {
    child: C,
    is_focusable: E,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for Focusable<C, E> {
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

struct SetTabIndex<C: UiNode, T: LocalVar<TabIndex>> {
    child: C,
    tab_index: T,
}

#[impl_ui_node(child)]
impl<C, T> UiNode for SetTabIndex<C, T>
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

struct FocusScope<C: UiNode, E: LocalVar<bool>> {
    child: C,
    is_focus_scope: E,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for FocusScope<C, E> {
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

struct SetTabNav<C: UiNode, E: LocalVar<TabNav>> {
    child: C,
    tab_nav: E,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<TabNav>> UiNode for SetTabNav<C, E> {
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

struct SetDirectionalNav<C: UiNode, E: LocalVar<DirectionalNav>> {
    child: C,
    directional_nav: E,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<DirectionalNav>> UiNode for SetDirectionalNav<C, E> {
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

struct FocusShortcut<C: UiNode, S: Var<KeyShortcut>> {
    child: C,
    shortcut: S,
    key_down: EventListener<KeyInputArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode, S: Var<KeyShortcut>> UiNode for FocusShortcut<C, S> {
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
