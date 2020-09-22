use crate::core::color::Rgba;
use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::GradientStop;
use crate::core::units::*;
use crate::core::var::*;
use crate::core::{impl_ui_node, profiler::profile_scope, UiNode};

struct FillGradientNode<A: LocalVar<Point>, B: LocalVar<Point>, S: LocalVar<Vec<GradientStop>>> {
    start: A,
    end: B,
    stops: S,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: LocalVar<Point>, B: LocalVar<Point>, S: LocalVar<Vec<GradientStop>>> UiNode for FillGradientNode<A, B, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.start.init_local(ctx.vars);
        self.end.init_local(ctx.vars);
        self.stops.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.start.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }
        if self.end.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.render_start = self.start.get_local().to_layout(final_size, ctx);
        self.render_end = self.end.get_local().to_layout(final_size, ctx);
        self.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("render_gradient");

        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            self.stops.get_local(),
        );
    }
}

/// Fill the widget area with a linear gradient.
pub fn fill_gradient(start: impl IntoVar<Point>, end: impl IntoVar<Point>, stops: impl IntoVar<Vec<GradientStop>>) -> impl UiNode {
    FillGradientNode {
        start: start.into_local(),
        end: end.into_local(),
        stops: stops.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        final_size: LayoutSize::zero(),
    }
}

struct FillColorNode<C: LocalVar<Rgba>> {
    color: C,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<C: LocalVar<Rgba>> UiNode for FillColorNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.color.init_local(ctx.vars);
    }
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.color.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
    }
    fn arrange(&mut self, final_size: LayoutSize, _: &mut LayoutContext) {
        self.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("render_color");
        frame.push_color(LayoutRect::from_size(self.final_size), (*self.color.get_local()).into());
    }
}

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<Rgba>) -> impl UiNode {
    FillColorNode {
        color: color.into_local(),
        final_size: LayoutSize::default(),
    }
}
