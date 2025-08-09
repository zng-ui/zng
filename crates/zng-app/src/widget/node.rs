//! Widget nodes types, [`UiNode`], [`UiNodeList`] and others.

use std::any::Any;

mod adopt;
pub use adopt::*;

mod arc;
pub use arc::*;

mod extend;
pub use extend::*;

mod match_node;
pub use match_node::*;

mod when;
pub use when::*;

mod list;
pub use list::*;
use zng_app_proc_macros::widget;
use zng_layout::{context::LAYOUT, unit::PxSize};
use zng_var::{BoxAnyVarValue, ContextInitHandle, ResponseVar, response_done_var, response_var};

use crate::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
};

use super::{
    WIDGET, WidgetId, WidgetUpdateMode,
    base::{PARALLEL_VAR, Parallel},
    info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
};

/// Represents an [`UiNode`] implementation.
///
/// You can use the [`match_node`] helper to quickly declare a new node from a closure, most property nodes are implemented
/// using the match helpers. For more advanced nodes you can manually implement this trait.
pub trait UiNodeImpl: Any + Send {
    /// Gets the current count of children nodes.
    fn children_len(&self) -> usize;

    /// Gets if the node represents a list of other nodes.
    ///
    /// If `true` the node provides only minimal layout implementations and expects the caller
    /// to use [`measure_list`], [`layout_list`] or direct access to child nodes for layout.
    ///
    /// [`measure_list`]: UiNodeImpl::measure_list
    /// [`layout_list`]: UiNodeImpl::layout_list
    fn is_list(&self) -> bool {
        false
    }

    /// Visit a child node by `index`. If the index is not valid `visitor` is not called.
    ///
    /// Nodes with many children should also implement [`for_each_child`] and [`par_each_child`] for better performance.
    ///
    /// [`for_each_child`]: UiNodeImpl::for_each_child
    /// [`par_each_child`]: UiNodeImpl::par_each_child
    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode));

    /// Call `visitor` for each child node of `self`, one at a time.
    ///
    /// The closure parameters are the child index and the child.
    fn for_each_child(&mut self, visitor: &mut dyn FnMut(usize, &mut UiNode)) {
        #[cfg(debug_assertions)]
        if self.is_list() {
            tracing::warn!("UiNodeImpl is_list without implementing `for_each_child`");
        }

        for i in 0..self.children_len() {
            self.with_child(i, &mut |n| visitor(i, n));
        }
    }

    /// Calls `visitor` for each child node in parallel.
    ///
    /// The closure parameters are the child index and the child.
    fn par_each_child(&mut self, visitor: &(dyn Fn(usize, &mut UiNode) + Sync)) {
        #[cfg(debug_assertions)]
        if self.is_list() {
            tracing::warn!("UiNodeImpl is_list without implementing `par_each_child`");
        }

        for i in 0..self.children_len() {
            self.with_child(i, &mut |n| visitor(i, n));
        }
    }

    /// Calls `fold` for each child node in parallel, with fold accumulators produced by cloning `identity`, then merges the folded results
    /// using `reduce` to produce the final value also in parallel.
    ///
    /// If the `reduce` closure is [associative], an *append* like operation will produce a result in the same order as the input items.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        fold: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        reduce: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        #[cfg(debug_assertions)]
        if self.is_list() {
            tracing::warn!("UiNodeImpl is_list without implementing `par_fold_reduce`");
        }

        let _ = reduce;
        let mut accumulator = identity;
        for i in 0..self.children_len() {
            self.with_child(i, &mut |n| {
                accumulator = fold(std::mem::replace(&mut accumulator, BoxAnyVarValue::new(())), i, n);
            });
        }
        accumulator
    }

    /// Initializes the node in a new UI context.
    ///
    /// Common init operations are subscribing to variables and events and initializing data.
    /// You can use [`WIDGET`] to subscribe events and vars, the subscriptions live until the widget is deinited.
    ///
    /// If the node is a custom widget ([`is_widget`]) it must request an info, layout and render updates, other nodes
    /// do not need to request any sort of update on init.
    ///
    /// Note that this method can be called again, after a [`deinit`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    /// [`deinit`]: UiNode::deinit
    fn init(&mut self) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.0.init()),
            _ if PARALLEL_VAR.get().contains(Parallel::INIT) => {
                self.par_each_child(&|_, n| n.0.init());
            }
            _ => self.for_each_child(&mut |_, n| n.0.init()),
        }
    }

    /// Deinitializes the node in the current UI context.
    ///
    /// Common deinit operations include dropping allocations and handlers.
    ///
    /// If the node is a custom widget ([`is_widget`]) it must request an info, layout and render updates, other nodes
    /// do not need to request any sort of update on deinit.
    ///
    /// Note that [`init`] can be called again after this.
    ///
    /// [`is_widget`]: UiNode::is_widget
    /// [`init`]: UiNode::init
    fn deinit(&mut self) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.0.deinit()),
            _ if PARALLEL_VAR.get().contains(Parallel::DEINIT) => {
                self.par_each_child(&|_, n| n.0.deinit());
            }
            _ => self.for_each_child(&mut |_, n| n.0.deinit()),
        }
    }

    /// Builds widget info.
    ///
    /// This method is called every time there are structural changes in the UI tree such as a node added or removed, you
    /// can also request an info rebuild using [`WIDGET.update_info`].
    ///
    /// Only nodes in widgets that requested info rebuild and nodes in their ancestors receive this call. Other
    /// widgets reuse their info in the new info tree. The widget's latest built info is available in [`WIDGET.info`].
    ///
    /// Note that info rebuild has higher priority over event, update, layout and render, this means that if you set a variable
    /// and request info update the next info rebuild will still observe the old variable value, you can work around this issue by
    /// only requesting info rebuild after the variable updates.
    ///
    /// [`WIDGET.info`]: crate::widget::WIDGET::info
    /// [`WIDGET.update_info`]: crate::widget::WIDGET::update_info
    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.0.info(info)),
            _ => {
                #[cfg(debug_assertions)]
                if self.is_list() && PARALLEL_VAR.get().contains(Parallel::INFO) {
                    // info.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                    // so default parallel here would alloc
                    tracing::info!("UiNodeImpl is_list without implementing `info`");
                }
                self.for_each_child(&mut |_, n| n.0.info(info))
            }
        }
    }

    /// Receives an event.
    ///
    /// Every call to this method is for a single update of a single event type, you can listen to events
    /// by subscribing to then on init and using the [`Event::on`] method in this method to detect the event.
    ///
    /// Note that events sent to descendant nodes also flow through this method and must be delegated. If you observe
    /// an event for a descendant before delegating to the descendant this is a ***preview*** handling, in the normal handling
    /// you delegate first, then check the event propagation.
    ///
    /// [`Event::on`]: crate::event::Event::on
    fn event(&mut self, update: &EventUpdate) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.0.event(update)),
            _ if PARALLEL_VAR.get().contains(Parallel::DEINIT) => {
                self.par_each_child(&|_, n| n.0.event(update));
            }
            _ => self.for_each_child(&mut |_, n| n.0.event(update)),
        }
    }

    /// Receives variable and other non-event updates.
    ///
    /// Calls to this method aggregate all updates that happen in the last pass, multiple variables can be new at the same time.
    /// You can listen to variable updates by subscribing to then on init and using the [`Var::get_new`] method in this method to
    /// receive the new values.
    ///
    /// A custom update can be requested using the context [`WIDGET.update`]. Common update operations include reacting to variable
    /// changes that generate an intermediary value for layout or render, the update implementation uses [`WIDGET`] to request layout
    /// and render after updating the data. Note that for simple variables that are used directly on layout or render you can subscribe
    /// to that operation directly, skipping update.
    ///
    /// [`Var::get_new`]: zng_var::Var::get_new
    /// [`WIDGET.update`]: crate::widget::WIDGET::update
    fn update(&mut self, updates: &WidgetUpdates) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.0.update(updates)),
            _ if PARALLEL_VAR.get().contains(Parallel::DEINIT) => {
                self.par_each_child(&|_, n| n.0.update(updates));
            }
            _ => self.for_each_child(&mut |_, n| n.0.update(updates)),
        }
    }

    /// Does [`update`] and if the node is a list notifies list changes to the `observer`.
    fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        if self.is_list() {
            #[cfg(debug_assertions)]
            tracing::info!("UiNodeImpl is_list without implementing `update_list`");

            let len = self.children_len();
            self.update(updates);
            if len != self.children_len() {
                observer.reset();
            }
        } else {
            self.update(updates);
        }
    }

    /// Computes the widget size given the contextual layout metrics without actually updating the widget layout.
    ///
    /// Implementers must return the same size [`layout`] returns for the given [`LayoutMetrics`], without
    /// affecting the actual widget render. Panel widgets that implement some complex layouts need to get an
    /// what the widget would be given some constraints, this value is used to inform the actual [`layout`] call.
    ///
    /// Nodes that implement [`layout`] must also implement this method, the [`LAYOUT`] context can be used to retrieve the metrics,
    /// the [`WidgetMeasure`] parameter can be used to communicate with the parent layout, such as disabling inline layout, the
    /// returned [`PxSize`] is the desired size given the parent constraints.
    ///
    /// [`layout`]: Self::layout
    /// [`LayoutMetrics`]: zng_layout::context::LayoutMetrics
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    /// [`PxSize`]: zng_layout::unit::PxSize
    #[must_use]
    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        match self.children_len() {
            0 => LAYOUT.constraints().fill_size(),
            1 => {
                let mut r = PxSize::zero();
                self.with_child(0, &mut |c| r = c.measure(wm));
                r
            }
            _ => {
                #[cfg(debug_assertions)]
                if self.is_list() && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
                    // wm.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                    // so default parallel here would alloc
                    tracing::info!("UiNodeImpl is_list without implementing `measure`");
                }
                let mut accumulator = PxSize::zero();
                self.for_each_child(&mut |_, n| accumulator = accumulator.max(n.0.measure(wm)));
                accumulator
            }
        }
    }

    /// If the node [`is_list`] measure each child and combine the size using `fold_size`.
    ///
    /// If the node is not a list, simply measures it.
    ///
    /// [`is_list`]: UiNodeImpl::is_list
    #[must_use]
    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        if self.is_list() {
            match self.children_len() {
                0 => PxSize::zero(),
                1 => {
                    let mut r = PxSize::zero();
                    self.with_child(0, &mut |c| r = measure(0, c, wm));
                    r
                }
                _ => {
                    #[cfg(debug_assertions)]
                    if PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
                        // wm.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                        // so default parallel here would alloc
                        tracing::info!("UiNodeImpl is_list without implementing `measure_list`");
                    }

                    let mut accumulator = PxSize::zero();
                    self.for_each_child(&mut |i, n| {
                        let c_s = measure(i, n, wm);
                        accumulator = fold_size(accumulator, c_s)
                    });
                    accumulator
                }
            }
        } else {
            self.measure(wm)
        }
    }

    /// Computes the widget layout given the contextual layout metrics.
    ///
    /// Implementers must also implement [`measure`]. This method is called by the parent layout once the final constraints
    /// for the frame are defined, the [`LAYOUT`] context can be used to retrieve the constraints, the [`WidgetLayout`] parameter
    /// can be used to communicate layout metadata such as inline segments to the parent layout, the returned [`PxSize`] is the
    /// final size given the constraints.
    ///
    /// Only widgets and ancestors that requested layout or use metrics that changed since last layout receive this call. Other
    /// widgets reuse the last layout result.
    ///
    /// Nodes that render can also implement this operation just to observe the latest widget size, if changes are detected
    /// the [`WIDGET.render`] method can be used to request render.
    ///
    /// [`measure`]: Self::measure
    /// [`LayoutMetrics`]: zng_layout::context::LayoutMetrics
    /// [`constraints`]: zng_layout::context::LayoutMetrics::constraints
    /// [`WIDGET.render`]: crate::widget::WIDGET::render
    /// [`LAYOUT`]: zng_layout::context::LAYOUT
    /// [`PxSize`]: zng_layout::unit::PxSize
    #[must_use]
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        match self.children_len() {
            0 => LAYOUT.constraints().fill_size(),
            1 => {
                let mut r = PxSize::zero();
                self.with_child(0, &mut |c| r = c.layout(wl));
                r
            }
            _ => {
                #[cfg(debug_assertions)]
                if self.is_list() && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
                    // wl.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                    // so default parallel here would alloc
                    tracing::info!("UiNodeImpl is_list without implementing `layout`");
                }
                let mut accumulator = PxSize::zero();
                self.for_each_child(&mut |_, n| accumulator = accumulator.max(n.0.layout(wl)));
                accumulator
            }
        }
    }

    /// If the node [`is_list`] layout each child and combine the size using `fold_size`.
    ///
    /// If the node is not a list, simply layout it.
    ///
    /// [`is_list`]: UiNodeImpl::is_list
    #[must_use]
    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        if self.is_list() {
            match self.children_len() {
                0 => PxSize::zero(),
                1 => {
                    let mut r = PxSize::zero();
                    self.with_child(0, &mut |c| r = layout(0, c, wl));
                    r
                }
                _ => {
                    #[cfg(debug_assertions)]
                    if PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
                        // wm.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                        // so default parallel here would alloc
                        tracing::info!("UiNodeImpl is_list without implementing `layout_list`");
                    }

                    let mut accumulator = PxSize::zero();
                    self.for_each_child(&mut |i, n| {
                        let c_s = layout(i, n, wl);
                        accumulator = fold_size(accumulator, c_s)
                    });
                    accumulator
                }
            }
        } else {
            self.layout(wl)
        }
    }

    /// Generates render instructions and updates transforms and hit-test areas.
    ///
    /// This method does not generate pixels immediately, it generates *display items* that are visual building block instructions
    /// for the renderer that will run after the window *display list* is built.
    ///
    /// Only widgets and ancestors that requested render receive this call, other widgets reuse the display items and transforms
    /// from the last frame.
    fn render(&mut self, frame: &mut FrameBuilder) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.render(frame)),
            _ => {
                #[cfg(debug_assertions)]
                if self.is_list() && PARALLEL_VAR.get().contains(Parallel::RENDER) {
                    // frame.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                    // so default parallel here would alloc
                    tracing::info!("UiNodeImpl is_list without implementing `render`");
                }
                self.for_each_child(&mut |_, n| n.0.render(frame));
            }
        }
    }

    /// If the node [`is_list`] render each child.
    ///
    /// If the node is not a list, simply renders it.
    ///
    /// [`is_list`]: UiNodeImpl::is_list
    fn render_list(&mut self, frame: &mut FrameBuilder, render: &(dyn Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync)) {
        if self.is_list() {
            match self.children_len() {
                0 => {}
                1 => self.with_child(0, &mut |n| render(0, n, frame)),
                _ => {
                    #[cfg(debug_assertions)]
                    if PARALLEL_VAR.get().contains(Parallel::RENDER) {
                        // wm.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                        // so default parallel here would alloc
                        tracing::info!("UiNodeImpl is_list without implementing `render_list`");
                    }
                    self.for_each_child(&mut |i, n| render(i, n, frame));
                }
            }
        } else {
            self.render(frame);
        }
    }

    /// Updates values in the last generated frame.
    ///
    /// Some display item values and transforms can be updated directly, without needing to rebuild the display list. All [`FrameBuilder`]
    /// methods that accept a [`FrameValue<T>`] input can be bound to an ID that can be used to update that value.
    ///
    /// Only widgets and ancestors that requested render update receive this call. Note that if any other widget in the same window
    /// requests render all pending render update requests are upgraded to render requests.
    ///
    /// [`FrameValue<T>`]: crate::render::FrameValue
    fn render_update(&mut self, update: &mut FrameUpdate) {
        match self.children_len() {
            0 => {}
            1 => self.with_child(0, &mut |c| c.render_update(update)),
            _ => {
                #[cfg(debug_assertions)]
                if self.is_list() && PARALLEL_VAR.get().contains(Parallel::RENDER) {
                    // update.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                    // so default parallel here would alloc
                    tracing::info!("UiNodeImpl is_list without implementing `update`");
                }
                self.for_each_child(&mut |_, n| n.0.render_update(update));
            }
        }
    }

    /// If the node [`is_list`] render_update each child.
    ///
    /// If the node is not a list, simply render_update it.
    ///
    /// [`is_list`]: UiNodeImpl::is_list
    fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: &(dyn Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync)) {
        if self.is_list() {
            match self.children_len() {
                0 => {}
                1 => self.with_child(0, &mut |n| render_update(0, n, update)),
                _ => {
                    #[cfg(debug_assertions)]
                    if PARALLEL_VAR.get().contains(Parallel::RENDER) {
                        // wm.parallel_split() is too large to fit in BoxAnyVarValue stack space,
                        // so default parallel here would alloc
                        tracing::info!("UiNodeImpl is_list without implementing `render_list`");
                    }
                    self.for_each_child(&mut |i, n| render_update(i, n, update));
                }
            }
        } else {
            self.render_update(update);
        }
    }

    /// Gets the node implementation as a [`WidgetUiNodeImpl`], if the node defines a widget instance scope.
    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// Represents an [`UiNodeImpl`] that defines a widget instance scope.
///
/// Widget defining nodes implement this trait and [`UiNodeImpl::as_widget`].
pub trait WidgetUiNodeImpl: UiNodeImpl {
    /// Calls `visitor` with the [`WIDGET`] context of the widget instance defined by the node.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the widget in `visitor` will be copied to the
    /// caller widget context, otherwise they are ignored.
    fn with_context(&mut self, update_mode: WidgetUpdateMode, visitor: &mut dyn FnMut());
}

/// Represents a value that can become a [`UiNode`] instance.
#[diagnostic::on_unimplemented(note = "`IntoUiNode` is implemented for all `U: UiNodeImpl`")]
pub trait IntoUiNode {
    /// Instantiate the UI node.
    fn into_node(self) -> UiNode;
}

impl<U: UiNodeImpl> IntoUiNode for U {
    #[inline(always)]
    fn into_node(self) -> UiNode {
        UiNode::new(self)
    }
}
impl IntoUiNode for UiNode {
    #[inline(always)]
    fn into_node(self) -> UiNode {
        self
    }
}
impl<U: IntoUiNode> IntoUiNode for Option<U> {
    /// Unwrap or nil.
    fn into_node(self) -> UiNode {
        self.map(IntoUiNode::into_node).unwrap_or_else(UiNode::nil)
    }
}

/// Represents an UI tree node instance.
///
/// You can use the [`match_node`] helper to quickly declare a new node from a closure, most property nodes are implemented
/// using the match helpers. For more advanced nodes can implement the [`UiNodeImpl`] trait. Other types can be converted to nodes
/// if they implement [`IntoUiNode`].
///
/// [`match_node`]:fn@match_node
pub struct UiNode(Box<dyn UiNodeImpl>);

/// Constructors.
impl UiNode {
    /// New UI node instance from implementation.
    ///
    /// Note that [`IntoUiNode`] is implemented for all `U: UiNodeImpl` so you don't usually need to call this.
    pub fn new(implementation: impl UiNodeImpl) -> Self {
        Self(Box::new(implementation))
    }

    /// New UI node that does nothing and collapses layout.
    pub fn nil() -> Self {
        Self::new(NilUiNode)
    }
}

/// UI operations.
impl UiNode {
    /// Calls the [`UiNodeOp`].
    pub fn op(&mut self, op: UiNodeOp) {
        match op {
            UiNodeOp::Init => self.init(),
            UiNodeOp::Deinit => self.deinit(),
            UiNodeOp::Info { info } => self.info(info),
            UiNodeOp::Event { update } => self.event(update),
            UiNodeOp::Update { updates } => self.update(updates),
            UiNodeOp::Measure { wm, desired_size } => *desired_size = self.measure(wm),
            UiNodeOp::Layout { wl, final_size } => *final_size = self.layout(wl),
            UiNodeOp::Render { frame } => self.render(frame),
            UiNodeOp::RenderUpdate { update } => self.render_update(update),
        }
    }

    /// Initialize the node in a new UI context.
    ///
    /// See [`UiNodeImpl::init`] for more details.
    #[inline(always)]
    pub fn init(&mut self) {
        self.0.init();
    }

    /// Deinitialize the node in the current UI context.
    ///
    /// This must be called before dropping the node.
    ///
    /// After calling this you can move the node to a new context and call [`init`] again.
    ///
    /// See [`UiNodeImpl::deinit`] for more details.
    ///
    /// [`init`]: Self::init
    #[inline(always)]
    pub fn deinit(&mut self) {
        self.0.deinit();
    }

    /// Continue building widget info metadata.
    ///
    /// See [`UiNodeImpl::info`] for more details.
    #[inline(always)]
    pub fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.0.info(info);
    }

    /// Notify event update.
    ///
    /// See [`UiNodeImpl::event`] for more details.
    #[inline(always)]
    pub fn event(&mut self, update: &EventUpdate) {
        self.0.event(update);
    }

    /// Notify non-event update.
    ///
    /// See [`UiNodeImpl::update`] for more details.
    #[inline(always)]
    pub fn update(&mut self, updates: &WidgetUpdates) {
        self.0.update(updates);
    }

    /// Notify non-event update and observe list changes if the widget is a list.
    ///
    /// See [`UiNodeImpl::update_list`] for more details.
    #[inline(always)]
    pub fn update_list(&mut self, updates: &WidgetUpdates, observer: &mut impl UiNodeListObserver) {
        self.0.update_list(updates, observer);
    }

    /// Estimate node layout without actually updating the node render state.
    ///
    /// See [`UiNodeImpl::measure`] for more details.
    #[inline(always)]
    #[must_use]
    pub fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.0.measure(wm)
    }

    /// If the node [`is_list`] measure each child and combine the size using `fold_size`.
    ///
    /// If the node is not a list, simply measures it.
    ///
    /// See [`UiNodeImpl::measure_list`] for more details.
    ///
    /// [`is_list`]: UiNode::is_list
    #[inline(always)]
    #[must_use]
    pub fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: impl Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync,
        fold_size: impl Fn(PxSize, PxSize) -> PxSize + Sync,
    ) -> PxSize {
        self.0.measure_list(wm, &measure, &fold_size)
    }

    /// Update node layout.
    ///
    /// See [`UiNodeImpl::layout`] for more details.
    #[inline(always)]
    #[must_use]
    pub fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.0.layout(wl)
    }

    /// If the node [`is_list`] layout each child and combine the size using `fold_size`.
    ///
    /// If the node is not a list, simply layout it.
    ///
    /// See [`UiNodeImpl::layout_list`] for more details.
    ///
    /// [`is_list`]: UiNode::is_list
    #[inline(always)]
    #[must_use]
    pub fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        layout: impl Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync,
        fold_size: impl Fn(PxSize, PxSize) -> PxSize + Sync,
    ) -> PxSize {
        self.0.layout_list(wl, &layout, &fold_size)
    }

    /// Collect render instructions for a new frame.
    ///
    /// See [`UiNodeImpl::render`] for more details.
    #[inline(always)]
    pub fn render(&mut self, frame: &mut FrameBuilder) {
        self.0.render(frame)
    }

    /// If the node [`is_list`] render each child.
    ///
    /// If the node is not a list, simply renders it.
    ///
    /// See [`UiNodeImpl::render_list`] for more details.
    ///
    /// [`is_list`]: UiNode::is_list
    #[inline(always)]
    pub fn render_list(&mut self, frame: &mut FrameBuilder, render: impl Fn(usize, &mut UiNode, &mut FrameBuilder) + Sync) {
        self.0.render_list(frame, &render);
    }

    /// Collect render patches to apply to the previous frame.
    ///
    /// See [`UiNodeImpl::render_update`] for more details.
    #[inline(always)]
    pub fn render_update(&mut self, update: &mut FrameUpdate) {
        self.0.render_update(update);
    }

    /// If the node [`is_list`] render_update each child.
    ///
    /// If the node is not a list, simply render_update it.
    ///
    /// See [`UiNodeImpl::render_update_list`] for more details.
    ///
    /// [`is_list`]: UiNode::is_list
    #[inline(always)]
    pub fn render_update_list(&mut self, update: &mut FrameUpdate, render_update: impl Fn(usize, &mut UiNode, &mut FrameUpdate) + Sync) {
        self.0.render_update_list(update, &render_update);
    }
}

/// Children.
impl UiNode {
    /// Number of direct descendants of this node.
    pub fn children_len(&self) -> usize {
        self.0.children_len()
    }

    /// Call `visitor` with a exclusive reference to the child node identified by `index`.
    ///
    /// If the `index` is out of bounds the closure is not called and returns `None`.
    pub fn try_with_child<R>(&mut self, index: usize, visitor: impl FnOnce(&mut UiNode) -> R) -> Option<R> {
        let mut once = Some(visitor);
        let mut r = None;
        self.0.with_child(index, &mut |child| r = Some(once.take().unwrap()(child)));
        r
    }

    /// Call `visitor` with a exclusive reference to the child node identified by `index`.
    ///
    /// Panics if the `index` is out of bounds.
    pub fn with_child<R>(&mut self, index: usize, visitor: impl FnOnce(&mut UiNode) -> R) -> R {
        self.try_with_child(index, visitor).expect("index out of bounds")
    }

    /// Call `visitor` for each child node of `self`, one at a time.
    ///
    /// The closure parameters are the child index and the child.
    pub fn for_each_child(&mut self, mut visitor: impl FnMut(usize, &mut UiNode)) {
        self.0.for_each_child(&mut visitor);
    }

    /// Calls `visitor` for each child node in parallel.
    ///
    /// The closure parameters are the child index and the child.
    pub fn par_each_child(&mut self, visitor: impl Fn(usize, &mut UiNode) + Sync) {
        self.0.par_each_child(&visitor);
    }

    /// Calls `fold` for each child node in parallel, with fold accumulators produced by cloning `identity`, then merges the folded results
    /// using `reduce` to produce the final value also in parallel.
    ///
    /// If the `reduce` closure is [associative], an *append* like operation will produce a result in the same order as the input items.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    pub fn par_fold_reduce<T: zng_var::VarValue>(
        &mut self,
        identity: T,
        fold: impl Fn(T, usize, &mut UiNode) -> T + Sync,
        reduce: impl Fn(T, T) -> T + Sync,
    ) -> T {
        use zng_var::BoxAnyVarValue as B;
        self.0
            .par_fold_reduce(
                B::new(identity),
                &|accumulator, index, node| {
                    let r = fold(accumulator.downcast::<T>().unwrap(), index, node);
                    B::new(r)
                },
                &|a, b| {
                    let r = reduce(a.downcast::<T>().unwrap(), b.downcast::<T>().unwrap());
                    B::new(r)
                },
            )
            .downcast::<T>()
            .unwrap()
    }
}

/// Node type.
impl UiNode {
    /// Returns some reference to implementation of type `U`, if the node instance is of that implementation.
    pub fn downcast_ref<U: UiNodeImpl>(&self) -> Option<&U> {
        let u: &dyn Any = &*self.0;
        u.downcast_ref::<U>()
    }

    /// Returns some mutable reference to implementation of type `U`, if the node instance is of that implementation.
    pub fn downcast_mut<U: UiNodeImpl>(&mut self) -> Option<&mut U> {
        let u: &mut dyn Any = &mut *self.0;
        u.downcast_mut::<U>()
    }

    /// Gets if the node is an instance of implementation `U`.
    pub fn is<U: UiNodeImpl>(&self) -> bool {
        self.downcast_ref::<U>().is_some()
    }

    /// Gets if the node represents a list of other nodes.
    ///
    /// If `true` the node provides only minimal layout implementations and expects the caller
    /// to use [`measure_list`], [`layout_list`] or direct access to the child nodes for layout.
    ///
    /// [`measure_list`]: Self::measure_list
    /// [`layout_list`]: Self::layout_list
    pub fn is_list(&self) -> bool {
        self.0.is_list()
    }

    /// Returns a node that [`is_list`].
    ///
    /// If `self` is a list returns it unchanged.
    ///
    /// If `self` is nil returns an empty list node.
    ///
    /// Otherwise returns a new list node with `self` as the single entry.
    ///
    /// [`is_list`]: Self::is_list
    pub fn into_list(self) -> UiNode {
        if self.is_list() {
            self
        } else if self.is_nil() {
            ui_vec![].into_node()
        } else {
            ui_vec![self].into_node()
        }
    }

    /// Gets if is [`nil`].
    ///
    /// [`nil`]: Self::nil
    pub fn is_nil(&self) -> bool {
        self.is::<NilUiNode>()
    }

    /// Access widget node methods, if the node defines a widget context.
    ///
    /// [`is_widget`]: Self::is_widget
    pub fn as_widget(&mut self) -> Option<WidgetUiNode<'_>> {
        self.0.as_widget().map(WidgetUiNode)
    }

    /// Returns a node that defines a widget context.
    ///
    /// If this node already defines a widget just returns it, if not wraps it in a minimal widget implementation.
    ///
    /// See also [`init_widget`] for a node that awaits until `self` is inited to verify if a new widget really needs to be declared.
    pub fn into_widget(mut self) -> UiNode {
        if self.0.as_widget().is_some() {
            self
        } else {
            into_widget!(child = self)
        }
    }

    /// Returns a node that defines a widget context or will begin defining it after [`init`].
    ///
    /// Also returns a response var that contains or will contain the widget instance ID.
    ///
    /// If `self` is already an widget node simply returns it and the ID, otherwise returns a node that wraps `self`
    /// and checks again if `self` is a widget after init, if `self` is still not a widget after init the wrapper node starts
    /// defining a minimal widget context.
    ///
    /// Some nodes like [`ArcNode::take_on_init`] can only become widgets on init, this helper is an alternative to [`into_widget`]
    /// that avoids declaring a second wrapper widget in those cases. Note that because the wrapper node needs to define a widget context
    /// after the [`init`] call the wrapped `self` node will need to be reinited inside the new widget.
    ///
    /// [`init`]: Self::init
    /// [`into_widget`]: Self::into_widget
    pub fn init_widget(mut self) -> (UiNode, ResponseVar<WidgetId>) {
        if let Some(mut wgt) = self.as_widget() {
            let id = response_done_var(wgt.id());
            (self, id)
        } else {
            let (r, id) = response_var::<WidgetId>();
            let mut first_init = Some(r);
            let wgt = match_widget(self, move |c, op| {
                if let UiNodeOp::Init = op {
                    c.init();

                    if let Some(r) = first_init.take() {
                        if let Some(mut wgt) = c.node().as_widget() {
                            r.respond(wgt.id());
                        } else {
                            // reinit inside a new widget
                            c.deinit();
                            let not_wgt = std::mem::replace(c.node(), UiNode::nil());
                            *c.node() = into_widget!(child = not_wgt);
                            c.init();

                            r.respond(c.node().as_widget().unwrap().id());
                        }
                    }
                }
            });
            (wgt, id)
        }
    }

    /// Wraps the node in a node that, before delegating each method, calls a closure with
    /// the [`UiNodeOpMethod`], the closure can return a *span* that is dropped after the method delegation.
    ///
    /// You can use the [`tracing`](https://docs.rs/tracing) crate to create the span.
    pub fn trace<E, S>(self, mut enter_mtd: E) -> UiNode
    where
        Self: Sized,
        E: FnMut(UiNodeOpMethod) -> S + Send + 'static,
    {
        match_node(self, move |node, op| {
            let _span = enter_mtd(op.mtd());
            node.op(op);
        })
    }
}

/// Extra [`UiNode`] methods for nodes that define a widget instance context.
///
/// See [`UiNode::as_widget`] for more details.
pub struct WidgetUiNode<'u>(&'u mut dyn WidgetUiNodeImpl);

impl<'u> WidgetUiNode<'u> {
    /// Calls `visitor` with the [`WIDGET`] context of the widget instance defined by the node.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the widget in `visitor` will be copied to the
    /// caller widget context, otherwise they are ignored.
    pub fn with_context<R>(&mut self, update_mode: WidgetUpdateMode, visitor: impl FnOnce() -> R) -> R {
        let mut once = Some(visitor);
        let mut r = None;
        self.0.with_context(update_mode, &mut || r = Some(once.take().unwrap()()));
        r.unwrap()
    }

    /// Gets the widget instance ID.
    pub fn id(&mut self) -> WidgetId {
        self.with_context(WidgetUpdateMode::Ignore, || WIDGET.id())
    }

    // TODO other helpers, with_state, state_get?
}

/// See [`UiNode::into_widget`]
#[expect(non_camel_case_types)]
#[widget($crate::widget::node::into_widget)]
struct into_widget(crate::widget::base::WidgetBase);
#[zng_app_proc_macros::property(CHILD, capture, widget_impl(into_widget))]
fn child(child: impl IntoUiNode) {}
impl into_widget {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|b| {
            let child = b.capture_ui_node(crate::property_id!(Self::child)).unwrap();
            b.set_child(child);
        });
    }
}

struct NilUiNode;
impl UiNodeImpl for NilUiNode {
    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        PxSize::zero()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        PxSize::zero()
    }

    fn children_len(&self) -> usize {
        0
    }

    fn with_child(&mut self, _: usize, _: &mut dyn FnMut(&mut UiNode)) {}

    fn is_list(&self) -> bool {
        false
    }

    fn for_each_child(&mut self, _: &mut dyn FnMut(usize, &mut UiNode)) {}

    fn par_each_child(&mut self, _: &(dyn Fn(usize, &mut UiNode) + Sync)) {}

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        _: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        _: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        identity
    }

    fn init(&mut self) {}

    fn deinit(&mut self) {}

    fn info(&mut self, _: &mut WidgetInfoBuilder) {}

    fn event(&mut self, _: &EventUpdate) {}

    fn update(&mut self, _: &WidgetUpdates) {}

    fn update_list(&mut self, _: &WidgetUpdates, _: &mut dyn UiNodeListObserver) {}

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        _: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        _: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.measure(wm)
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        _: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        _: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.layout(wl)
    }

    fn render(&mut self, _: &mut FrameBuilder) {}

    fn render_update(&mut self, _: &mut FrameUpdate) {}

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// A UI node that fills the available layout space.
///
/// The space is blank, the node does nothing other then layout to fill.
pub struct FillUiNode;
impl UiNodeImpl for FillUiNode {
    fn children_len(&self) -> usize {
        0
    }

    fn with_child(&mut self, _: usize, _: &mut dyn FnMut(&mut UiNode)) {}

    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        LAYOUT.constraints().fill_size()
    }

    fn is_list(&self) -> bool {
        false
    }

    fn for_each_child(&mut self, _: &mut dyn FnMut(usize, &mut UiNode)) {}

    fn par_each_child(&mut self, _: &(dyn Fn(usize, &mut UiNode) + Sync)) {}

    fn par_fold_reduce(
        &mut self,
        identity: BoxAnyVarValue,
        _: &(dyn Fn(BoxAnyVarValue, usize, &mut UiNode) -> BoxAnyVarValue + Sync),
        _: &(dyn Fn(BoxAnyVarValue, BoxAnyVarValue) -> BoxAnyVarValue + Sync),
    ) -> BoxAnyVarValue {
        identity
    }

    fn init(&mut self) {}

    fn deinit(&mut self) {}

    fn info(&mut self, _: &mut WidgetInfoBuilder) {}

    fn event(&mut self, _: &EventUpdate) {}

    fn update(&mut self, _: &WidgetUpdates) {}

    fn update_list(&mut self, _: &WidgetUpdates, _: &mut dyn UiNodeListObserver) {}

    fn measure_list(
        &mut self,
        wm: &mut WidgetMeasure,
        _: &(dyn Fn(usize, &mut UiNode, &mut WidgetMeasure) -> PxSize + Sync),
        _: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.measure(wm)
    }

    fn layout_list(
        &mut self,
        wl: &mut WidgetLayout,
        _: &(dyn Fn(usize, &mut UiNode, &mut WidgetLayout) -> PxSize + Sync),
        _: &(dyn Fn(PxSize, PxSize) -> PxSize + Sync),
    ) -> PxSize {
        self.layout(wl)
    }

    fn render(&mut self, _: &mut FrameBuilder) {}

    fn render_update(&mut self, _: &mut FrameUpdate) {}

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        None
    }
}

/// Wraps `child` in a node that provides a unique [`ContextInitHandle`], refreshed every (re)init.
///
/// [`ContextInitHandle`]: zng_var::ContextInitHandle
pub fn with_new_context_init_id(child: impl IntoUiNode) -> UiNode {
    let mut id = None;

    match_node(child, move |child, op| {
        let is_deinit = matches!(op, UiNodeOp::Deinit);
        id.get_or_insert_with(ContextInitHandle::new).with_context(|| child.op(op));

        if is_deinit {
            id = None;
        }
    })
}
