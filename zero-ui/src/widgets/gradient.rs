use crate::core::gradient::*;
use crate::prelude::new_widget::*;

/// Node that fills the widget area with a linear gradient defined by angle or points.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a linear gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a Linear gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Reflect)
}
/// Node that fills the widget area with a linear gradient with extend mode configurable.
pub fn linear_gradient_ext(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    #[ui_node(struct LinearGradientNode {
        #[var] axis: impl Var<LinearGradientAxis>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,

        render_line: PxLine,
        render_stops: Vec<RenderGradientStop>,

        final_size: PxSize,
    })]
    impl UiNode for LinearGradientNode {
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.axis.is_new(ctx) || self.stops.is_new(ctx) || self.extend_mode.is_new(ctx) {
                self.final_size = PxSize::zero();
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext, _: &mut WidgetMeasure) -> PxSize {
            ctx.constrains().fill_size()
        }

        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            let final_size = ctx.constrains().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                self.render_line = self.axis.get().layout(ctx);

                let length = self.render_line.length();

                ctx.with_constrains(
                    |c| c.with_new_exact_x(length),
                    |ctx| {
                        self.stops
                            .with(|s| s.layout_linear(ctx.for_x(), self.extend_mode.get(), &mut self.render_line, &mut self.render_stops))
                    },
                );

                ctx.updates.render();
            }
            final_size
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_linear_gradient(
                PxRect::from_size(self.final_size),
                self.render_line,
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_size,
                PxSize::zero(),
            );
        }
    }
    LinearGradientNode {
        axis: axis.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),

        render_line: PxLine::zero(),
        render_stops: vec![],

        final_size: PxSize::zero(),
    }
}
/// Node that fills the widget area with a Linear gradient with all features configurable.
pub fn linear_gradient_full(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    #[ui_node(struct LinearGradientFullNode {
        #[var] axis: impl Var<LinearGradientAxis>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,
        #[var] tile_size: impl Var<Size>,
        #[var] tile_spacing: impl Var<Size>,

        final_line: PxLine,
        final_stops: Vec<RenderGradientStop>,

        final_size: PxSize,
        final_tile_size: PxSize,
        final_tile_spacing: PxSize,
    })]
    impl UiNode for LinearGradientFullNode {
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.axis.is_new(ctx)
                || self.stops.is_new(ctx)
                || self.extend_mode.is_new(ctx)
                || self.tile_size.is_new(ctx)
                || self.tile_spacing.is_new(ctx)
            {
                self.final_size = PxSize::zero();
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext, _: &mut WidgetMeasure) -> PxSize {
            ctx.constrains().fill_size()
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            self.final_size = ctx.constrains().fill_size();

            self.final_tile_size = self.tile_size.get().layout(ctx.metrics, |_| self.final_size);
            self.final_tile_spacing = self.tile_spacing.get().layout(ctx.metrics, |_| self.final_size);

            self.final_line = ctx.with_constrains(|c| c.with_exact_size(self.final_tile_size), |ctx| self.axis.get().layout(ctx));

            let length = self.final_line.length();
            ctx.with_constrains(
                |c| c.with_new_exact_x(length),
                |ctx| {
                    self.stops
                        .with(|s| s.layout_linear(ctx.for_x(), self.extend_mode.get(), &mut self.final_line, &mut self.final_stops))
                },
            );

            ctx.updates.render();

            self.final_size
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_linear_gradient(
                PxRect::from_size(self.final_size),
                self.final_line,
                &self.final_stops,
                self.extend_mode.get().into(),
                self.final_tile_size,
                self.final_tile_spacing,
            );
        }
    }

    LinearGradientFullNode {
        axis: axis.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),
        tile_size: tile_size.into_var(),
        tile_spacing: tile_spacing.into_var(),

        final_line: PxLine::zero(),
        final_stops: vec![],

        final_size: PxSize::zero(),
        final_tile_size: PxSize::zero(),
        final_tile_spacing: PxSize::zero(),
    }
    .cfg_boxed()
}

/// Node that fills the widget area with a radial gradient defined by the center point and radius.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp)
pub fn radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<RadialGradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a linear gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<RadialGradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a Linear gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<RadialGradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Reflect)
}
/// Node that fill the widget area with a radial gradient with extend mode configurable.
pub fn radial_gradient_ext(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<RadialGradientRadius>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    #[ui_node(struct RadialGradientNode {
        #[var] center: impl Var<Point>,
        #[var] radius: impl Var<RadialGradientRadius>,
        #[var] stops: impl Var<GradientStops>,
        #[var] extend_mode: impl Var<ExtendMode>,

        render_stops: Vec<RenderGradientStop>,
        render_center: PxPoint,
        render_radius: PxSize,
        final_size: PxSize,
    })]
    impl UiNode for RadialGradientNode {
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.center.is_new(ctx) || self.radius.is_new(ctx) || self.stops.is_new(ctx) || self.extend_mode.is_new(ctx) {
                self.final_size = PxSize::zero();
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext, _: &mut WidgetMeasure) -> PxSize {
            ctx.constrains().fill_size()
        }

        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            let final_size = ctx.constrains().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(self.final_size),
                    |ctx| {
                        self.render_center = self
                            .center
                            .get()
                            .layout(ctx, |_| self.final_size.to_vector().to_point() * 0.5.fct());

                        self.render_radius = self.radius.get().layout(ctx, self.render_center);
                    },
                );

                ctx.with_constrains(
                    |c| c.with_exact_x(self.render_radius.width.max(self.render_radius.height)),
                    |ctx| {
                        self.stops
                            .with(|s| s.layout_radial(ctx.for_x(), self.extend_mode.get(), &mut self.render_stops))
                    },
                );
            }
            final_size
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_radial_gradient(
                PxRect::from_size(self.final_size),
                self.render_center,
                self.render_radius,
                &self.render_stops,
                self.extend_mode.get().into(),
                self.final_size,
                PxSize::zero(),
            );
        }
    }
    RadialGradientNode {
        center: center.into_var(),
        radius: radius.into_var(),
        stops: stops.into_var(),
        extend_mode: extend_mode.into_var(),

        render_stops: vec![],
        render_center: PxPoint::zero(),
        render_radius: PxSize::zero(),
        final_size: PxSize::zero(),
    }
}
