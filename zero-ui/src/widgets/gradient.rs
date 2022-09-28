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
    LinearGradientNode::new(axis.into_var(), stops.into_var(), extend_mode.into_var()).cfg_boxed()
}

/// Node that fills the widget area with a Linear gradient with all features configurable.
pub fn linear_gradient_full(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    LinearGradientFullNode {
        g: LinearGradientNode::new(axis.into_var(), stops.into_var(), extend_mode.into_var()),
        tile_size: tile_size.into_var(),
        tile_spacing: tile_spacing.into_var(),
        render_tile_size: PxSize::zero(),
        render_tile_spacing: PxSize::zero(),
    }
    .cfg_boxed()
}

struct LinearGradientNode<A, S, E> {
    axis: A,
    stops: S,
    extend_mode: E,

    do_layout: bool,
    render_line: PxLine,
    render_stops: Vec<RenderGradientStop>,

    final_size: PxSize,
}
#[impl_ui_node(none)]
impl<A, S, E> LinearGradientNode<A, S, E>
where
    A: Var<LinearGradientAxis>,
    S: Var<GradientStops>,
    E: Var<ExtendMode>,
{
    fn new(axis: A, stops: S, extend_mode: E) -> Self {
        Self {
            axis,
            stops,
            extend_mode,

            do_layout: true,
            render_line: PxLine::zero(),
            render_stops: vec![],

            final_size: PxSize::zero(),
        }
    }

    #[UiNode]
    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        subs.vars(ctx).var(&self.axis).var(&self.stops).var(&self.extend_mode);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
        if self.axis.is_new(ctx) || self.stops.is_new(ctx) || self.extend_mode.is_new(ctx) {
            self.do_layout = true;
            ctx.updates.layout();
        }
    }
    #[UiNode]
    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        ctx.constrains().fill_size()
    }
    #[UiNode]
    fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
        let final_size = ctx.constrains().fill_size();
        if self.do_layout || self.final_size != final_size {
            self.do_layout = false;

            self.final_size = final_size;
            self.render_line = self.axis.get(ctx).layout(ctx);

            let length = self.render_line.length();

            ctx.with_constrains(
                |c| c.with_new_exact_x(length),
                |ctx| {
                    self.stops.get(ctx).layout_linear(
                        ctx.for_x(),
                        *self.extend_mode.get(ctx),
                        &mut self.render_line,
                        &mut self.render_stops,
                    )
                },
            );

            ctx.updates.render();
        }
        final_size
    }
    #[UiNode]
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            PxRect::from_size(self.final_size),
            self.render_line,
            &self.render_stops,
            (*self.extend_mode.get(ctx)).into(),
            self.final_size,
            PxSize::zero(),
        );
    }
}

struct LinearGradientFullNode<A, S, E, T, TS> {
    g: LinearGradientNode<A, S, E>,

    tile_size: T,
    tile_spacing: TS,

    render_tile_size: PxSize,
    render_tile_spacing: PxSize,
}

#[impl_ui_node(none)]
impl<A, S, E, T, TS> UiNode for LinearGradientFullNode<A, S, E, T, TS>
where
    A: Var<LinearGradientAxis>,
    S: Var<GradientStops>,
    E: Var<ExtendMode>,
    T: Var<Size>,
    TS: Var<Size>,
{
    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        subs.vars(ctx).var(&self.tile_size).var(&self.tile_spacing);
        self.g.subscriptions(ctx, subs);
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        self.g.update(ctx, updates);

        if self.tile_size.is_new(ctx) || self.tile_spacing.is_new(ctx) {
            ctx.updates.layout();
        }
    }

    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        ctx.constrains().fill_size()
    }
    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let final_size = ctx.constrains().fill_size();

        if self.g.final_size != final_size {
            ctx.updates.render();
        }

        self.g.final_size = self.render_tile_size;

        self.render_tile_size = self.tile_size.get(ctx.vars).layout(ctx.metrics, |_| final_size);
        self.render_tile_spacing = self.tile_spacing.get(ctx.vars).layout(ctx.metrics, |_| final_size);

        ctx.with_constrains(
            |c| c.with_max_size(self.render_tile_size).with_fill(true, true),
            |ctx| {
                self.g.layout(ctx, wl);
            },
        );

        self.g.final_size = final_size;

        final_size
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        frame.push_linear_gradient(
            PxRect::from_size(self.g.final_size),
            self.g.render_line,
            &self.g.render_stops,
            self.g.extend_mode.copy(ctx).into(),
            self.render_tile_size,
            self.render_tile_spacing,
        );
    }
}
