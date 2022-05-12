use crate::{
    context::{InfoContext, LayoutContext, RenderContext, WidgetContext},
    render::{FrameBuilder, FrameUpdate},
    units::*,
    widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
    UiNode,
};

/// Instruments a [`UiNode`] to trace a span for each node method.
///
/// This node must be used in conjunction with the [`tracing`](https://docs.rs/tracing) crate.
pub struct InstrumentedNode<N, S> {
    node: N,
    span: S,
}
impl<N, S> InstrumentedNode<N, S>
where
    N: UiNode,
    S: Fn(&mut InfoContext, &'static str) -> tracing::span::EnteredSpan + 'static,
{
    /// Instrument the `node`.
    ///
    /// The `span` closure must use the `tracing` crate to create an entered span, the inputs
    /// a [`InfoContext`] and the [`UiNode`] method name, the closure is called for each UiNode methods.
    pub fn new(node: N, span: S) -> Self {
        InstrumentedNode { node, span }
    }
}
impl<N, S> UiNode for InstrumentedNode<N, S>
where
    N: UiNode,
    S: Fn(&mut InfoContext, &'static str) -> tracing::span::EnteredSpan + 'static,
{
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        let _span = (self.span)(ctx, "info");
        self.node.info(ctx, info);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        let _span = (self.span)(ctx, "subscriptions");
        self.node.subscriptions(ctx, subscriptions);
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        let _span = (self.span)(&mut ctx.as_info(), "init");
        self.node.init(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let _span = (self.span)(&mut ctx.as_info(), "deinit");
        self.node.deinit(ctx);
    }

    fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
        let _span = (self.span)(&mut ctx.as_info(), "event");
        self.node.event(ctx, args);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let _span = (self.span)(&mut ctx.as_info(), "update");
        self.node.update(ctx);
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let _span = (self.span)(&mut ctx.as_info(), "layout");
        self.node.layout(ctx, wl)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        let _span = (self.span)(&mut ctx.as_info(), "render");
        self.node.render(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        let _span = (self.span)(&mut ctx.as_info(), "render_update");
        self.node.render_update(ctx, update);
    }
}
