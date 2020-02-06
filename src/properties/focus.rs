use crate::core::context::*;
use crate::core::focus::*;
use crate::core::frame::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

/// Enables a widget to receive focus.
#[property(context_var)]
pub fn focusable(child: impl UiNode, focusable: impl IntoVar<bool>) -> impl UiNode {
    Focusable {
        child,
        is_focusable: focusable.into_var().as_local(),
    }
}

/// Customizes the widget order during TAB navigation.
#[property(context_var)]
pub fn tab_index(child: impl UiNode, tab_index: impl IntoVar<TabIndex>) -> impl UiNode {
    SetTabIndex {
        child,
        tab_index: tab_index.into_var().as_local(),
    }
}

/// If this widget is a focus scope.
#[property(context_var)]
pub fn focus_scope(child: impl UiNode, focus_scope: impl IntoVar<bool>) -> impl UiNode {
    FocusScope {
        child,
        is_focus_scope: focus_scope.into_var().as_local(),
    }
}

/// Tab navigation within this widget.
#[property(context_var)]
pub fn tab_nav(child: impl UiNode, tab_nav: impl IntoVar<TabNav>) -> impl UiNode {
    SetTabNav {
        child,
        tab_nav: tab_nav.into_var().as_local(),
    }
}

/// Arrows navigation within this widget.
#[property(context_var)]
pub fn directional_nav(child: impl UiNode, directional_nav: impl IntoVar<DirectionalNav>) -> impl UiNode {
    SetDirectionalNav {
        child,
        directional_nav: directional_nav.into_var().as_local(),
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
        frame.widget_meta().set(IsFocusable, *self.is_focusable.get_local());
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
        frame.widget_meta().set(FocusTabIndex, *self.tab_index.get_local());
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
        frame.widget_meta().set(IsFocusScope, *self.is_focus_scope.get_local());
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
        frame.widget_meta().set(FocusTabNav, *self.tab_nav.get_local());
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
        frame.widget_meta().set(FocusDirectionalNav, *self.directional_nav.get_local());
        self.child.render(frame);
    }
}
