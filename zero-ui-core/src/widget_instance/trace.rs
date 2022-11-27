use crate::{
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, WidgetContext, WidgetUpdates},
    event::EventUpdate,
    render::{FrameBuilder, FrameUpdate},
    units::*,
    widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
    widget_instance::UiNode,
};

/// Debug helper for tracing the lifetime of [`UiNode`] method calls.
///
/// The node delegates to the traced node, before calling each method a closure is called with an [`InfoContext`]
/// and the method name as a `&'static str`, the closure can return a *span* that is dropped after the method delegation.
///
/// This node can be used in conjunction with the [`tracing`](https://docs.rs/tracing) crate for creating the span.
///
/// You can instantiate this trace using [`UiNode::trace`].
pub struct TraceNode<N, E> {
    node: N,
    enter_mtd: E,
}
impl<N, E, S> TraceNode<N, E>
where
    N: UiNode,
    E: Fn(&mut InfoContext, &'static str) -> S + Send + 'static,
{
    /// Wrap the `node`.
    ///
    /// Prefer using the [`UiNode::trace`] method.
    pub fn new(node: N, enter_mtd: E) -> Self {
        TraceNode { node, enter_mtd }
    }
}
impl<N, E, S> UiNode for TraceNode<N, E>
where
    N: UiNode,
    E: Fn(&mut InfoContext, &'static str) -> S + Send + 'static,
{
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let _span = (self.enter_mtd)(ctx, "info");
        self.node.info(ctx, info);
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "init");
        self.node.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "deinit");
        self.node.deinit(ctx);
    }

    fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "event");
        self.node.event(ctx, update);
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "update");
        self.node.update(ctx, updates);
    }

    fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "measure");
        self.node.measure(ctx, wm)
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "layout");
        self.node.layout(ctx, wl)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "render");
        self.node.render(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let _span = (self.enter_mtd)(&mut ctx.as_info(), "render_update");
        self.node.render_update(ctx, update);
    }
}
