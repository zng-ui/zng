use crate::{
    context::WidgetUpdates,
    event::EventUpdate,
    render::{FrameBuilder, FrameUpdate},
    units::*,
    widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
    widget_instance::UiNode,
};

/// Debug helper for tracing the lifetime of [`UiNode`] method calls.
///
/// The node delegates to the traced node, before calling each method a closure is called with the method name
/// as a `&'static str`, the closure can return a *span* that is dropped after the method delegation.
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
    E: Fn(&'static str) -> S + Send + 'static,
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
    E: Fn(&'static str) -> S + Send + 'static,
{
    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        let _span = (self.enter_mtd)("info");
        self.node.info(info);
    }

    fn init(&mut self) {
        let _span = (self.enter_mtd)("init");
        self.node.init();
    }

    fn deinit(&mut self) {
        let _span = (self.enter_mtd)("deinit");
        self.node.deinit();
    }

    fn event(&mut self, update: &EventUpdate) {
        let _span = (self.enter_mtd)("event");
        self.node.event(update);
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        let _span = (self.enter_mtd)("update");
        self.node.update(updates);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        let _span = (self.enter_mtd)("measure");
        self.node.measure(wm)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let _span = (self.enter_mtd)("layout");
        self.node.layout(wl)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        let _span = (self.enter_mtd)("render");
        self.node.render(frame);
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        let _span = (self.enter_mtd)("render_update");
        self.node.render_update(update);
    }
}
