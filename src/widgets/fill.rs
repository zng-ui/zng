use crate::prelude::new_widget::*;

pub use webrender::api::GradientStop;

struct FillGradientNode<A: VarLocal<Point>, B: VarLocal<Point>, S: VarLocal<GradientStops>> {
    start: A,
    end: B,
    stops: S,
    render_start: LayoutPoint,
    render_end: LayoutPoint,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<A: VarLocal<Point>, B: VarLocal<Point>, S: VarLocal<GradientStops>> UiNode for FillGradientNode<A, B, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.start.init_local(ctx.vars);
        self.end.init_local(ctx.vars);
        self.stops.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.start.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.end.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }
        if self.stops.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.render_start = self.start.get_local().to_layout(final_size, ctx);
        self.render_end = self.end.get_local().to_layout(final_size, ctx);
        self.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            LayoutRect::from_size(self.final_size),
            self.render_start,
            self.render_end,
            self.stops.get_local(),
        );
    }
}

/// Fill the widget area with a linear gradient.
pub fn fill_gradient(start: impl IntoVar<Point>, end: impl IntoVar<Point>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    FillGradientNode {
        start: start.into_local(),
        end: end.into_local(),
        stops: stops.into_local(),
        render_start: LayoutPoint::zero(),
        render_end: LayoutPoint::zero(),
        final_size: LayoutSize::zero(),
    }
}

struct FillColorNode<C: VarLocal<Rgba>> {
    color: C,
    final_size: LayoutSize,
}
#[impl_ui_node(none)]
impl<C: VarLocal<Rgba>> UiNode for FillColorNode<C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.color.init_local(ctx.vars);
    }
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.color.update_local(ctx.vars).is_some() {
            ctx.updates.render();
        }
    }
    fn arrange(&mut self, final_size: LayoutSize, _: &mut LayoutContext) {
        self.final_size = final_size;
    }

    fn render(&self, frame: &mut FrameBuilder) {
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

/// Gradient stops for linear or radial gradients.
#[derive(Debug, Clone)]
pub struct GradientStops(pub Vec<GradientStop>);
impl std::ops::Deref for GradientStops {
    type Target = [GradientStop];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl_from_and_into_var! {
    fn from(stops: Vec<(f32, Rgba)>) -> GradientStops {
        GradientStops(stops.into_iter()
        .map(|(offset, color)| GradientStop {
            offset,
            color: color.into(),
        })
        .collect())
    }

    /// Gradient stops that are all evenly spaced.
    fn from(stops: Vec<Rgba>) -> GradientStops {{
        let point = 1. / (stops.len() as f32 - 1.);
        GradientStops(stops.into_iter()
        .enumerate()
        .map(|(i, color)| GradientStop {
            offset: (i as f32) * point,
            color: color.into(),
        })
        .collect())
    }}

    /// A single two color gradient stops. The first color is at offset `0.0`,
    /// the second color is at offset `1.0`.
    fn from((stop0, stop1): (Rgba, Rgba)) -> GradientStops {
        GradientStops(vec![
            GradientStop { offset: 0.0, color: stop0.into() },
            GradientStop { offset: 1.0, color: stop1.into() },
        ])
    }

    fn from(stops: Vec<GradientStop>) -> GradientStops {
        GradientStops(stops)
    }
}
