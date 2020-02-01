use crate::core::context::*;
use crate::core::event::*;
use crate::core::events::*;
use crate::core::focus::*;
use crate::core::frame::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct Focusable<C: UiNode, E: LocalVar<bool>> {
    child: C,
    enabled: E,
}

#[impl_ui_node(child)]
impl<C: UiNode, E: LocalVar<bool>> UiNode for Focusable<C, E> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.enabled.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.enabled.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if *self.enabled.get_local() {
            if !frame.widget_meta().contains(FocusableInfo) {
                frame.widget_meta().set(FocusableInfo, TabIndex::AUTO);
            }
        } else {
            frame.widget_meta().flag(FocusableDisabled);
            frame.widget_meta().remove(FocusableInfo);
        }
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
        if !frame.widget_meta().flagged(FocusableDisabled) {
            frame.widget_meta().set(FocusableInfo, *self.tab_index.get_local());
        }
        self.child.render(frame);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabIndex(u32);

impl TabIndex {
    /// Widget is not focusable.
    pub const NONE: TabIndex = TabIndex(0);

    /// Widget is focusable but uses the declaration order for navigation.
    pub const AUTO: TabIndex = TabIndex(u32::max_value());

    /// If is [NONE].
    #[inline]
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }

    /// If is [AUTO].
    #[inline]
    pub fn is_auto(self) -> bool {
        self == Self::AUTO
    }
}

state_key! {
    ///
    pub(crate) struct FocusableInfo: TabIndex;

    ///
    struct FocusableDisabled: ();
}

/// Enables a widget to receive focus.
#[property(context_var)]
pub fn focusable(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    Focusable {
        child,
        enabled: enabled.into_var().as_local(),
    }
}

struct FocusScope<C: UiNode> {
    child: C,
    config: FocusScopeConfig,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for FocusScope<C> {}

#[property(context_var)]
pub fn focus_scope(child: impl UiNode, config: FocusScopeConfig) -> impl UiNode {
    FocusScope { child, config }
}

/// Configuration of a focusable widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusScopeConfig {
    pub tab_index: u32,
    pub skip: bool,
    pub tab: Option<TabNav>,
    pub directional: Option<DirectionalNav>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabNav {
    Continue,
    Contained,
    Cycle,
    Once,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalNav {
    Continue,
    Contained,
    Cycle,
}
