//! Background properties, [`background_color`], [`background_gradient`] and more.

use crate::prelude::new_property::*;
use crate::widgets::{fill_color, linear_gradient, ExtendMode, GradientStops};

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
        self.background.render(frame); // TODO, disable events and focus for this?
        self.child.render(frame);
    }
}

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not hit-testable
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
/// This property applies a [`linear_gradient`] as [`background`] using the [`Clamp`](ExtendMode::Clamp) extend mode.
#[property(inner)]
pub fn background_gradient(child: impl UiNode, angle: impl IntoVar<AngleRadian>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    background::set(child, linear_gradient(angle, stops, ExtendMode::Clamp))
}
