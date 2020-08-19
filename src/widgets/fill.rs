use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::{impl_ui_node, profiler::profile_scope, UiNode};

struct FillGradient<A: Var<LayoutPoint>, B: Var<LayoutPoint>, S: LocalVar<Vec<GradientStop>>> {
    start: A,
    end: B,
    stops: S,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    final_size: LayoutSize,
}

#[impl_ui_node(none)]
impl<A: Var<LayoutPoint>, B: Var<LayoutPoint>, S: LocalVar<Vec<GradientStop>>> UiNode for FillGradient<A, B, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.render_start = *self.start.get(ctx.vars);
        self.render_end = *self.end.get(ctx.vars);
        self.stops.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(start) = self.start.update(ctx.vars) {
            self.render_start = *start;
            self.render_start.x *= self.final_size.width;
            self.render_start.y *= self.final_size.height;
            ctx.updates.push_render();
        }
        if let Some(end) = self.end.update(ctx.vars) {
            self.render_end = *end;
            self.render_end.x *= self.final_size.width;
            self.render_end.y *= self.final_size.height;
            ctx.updates.push_render();
        }
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, _: PixelGrid) {
        self.render_start.x /= self.final_size.width;
        self.render_start.y /= self.final_size.height;
        self.render_end.x /= self.final_size.width;
        self.render_end.y /= self.final_size.height;

        self.final_size = final_size;

        self.render_start.x *= self.final_size.width;
        self.render_start.y *= self.final_size.height;
        self.render_end.x *= self.final_size.width;
        self.render_end.y *= self.final_size.height;
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
pub fn fill_gradient(
    start: impl IntoVar<LayoutPoint>,
    end: impl IntoVar<LayoutPoint>,
    stops: impl IntoVar<Vec<GradientStop>>,
) -> impl UiNode {
    FillGradient {
        start: start.into_var(),
        end: end.into_var(),
        stops: stops.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        final_size: LayoutSize::zero(),
    }
}

struct FillColor<C: LocalVar<ColorF>> {
    color: C,
    final_size: LayoutSize,
}

#[impl_ui_node(none)]
impl<C: LocalVar<ColorF>> UiNode for FillColor<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.color.init_local(ctx.vars);
    }
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.color.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
    }
    fn arrange(&mut self, final_size: LayoutSize, _: PixelGrid) {
        self.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("render_color");
        frame.push_color(LayoutRect::from_size(self.final_size), *self.color.get_local());
    }
}

/// Fill the widget area with a color.
pub fn fill_color(color: impl IntoVar<ColorF>) -> impl UiNode {
    FillColor {
        color: color.into_local(),
        final_size: LayoutSize::default(),
    }
}
