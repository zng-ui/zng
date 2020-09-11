use super::BorderDetails;
use crate::core::color::Rgba;
use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::units::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::core::{impl_ui_node, property};
use crate::widgets::{fill_color, fill_gradient};

struct BackgroundNode<T: UiNode, B: UiNode> {
    child: T,
    background: B,
}

#[impl_ui_node(child)]
impl<T: UiNode, B: UiNode> UiNode for BackgroundNode<T, B> {
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

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let available_size = self.child.measure(available_size, ctx);
        self.background.measure(available_size, ctx);
        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.background.arrange(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        self.background.render(frame);
        self.child.render(frame);
    }
}

/// Custom background property. Allows using any other widget as a background.
#[property(inner, allowed_in_when: false)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
    BackgroundNode { child, background }
}

/// Single color background property.
///
/// This property applies a [`fill_color`] as [`background`].
#[property(inner)]
pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    background::set(child, fill_color(color))
}

/// Linear gradient background property.
///
/// This property applies a [`fill_gradient`] as [`background`].
#[property(inner)]
pub fn background_gradient(
    child: impl UiNode,
    start: impl IntoVar<LayoutPoint>,
    end: impl IntoVar<LayoutPoint>,
    stops: impl IntoVar<Vec<GradientStop>>,
) -> impl UiNode {
    background::set(child, fill_gradient(start, end, stops))
}

struct ForegroundNode<T: UiNode, B: UiNode> {
    child: T,
    foreground: B,
}

#[impl_ui_node(child)]
impl<T: UiNode, B: UiNode> UiNode for ForegroundNode<T, B> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.foreground.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.child.deinit(ctx);
        self.foreground.deinit(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        self.foreground.update(ctx);
    }
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.child.update_hp(ctx);
        self.foreground.update_hp(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let available_size = self.child.measure(available_size, ctx);
        self.foreground.measure(available_size, ctx);
        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.foreground.arrange(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        self.child.render(frame);
        self.foreground.render(frame);
    }
}

/// Custom foreground property. Allows using any other widget as a foreground overlay.
#[property(inner, allowed_in_when: false)]
pub fn foreground(child: impl UiNode, foreground: impl UiNode) -> impl UiNode {
    ForegroundNode { child, foreground }
}

/// Foreground highlight border overlay.
#[property(inner)]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<LayoutSideOffsets>,
    widths: impl IntoVar<LayoutSideOffsets>,
    details: impl IntoVar<BorderDetails>,
) -> impl UiNode {
    use crate::properties as p;
    let border = p::border::set(crate::core::FillUiNode, widths, details);
    foreground::set(child, p::margin::set(border, offsets))
}
