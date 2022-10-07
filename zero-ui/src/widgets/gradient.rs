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
    #[impl_ui_node(struct LinearGradientNode {
        var_axis: impl Var<LinearGradientAxis>,
        var_stops: impl Var<GradientStops>,
        var_extend_mode: impl Var<ExtendMode>,

        render_line: PxLine,
        render_stops: Vec<RenderGradientStop>,

        final_size: PxSize,
    })]
    impl UiNode for LinearGradientNode {
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.var_axis.is_new(ctx) || self.var_stops.is_new(ctx) || self.var_extend_mode.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.constrains().fill_size()
        }

        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            let final_size = ctx.constrains().fill_size();
            if self.final_size != final_size {
                self.final_size = final_size;
                self.render_line = self.var_axis.get().layout(ctx);

                let length = self.render_line.length();

                ctx.with_constrains(
                    |c| c.with_new_exact_x(length),
                    |ctx| {
                        self.var_stops.get().layout_linear(
                            ctx.for_x(),
                            self.var_extend_mode.get(),
                            &mut self.render_line,
                            &mut self.render_stops,
                        )
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
                self.var_extend_mode.get().into(),
                self.final_size,
                PxSize::zero(),
            );
        }
    }
    LinearGradientNode {
        var_axis: axis.into_var(),
        var_stops: stops.into_var(),
        var_extend_mode: extend_mode.into_var(),

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
    #[impl_ui_node(struct LinearGradientFullNode {
        var_axis: impl Var<LinearGradientAxis>,
        var_stops: impl Var<GradientStops>,
        var_extend_mode: impl Var<ExtendMode>,
        var_tile_size: impl Var<Size>,
        var_tile_spacing: impl Var<Size>,

        final_line: PxLine,
        final_stops: Vec<RenderGradientStop>,

        final_size: PxSize,
        final_tile_size: PxSize,
        final_tile_spacing: PxSize,
    })]
    impl UiNode for LinearGradientFullNode {
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if self.var_axis.is_new(ctx)
                || self.var_stops.is_new(ctx)
                || self.var_extend_mode.is_new(ctx)
                || self.var_tile_size.is_new(ctx)
                || self.var_tile_spacing.is_new(ctx)
            {
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.constrains().fill_size()
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            self.final_size = ctx.constrains().fill_size();

            self.final_tile_size = self.var_tile_size.get().layout(ctx.metrics, |_| self.final_size);
            self.final_tile_spacing = self.var_tile_spacing.get().layout(ctx.metrics, |_| self.final_size);

            self.final_line = ctx.with_constrains(|c| c.with_exact_size(self.final_tile_size), |ctx| self.var_axis.get().layout(ctx));

            let length = self.final_line.length();
            ctx.with_constrains(
                |c| c.with_new_exact_x(length),
                |ctx| {
                    self.var_stops
                        .get()
                        .layout_linear(ctx.for_x(), self.var_extend_mode.get(), &mut self.final_line, &mut self.final_stops)
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
                self.var_extend_mode.get().into(),
                self.final_tile_size,
                self.final_tile_spacing,
            );
        }
    }

    LinearGradientFullNode {
        var_axis: axis.into_var(),
        var_stops: stops.into_var(),
        var_extend_mode: extend_mode.into_var(),
        var_tile_size: tile_size.into_var(),
        var_tile_spacing: tile_spacing.into_var(),

        final_line: PxLine::zero(),
        final_stops: vec![],

        final_size: PxSize::zero(),
        final_tile_size: PxSize::zero(),
        final_tile_spacing: PxSize::zero(),
    }
    .cfg_boxed()
}
