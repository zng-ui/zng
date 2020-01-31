use crate::core::context::*;
use crate::core::event::*;
use crate::core::events::*;
use crate::core::focus::*;
use crate::core::frame::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct Focusable<C: UiNode> {
    child: C,
    config: FocusableConfig,
    mouse_down: EventListener<MouseInputArgs>,
}

#[impl_ui_node(child)]
impl<C: UiNode> UiNode for Focusable<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.mouse_down = ctx.events.listen::<MouseDown>();
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.mouse_down.has_updates(ctx.events) && ctx.widget_is_hit() {
            ctx.services.require::<Focus>().focus_widget(ctx.widget_id);
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.widget_meta().set(FocusableInfo, self.config.clone());
        self.child.render(frame);
    }
}

/// Configuration of a focusable widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusableConfig {
    pub tab_index: u32,
}

impl Default for FocusableConfig {
    fn default() -> Self {
        FocusableConfig {
            tab_index: u32::max_value(),
        }
    }
}

state_key! {
    ///
    pub struct FocusableInfo: FocusableConfig;
}

/// Enables a widget to receive focus.
#[property(context_var)]
pub fn focusable(child: impl UiNode, config: FocusableConfig) -> impl UiNode {
    Focusable {
        child,
        config,
        mouse_down: EventListener::never(false),
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
