use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::widgets::{fill_color, fill_gradient};
use crate::{impl_ui_node, property};

struct Background<T: UiNode, B: UiNode> {
    child: T,
    background: B,
}

#[impl_ui_node(child)]
impl<T: UiNode, B: UiNode> UiNode for Background<T, B> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.background.init(ctx);
        self.child.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.background.deinit(ctx);
        self.child.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.background.update(ctx);
        self.child.update(ctx);
    }
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.background.update_hp(ctx);
        self.child.update_hp(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let available_size = self.child.measure(available_size);
        self.background.measure(available_size);
        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.background.arrange(final_size);
        self.child.arrange(final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        self.background.render(frame);
        self.child.render(frame);
    }
}

#[property(inner)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    Background { child, background }
}

#[property(inner)]
pub fn background_color(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    Background {
        child,
        background: fill_color(color),
    }
}

#[property(inner)]
pub fn background_gradient(
    child: impl UiNode,
    start: impl IntoVar<LayoutPoint>,
    end: impl IntoVar<LayoutPoint>,
    stops: impl IntoVar<Vec<GradientStop>>,
) -> impl UiNode {
    Background {
        child,
        background: fill_gradient(start, end, stops),
    }
}
