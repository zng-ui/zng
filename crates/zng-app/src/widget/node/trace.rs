use std::ops::ControlFlow;

use zng_layout::unit::PxSize;
use zng_var::BoxAnyVarValue;

use crate::{
    render::{FrameBuilder, FrameUpdate},
    update::WidgetUpdates,
    widget::{
        WidgetUpdateMode,
        info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
        node::{UiNode, UiNodeImpl, UiNodeOpMethod},
    },
};

use super::UiNodeListObserver;

pub(super) struct TraceNode {
    node: UiNode,
    trace: Box<dyn FnMut(&mut dyn UiNodeImpl, UiNodeOpMethod, &mut dyn FnMut(&mut dyn UiNodeImpl)) + Send + 'static>,
}
impl TraceNode {
    pub(super) fn new<S>(node: UiNode, mut enter_mtd: impl FnMut(UiNodeOpMethod) -> S + Send + 'static) -> Self {
        Self {
            node,
            trace: Box::new(move |node, op, call| {
                let _span = if let Some(w) = node.as_widget() {
                    let mut s = None;
                    w.with_context(WidgetUpdateMode::Bubble, &mut || s = Some(enter_mtd(op)));
                    match s {
                        Some(s) => s,
                        None => enter_mtd(op),
                    }
                } else {
                    enter_mtd(op)
                };

                call(node);

                if let Some(w) = node.as_widget() {
                    let mut _span = Some(_span);
                    w.with_context(WidgetUpdateMode::Bubble, &mut || drop(_span.take()));
                }
            }),
        }
    }
}
// TODO(breaking) replace UiNodeOpMethod with enum for all methods, with read-only peeking of args
impl UiNodeImpl for TraceNode {
    fn children_len(&self) -> usize {
        self.node.children_len()
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        self.node.0.with_child(index, visitor)
    }

    fn is_list(&self) -> bool {
        self.node.0.is_list()
    }

    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        self.node.0.for_each_child(visitor);
    }

    fn try_for_each_child(
        &mut self,
        visitor: &mut dyn FnMut(usize, &mut UiNode) -> ControlFlow<BoxAnyVarValue>,
    ) -> ControlFlow<BoxAnyVarValue> {
        self.node.0.try_for_each_child(visitor)
    }

    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        self.node.0.par_each_child(visitor);
    }

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        self.node.0.par_fold_reduce(identity, fold, reduce)
    }

    fn init(&mut self) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Init, &mut |n| n.init())
    }

    fn deinit(&mut self) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Deinit, &mut |n| n.deinit())
    }

    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Info, &mut |n| n.info(info))
    }

    fn event(&mut self, update: &crate::update::EventUpdate) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Event, &mut |n| n.event(update))
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Update, &mut |n| n.update(updates))
    }

    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.node.0.update_list(updates, observer);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        let mut out = PxSize::zero();
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Measure, &mut |n| out = n.measure(wm));
        out
    }

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.node.0.measure_list(wm, measure, fold_size)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let mut out = PxSize::zero();
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Layout, &mut |n| out = n.layout(wl));
        out
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.node.0.layout_list(wl, layout, fold_size)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::Render, &mut |n| n.render(frame))
    }

    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        self.node.0.render_list(frame, render);
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        (self.trace)(self.node.as_dyn(), UiNodeOpMethod::RenderUpdate, &mut |n| n.render_update(update))
    }

    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        self.node.0.render_update_list(update, render_update);
    }

    fn as_widget(&mut self) -> Option<&mut dyn super::WidgetUiNodeImpl> {
        self.node.0.as_widget()
    }
}
