//! Widget nodes types, [`UiNode`], [`UiNodeList`] and others.

use std::{
    any::{Any, TypeId},
    fmt,
};

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
use zng_app_proc_macros::{ui_node, widget};
use zng_layout::unit::PxSize;
use zng_var::{ContextInitHandle, ResponseVar, Var};

use crate::{
    render::{FrameBuilder, FrameUpdate},
    update::{EventUpdate, WidgetUpdates},
};

use super::{
    WIDGET, WidgetId, WidgetUpdateMode,
    base::{PARALLEL_VAR, Parallel},
    info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
};

/// Represents an UI tree node.
///
/// You can use the [`match_node`] helper to quickly declare a new node from a closure, most property nodes are implemented
/// using the match helpers. For more advanced nodes you can use the [`ui_node`] proc-macro attribute.
///
/// [`match_node`]:fn@match_node
#[diagnostic::on_unimplemented(
    note = "you can use `match_node` to declare a node from a closure",
    note = "you can use `#[ui_node]` to implement `UiNode` for `{Self}`"
)]
pub trait UiNode: Any + Send {
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
    fn init(&mut self);

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
    fn deinit(&mut self);

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
    fn info(&mut self, info: &mut WidgetInfoBuilder);

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
    fn event(&mut self, update: &EventUpdate);

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
    fn update(&mut self, updates: &WidgetUpdates);

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
    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize;

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
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize;

    /// Generates render instructions and updates transforms and hit-test areas.
    ///
    /// This method does not generate pixels immediately, it generates *display items* that are visual building block instructions
    /// for the renderer that will run after the window *display list* is built.
    ///
    /// Only widgets and ancestors that requested render receive this call, other widgets reuse the display items and transforms
    /// from the last frame.
    fn render(&mut self, frame: &mut FrameBuilder);

    /// Updates values in the last generated frame.
    ///
    /// Some display item values and transforms can be updated directly, without needing to rebuild the display list. All [`FrameBuilder`]
    /// methods that accept a [`FrameValue<T>`] input can be bound to an ID that can be used to update that value.
    ///
    /// Only widgets and ancestors that requested render update receive this call. Note that if any other widget in the same window
    /// requests render all pending render update requests are upgraded to render requests.
    ///
    /// [`FrameValue<T>`]: crate::render::FrameValue
    fn render_update(&mut self, update: &mut FrameUpdate);

    /// Box this node or just returns `self` if it is already a `BoxedUiNode`.
    fn boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        debug_assert_ne!(self.type_id(), TypeId::of::<BoxedUiNode>());
        Box::new(self)
    }

    /// Helper for complying with the `"dyn_node"` feature, boxes the node or just returns it depending of the
    /// compile time feature.
    #[cfg(feature = "dyn_node")]
    fn cfg_boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        self.boxed()
    }

    /// Helper for complying with the `"dyn_node"` feature, boxes the node or just returns it depending of the
    /// compile time feature.
    #[cfg(not(feature = "dyn_node"))]
    fn cfg_boxed(self) -> Self
    where
        Self: Sized,
    {
        self
    }

    /// Gets if this node represents a full widget, that is, it is the outer-most widget node and defines a widget context.
    ///
    /// If this is `true` the [`with_context`] method can be used to get the widget context.
    ///
    /// [`with_context`]: UiNode::with_context
    fn is_widget(&self) -> bool {
        false
    }

    /// Gets if this node does nothing and is layout collapsed.
    ///
    /// Implementers must return `true` only if the node will always do nothing, nodes that may change
    /// and stop being collapsed are not nil.
    fn is_nil(&self) -> bool {
        false
    }

    /// Calls `f` with the [`WIDGET`] context of the node if it [`is_widget`].
    ///
    /// Returns `None` if the node does not represent a widget.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the widget in `f` will be copied to the
    /// caller widget context, otherwise they are ignored.
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        let _ = (update_mode, f);
        None
    }

    /// Gets a [`BoxedUiNode`] that is a full widget.
    ///
    /// If this node [`is_widget`] returns `self` boxed. Otherwise returns a new minimal widget
    /// that has `self` as a child node.
    ///
    /// Use this if you know that the widget is not a full widget or you don't mind that some
    /// nodes become full widgets only after init, otherwise use [`init_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    /// [`init_widget`]: UiNode::init_widget
    fn into_widget(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        if self.is_widget() {
            return self.boxed();
        }

        into_widget! {
            child = self;
        }
        .boxed()
    }

    /// Gets a [`BoxedUiNode`] that already is a full widget or will be after init and a response var that
    /// already is the widget ID or will update once after init with the ID.
    ///
    /// If this node [`is_widget`] returns `self` boxed and an already responded var. Otherwise returns
    /// a node that will ensure `self` is a full widget after init and update the response var with the
    /// widget ID.
    ///
    /// Some nodes become full widgets only after init, the [`ArcNode::take_on_init`] for example, this node
    /// supports these cases at the expense of having to reinit inside the generated widget when `self` is
    /// not a full widget even after init.
    ///
    /// [`is_widget`]: UiNode::is_widget
    /// [`ArcNode::take_on_init`]: crate::widget::node::ArcNode::take_on_init
    fn init_widget(mut self) -> (BoxedUiNode, ResponseVar<WidgetId>)
    where
        Self: Sized,
    {
        if let Some(id) = self.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
            return (self.boxed(), crate::var::response_done_var(id));
        }

        let (responder, response) = crate::var::response_var();
        let widget = match_widget(self.boxed(), move |c, op| {
            if let UiNodeOp::Init = op {
                c.init();
                let widget_id = if let Some(id) = c.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                    id
                } else {
                    c.deinit();
                    let not_widget = std::mem::replace(c.child(), NilUiNode.boxed());
                    *c.child() = not_widget.into_widget();

                    c.init();
                    c.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()).unwrap()
                };

                responder.respond(widget_id);
            }
        });
        (widget.boxed(), response)
    }

    /// Downcast to `T`, if `self` is `T` or `self` is a [`BoxedUiNode`] that is `T`.
    fn downcast_unbox<T: UiNode>(self) -> Result<T, BoxedUiNode>
    where
        Self: Sized,
    {
        let boxed = self.boxed();
        if boxed.actual_type_id() == TypeId::of::<T>() {
            Ok(*boxed.into_any_boxed().downcast().unwrap())
        } else if TypeId::of::<T>() == TypeId::of::<BoxedUiNode>() {
            Ok(*(Box::new(boxed) as Box<dyn Any>).downcast().unwrap())
        } else {
            Err(boxed)
        }
    }

    /// Returns the [`type_id`] of the unboxed node.
    ///
    /// [`type_id`]: Any::type_id
    fn actual_type_id(&self) -> TypeId {
        self.type_id()
    }

    /// Access to `dyn Any` methods.
    fn as_any(&self) -> &dyn Any
    where
        Self: Sized,
    {
        self
    }

    /// Access to mut `dyn Any` methods.
    fn as_any_mut(&mut self) -> &mut dyn Any
    where
        Self: Sized,
    {
        self
    }

    /// Wraps the node in a node that, before delegating each method, calls a closure with
    /// the [`UiNodeOpMethod`], the closure can return a *span* that is dropped after the method delegation.
    ///
    /// You can use the [`tracing`](https://docs.rs/tracing) crate to create the span.
    fn trace<E, S>(self, mut enter_mtd: E) -> BoxedUiNode
    where
        Self: Sized,
        E: FnMut(UiNodeOpMethod) -> S + Send + 'static,
    {
        match_node(self, move |node, op| {
            let _span = enter_mtd(op.mtd());
            node.op(op);
        })
        .boxed()
    }

    /// Runs the [ `UiNodeOp`].
    fn op(&mut self, op: UiNodeOp) {
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
}

/// See [`UiNode::into_widget`]
#[expect(non_camel_case_types)]
#[widget($crate::widget::node::into_widget)]
struct into_widget(crate::widget::base::WidgetBase);
#[zng_app_proc_macros::property(CHILD, capture, widget_impl(into_widget))]
fn child(child: impl UiNode) {}
impl into_widget {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|b| {
            let child = b.capture_ui_node(crate::property_id!(Self::child)).unwrap();
            b.set_child(child);
        });
    }
}

/// Represents a list of UI nodes.
///
/// There are multiple node list types, panel implementers receive children as `impl UiNodeList` and usually wrap it in a [`PanelList`].
///
/// UI node lists delegate the [`UiNode`] method for each node in the list, potentially in parallel. Panel implementers call the `*_all`
/// methods to delegate, they can also use the [`match_node_list`] to implement delegation. The trait also offers [`for_each`], [`par_each`]
/// and other methods for direct access to the nodes, both sequentially and in parallel.
///
/// Note that trying to access the nodes before init will probably not work, the [`ArcNodeList`] type is used by properties that request
/// `impl UiNodeList` input, so captured property lists will always be empty before init.
///
/// [`for_each`]: UiNodeList::for_each
/// [`par_each`]: UiNodeList::par_each
pub trait UiNodeList: UiNodeListBoxed {
    /// Visit the specific node.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R;

    /// Calls `f` for each node in the list with the index, sequentially.
    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode);

    /// Calls `f` for each node in the list with the index, in parallel.
    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync;

    /// Calls `fold` for each node in the list in parallel, with fold accumulators produced by `identity`, then merges the folded results
    /// using `reduce` to produce the final value also in parallel.
    ///
    /// If `reduce` is [associative] the order is preserved in the result.
    ///
    /// # Example
    ///
    /// This example will collect the node indexes in order:
    ///
    /// ```
    /// # use zng_app::widget::node::UiNodeList;
    /// # fn demo(mut list: impl UiNodeList) -> Vec<usize> {
    /// list.par_fold_reduce(
    ///     Vec::new,
    ///     |mut v, i, _| {
    ///         v.push(i);
    ///         v
    ///     },
    ///     |mut a, b| {
    ///         a.extend(b);
    ///         a
    ///     },
    /// )
    /// # }
    /// ```
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync;

    /// Gets the current number of nodes in the list.
    fn len(&self) -> usize;

    /// Returns `true` if the list does not contain any nodes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets `self` boxed, or itself if it is already boxed.
    fn boxed(self) -> BoxedUiNodeList;

    /// Move all nodes into `vec`.
    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>);

    /// Init the list in a context, all nodes are also inited.
    ///
    /// The behavior of some list implementations depend on this call, manually initializing nodes is an error.
    fn init_all(&mut self) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::INIT) {
            self.par_each(|_, c| {
                c.init();
            });
        } else {
            self.for_each(|_, c| {
                c.init();
            })
        }
    }

    /// Deinit the list in a context, all nodes are also deinited.
    ///
    /// The behavior of some list implementations depend on this call, manually deiniting nodes is an error.
    fn deinit_all(&mut self) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::DEINIT) {
            self.par_each(|_, c| {
                c.deinit();
            });
        } else {
            self.for_each(|_, c| {
                c.deinit();
            });
        }
    }

    /// Rebuilds the list in a context, all node info is rebuilt.
    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::INFO) {
            let p_info = self.par_fold_reduce(
                || info.parallel_split(),
                |mut info, _, node| {
                    node.info(&mut info);
                    info
                },
                |mut a, b| {
                    a.parallel_fold(b);
                    a
                },
            );
            info.parallel_fold(p_info);
        } else {
            self.for_each(|_, c| {
                c.info(info);
            });
        }
    }

    /// Receive updates for the list in a context, all nodes are also updated.
    ///
    /// The behavior of some list implementations depend on this call, manually updating nodes is an error.
    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let _ = observer;

        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::UPDATE) {
            self.par_each(|_, c| {
                c.update(updates);
            });
        } else {
            self.for_each(|_, c| {
                c.update(updates);
            });
        }
    }

    /// Receive an event for the list in a context, all nodes are also notified.
    ///
    /// The behavior of some list implementations depend on this call, manually notifying nodes is an error.
    fn event_all(&mut self, update: &EventUpdate) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::EVENT) {
            self.par_each(|_, c| {
                c.event(update);
            });
        } else {
            self.for_each(|_, c| {
                c.event(update);
            });
        }
    }

    /// Call `measure` for each node and combines the final size using `fold_size`.
    ///
    /// The call to `measure` can be parallel if [`Parallel::LAYOUT`] is enabled.
    #[must_use]
    fn measure_each<F, S>(&mut self, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
        Self: Sized,
    {
        default_measure_each(self, wm, measure, fold_size)
    }

    /// Call `layout` for each node and combines the final size using `fold_size`.
    ///
    /// The call to `layout` can be parallel if [`Parallel::LAYOUT`] is enabled.
    #[must_use]
    fn layout_each<F, S>(&mut self, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
        Self: Sized,
    {
        default_layout_each(self, wl, layout, fold_size)
    }

    /// Render all nodes.
    ///
    /// The correct behavior of some list implementations depend on this call, using [`for_each`] to render nodes can
    /// break it.
    ///
    /// [`for_each`]: UiNodeList::for_each
    fn render_all(&mut self, frame: &mut FrameBuilder) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let p_frame = self.par_fold_reduce(
                || frame.parallel_split(),
                |mut frame, _, node| {
                    node.render(&mut frame);
                    frame
                },
                |mut a, b| {
                    a.parallel_fold(b);
                    a
                },
            );
            frame.parallel_fold(p_frame);
        } else {
            self.for_each(|_, c| {
                c.render(frame);
            })
        }
    }

    /// Render all nodes.
    ///
    /// The correct behavior of some list implementations depend on this call, using [`for_each`] to render nodes can
    /// break it.
    ///
    /// [`for_each`]: UiNodeList::for_each
    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
            let p_update = self.par_fold_reduce(
                || update.parallel_split(),
                |mut update, _, node| {
                    node.render_update(&mut update);
                    update
                },
                |mut a, b| {
                    a.parallel_fold(b);
                    a
                },
            );
            update.parallel_fold(p_update);
        } else {
            self.for_each(|_, c| {
                c.render_update(update);
            })
        }
    }
    /// Downcast to `L`, if `self` is `L` or is a [`BoxedUiNodeList`] that is `L`.
    fn downcast_unbox<L: UiNodeList>(self) -> Result<L, BoxedUiNodeList>
    where
        Self: Sized,
    {
        let boxed = self.boxed();
        if boxed.actual_type_id() == TypeId::of::<L>() {
            Ok(*boxed.into_any_boxed().downcast().unwrap())
        } else if TypeId::of::<L>() == TypeId::of::<BoxedUiNodeList>() {
            Ok(*(Box::new(boxed) as Box<dyn Any>).downcast().unwrap())
        } else {
            Err(boxed)
        }
    }

    /// Returns the [`type_id`] of the unboxed list.
    ///
    /// [`type_id`]: Any::type_id
    fn actual_type_id(&self) -> TypeId {
        self.type_id()
    }

    /// Access to mut `dyn Any` methods.
    fn as_any(&mut self) -> &mut dyn Any
    where
        Self: Sized,
    {
        self
    }

    /// Runs the [`UiNodeOp`].
    fn op(&mut self, op: UiNodeOp)
    where
        Self: Sized,
    {
        match op {
            UiNodeOp::Init => ui_node_list_default::init_all(self),
            UiNodeOp::Deinit => ui_node_list_default::deinit_all(self),
            UiNodeOp::Info { info } => ui_node_list_default::info_all(self, info),
            UiNodeOp::Event { update } => ui_node_list_default::event_all(self, update),
            UiNodeOp::Update { updates } => ui_node_list_default::update_all(self, updates),
            UiNodeOp::Measure { wm, desired_size } => *desired_size = ui_node_list_default::measure_all(self, wm),
            UiNodeOp::Layout { wl, final_size } => *final_size = ui_node_list_default::layout_all(self, wl),
            UiNodeOp::Render { frame } => ui_node_list_default::render_all(self, frame),
            UiNodeOp::RenderUpdate { update } => ui_node_list_default::render_update_all(self, update),
        }
    }
}

fn default_measure_each<F, S>(self_: &mut impl UiNodeList, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
where
    F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
    S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
{
    #[cfg(feature = "dyn_closure")]
    let measure: Box<dyn Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync> = Box::new(measure);
    #[cfg(feature = "dyn_closure")]
    let fold_size: Box<dyn Fn(PxSize, PxSize) -> PxSize + Send + Sync> = Box::new(fold_size);

    default_measure_each_impl(self_, wm, measure, fold_size)
}
fn default_measure_each_impl<F, S>(self_: &mut impl UiNodeList, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
where
    F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
    S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
{
    if self_.len() > 1 && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
        // fold a tuple of `(wm, size)`
        let (pwm, size) = self_.par_fold_reduce(
            || (wm.parallel_split(), PxSize::zero()),
            |(mut a_wm, a_size), i, n| {
                let b_size = measure(i, n, &mut a_wm);
                (a_wm, fold_size(a_size, b_size))
            },
            |(mut awm, a_size), (bwm, b_size)| {
                (
                    {
                        awm.parallel_fold(bwm);
                        awm
                    },
                    fold_size(a_size, b_size),
                )
            },
        );
        wm.parallel_fold(pwm);
        size
    } else {
        let mut size = PxSize::zero();
        self_.for_each(|i, n| {
            let b = measure(i, n, wm);
            size = fold_size(size, b);
        });
        size
    }
}

fn default_layout_each<F, S>(self_: &mut impl UiNodeList, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
where
    F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
    S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
{
    #[cfg(feature = "dyn_closure")]
    let layout: Box<dyn Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync> = Box::new(layout);
    #[cfg(feature = "dyn_closure")]
    let fold_size: Box<dyn Fn(PxSize, PxSize) -> PxSize + Send + Sync> = Box::new(fold_size);

    default_layout_each_impl(self_, wl, layout, fold_size)
}
fn default_layout_each_impl<F, S>(self_: &mut impl UiNodeList, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
where
    F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
    S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
{
    if self_.len() > 1 && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
        // fold a tuple of `(wl, size)`
        let (pwl, size) = self_.par_fold_reduce(
            || (wl.parallel_split(), PxSize::zero()),
            |(mut awl, a_size), i, n| {
                let b_size = layout(i, n, &mut awl);
                (awl, fold_size(a_size, b_size))
            },
            |(mut awl, a_size), (bwl, b_size)| {
                (
                    {
                        awl.parallel_fold(bwl);
                        awl
                    },
                    fold_size(a_size, b_size),
                )
            },
        );
        wl.parallel_fold(pwl);
        size
    } else {
        let mut size = PxSize::zero();
        self_.for_each(|i, n| {
            let b = layout(i, n, wl);
            size = fold_size(size, b);
        });
        size
    }
}

#[doc(hidden)]
pub mod ui_node_list_default {
    use super::*;

    pub fn init_all(list: &mut impl UiNodeList) {
        list.init_all();
    }

    pub fn deinit_all(list: &mut impl UiNodeList) {
        list.deinit_all();
    }

    pub fn info_all(list: &mut impl UiNodeList, info: &mut WidgetInfoBuilder) {
        list.info_all(info)
    }

    pub fn event_all(list: &mut impl UiNodeList, update: &EventUpdate) {
        list.event_all(update);
    }

    pub fn update_all(list: &mut impl UiNodeList, updates: &WidgetUpdates) {
        let mut changed = false;

        list.update_all(updates, &mut changed);

        if changed {
            WIDGET.layout().render();
        }
    }

    pub fn measure_all(list: &mut impl UiNodeList, wm: &mut WidgetMeasure) -> PxSize {
        list.measure_each(wm, |_, n, wm| n.measure(wm), PxSize::max)
    }

    pub fn layout_all(list: &mut impl UiNodeList, wl: &mut WidgetLayout) -> PxSize {
        list.layout_each(wl, |_, n, wl| n.layout(wl), PxSize::max)
    }

    pub fn render_all(list: &mut impl UiNodeList, frame: &mut FrameBuilder) {
        list.render_all(frame);
    }

    pub fn render_update_all(list: &mut impl UiNodeList, update: &mut FrameUpdate) {
        list.render_update_all(update)
    }
}

#[doc(hidden)]
pub trait UiNodeBoxed: Any + Send {
    fn info_boxed(&mut self, info: &mut WidgetInfoBuilder);
    fn init_boxed(&mut self);
    fn deinit_boxed(&mut self);
    fn update_boxed(&mut self, updates: &WidgetUpdates);
    fn event_boxed(&mut self, update: &EventUpdate);
    fn measure_boxed(&mut self, wm: &mut WidgetMeasure) -> PxSize;
    fn layout_boxed(&mut self, wl: &mut WidgetLayout) -> PxSize;
    fn render_boxed(&mut self, frame: &mut FrameBuilder);
    fn render_update_boxed(&mut self, update: &mut FrameUpdate);

    fn is_widget_boxed(&self) -> bool;
    fn is_nil_boxed(&self) -> bool;
    fn with_context_boxed(&mut self, update_mode: WidgetUpdateMode, f: &mut dyn FnMut());
    fn into_widget_boxed(self: Box<Self>) -> BoxedUiNode;
    fn as_any_boxed(&self) -> &dyn Any;
    fn as_any_mut_boxed(&mut self) -> &mut dyn Any;

    fn actual_type_id_boxed(&self) -> TypeId;
    fn into_any_boxed(self: Box<Self>) -> Box<dyn Any>;
}

impl<U: UiNode> UiNodeBoxed for U {
    fn info_boxed(&mut self, info: &mut WidgetInfoBuilder) {
        self.info(info);
    }

    fn init_boxed(&mut self) {
        self.init();
    }

    fn deinit_boxed(&mut self) {
        self.deinit();
    }

    fn update_boxed(&mut self, updates: &WidgetUpdates) {
        self.update(updates);
    }

    fn event_boxed(&mut self, update: &EventUpdate) {
        self.event(update);
    }

    fn measure_boxed(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.measure(wm)
    }

    fn layout_boxed(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.layout(wl)
    }

    fn render_boxed(&mut self, frame: &mut FrameBuilder) {
        self.render(frame);
    }

    fn render_update_boxed(&mut self, update: &mut FrameUpdate) {
        self.render_update(update);
    }

    fn is_widget_boxed(&self) -> bool {
        self.is_widget()
    }

    fn is_nil_boxed(&self) -> bool {
        self.is_nil()
    }

    fn into_widget_boxed(self: Box<Self>) -> BoxedUiNode {
        self.into_widget()
    }

    fn actual_type_id_boxed(&self) -> TypeId {
        self.type_id()
    }

    fn into_any_boxed(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn with_context_boxed(&mut self, update_mode: WidgetUpdateMode, f: &mut dyn FnMut()) {
        self.with_context(update_mode, f);
    }

    fn as_any_boxed(&self) -> &dyn Any {
        self.as_any()
    }

    fn as_any_mut_boxed(&mut self) -> &mut dyn Any {
        self.as_any_mut()
    }
}

#[doc(hidden)]
pub trait UiNodeListBoxed: Any + Send {
    fn with_node_boxed(&mut self, index: usize, f: &mut dyn FnMut(&mut BoxedUiNode));
    fn for_each_boxed(&mut self, f: &mut dyn FnMut(usize, &mut BoxedUiNode));
    fn par_each_boxed(&mut self, f: &(dyn Fn(usize, &mut BoxedUiNode) + Send + Sync));
    fn par_fold_reduce_boxed(
        &mut self,
        identity: &(dyn Fn() -> Box<dyn Any + Send> + Send + Sync),
        fold: &(dyn Fn(Box<dyn Any + Send>, usize, &mut BoxedUiNode) -> Box<dyn Any + Send> + Send + Sync),
        reduce: &(dyn Fn(Box<dyn Any + Send>, Box<dyn Any + Send>) -> Box<dyn Any + Send> + Send + Sync),
    ) -> Box<dyn Any + Send>;
    fn measure_each_boxed(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Send + Sync),
    ) -> PxSize;
    fn layout_each_boxed(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Send + Sync),
    ) -> PxSize;
    fn len_boxed(&self) -> usize;
    fn drain_into_boxed(&mut self, vec: &mut Vec<BoxedUiNode>);
    fn init_all_boxed(&mut self);
    fn deinit_all_boxed(&mut self);
    fn info_all_boxed(&mut self, info: &mut WidgetInfoBuilder);
    fn event_all_boxed(&mut self, update: &EventUpdate);
    fn update_all_boxed(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver);
    fn render_all_boxed(&mut self, frame: &mut FrameBuilder);
    fn render_update_all_boxed(&mut self, update: &mut FrameUpdate);
    fn actual_type_id_boxed(&self) -> TypeId;
    fn into_any_boxed(self: Box<Self>) -> Box<dyn Any>;
    fn as_any_boxed(&mut self) -> &mut dyn Any;
}
impl<L: UiNodeList> UiNodeListBoxed for L {
    fn with_node_boxed(&mut self, index: usize, f: &mut dyn FnMut(&mut BoxedUiNode)) {
        self.with_node(index, f)
    }

    fn for_each_boxed(&mut self, f: &mut dyn FnMut(usize, &mut BoxedUiNode)) {
        self.for_each(f);
    }

    fn par_each_boxed(&mut self, f: &(dyn Fn(usize, &mut BoxedUiNode) + Send + Sync)) {
        self.par_each(f)
    }

    fn par_fold_reduce_boxed(
        &mut self,
        identity: &(dyn Fn() -> Box<dyn Any + Send> + Send + Sync),
        fold: &(dyn Fn(Box<dyn Any + Send>, usize, &mut BoxedUiNode) -> Box<dyn Any + Send> + Send + Sync),
        reduce: &(dyn Fn(Box<dyn Any + Send>, Box<dyn Any + Send>) -> Box<dyn Any + Send> + Send + Sync),
    ) -> Box<dyn Any + Send> {
        self.par_fold_reduce(identity, fold, reduce)
    }

    fn measure_each_boxed(
        &mut self,
        wm: &mut WidgetMeasure,
        measure: &(dyn Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Send + Sync),
    ) -> PxSize {
        self.measure_each(wm, measure, fold_size)
    }

    fn layout_each_boxed(
        &mut self,
        wl: &mut WidgetLayout,
        layout: &(dyn Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync),
        fold_size: &(dyn Fn(PxSize, PxSize) -> PxSize + Send + Sync),
    ) -> PxSize {
        self.layout_each(wl, layout, fold_size)
    }

    fn len_boxed(&self) -> usize {
        self.len()
    }

    fn drain_into_boxed(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.drain_into(vec)
    }

    fn init_all_boxed(&mut self) {
        self.init_all();
    }

    fn deinit_all_boxed(&mut self) {
        self.deinit_all();
    }

    fn info_all_boxed(&mut self, info: &mut WidgetInfoBuilder) {
        self.info_all(info);
    }

    fn event_all_boxed(&mut self, update: &EventUpdate) {
        self.event_all(update);
    }

    fn update_all_boxed(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.update_all(updates, observer);
    }

    fn render_all_boxed(&mut self, frame: &mut FrameBuilder) {
        self.render_all(frame);
    }

    fn render_update_all_boxed(&mut self, update: &mut FrameUpdate) {
        self.render_update_all(update);
    }

    fn actual_type_id_boxed(&self) -> TypeId {
        self.type_id()
    }

    fn into_any_boxed(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any_boxed(&mut self) -> &mut dyn Any {
        self.as_any()
    }
}

/// An [`UiNode`] in a box.
pub type BoxedUiNode = Box<dyn UiNodeBoxed>;

/// An [`UiNodeList`] in a box.
pub type BoxedUiNodeList = Box<dyn UiNodeListBoxed>;

impl UiNode for BoxedUiNode {
    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        self.as_mut().info_boxed(info);
    }

    fn init(&mut self) {
        self.as_mut().init_boxed();
    }

    fn deinit(&mut self) {
        self.as_mut().deinit_boxed();
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        self.as_mut().update_boxed(updates);
    }

    fn event(&mut self, update: &EventUpdate) {
        self.as_mut().event_boxed(update);
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        self.as_mut().measure_boxed(wm)
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        self.as_mut().layout_boxed(wl)
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        self.as_mut().render_boxed(frame);
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        self.as_mut().render_update_boxed(update);
    }

    fn boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        self
    }

    fn actual_type_id(&self) -> TypeId {
        self.as_ref().actual_type_id_boxed()
    }

    fn is_widget(&self) -> bool {
        self.as_ref().is_widget_boxed()
    }

    fn is_nil(&self) -> bool {
        self.as_ref().is_nil_boxed()
    }

    fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_mut()
            .with_context_boxed(update_mode, &mut || r = Some((f.take().unwrap())()));
        r
    }

    fn into_widget(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        self.into_widget_boxed()
    }

    fn as_any(&self) -> &dyn Any
    where
        Self: Sized,
    {
        self.as_ref().as_any_boxed()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any
    where
        Self: Sized,
    {
        self.as_mut().as_any_mut_boxed()
    }
}

impl UiNodeList for BoxedUiNodeList {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_mut().with_node_boxed(index, &mut |n| r = Some((f.take().unwrap())(n)));
        r.unwrap()
    }

    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.as_mut().for_each_boxed(&mut f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.as_mut().par_each_boxed(&f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.as_mut()
            .par_fold_reduce_boxed(
                &move || Box::new(Some(identity())),
                &move |mut r, i, n| {
                    let r_mut = r.downcast_mut::<Option<T>>().unwrap();
                    *r_mut = Some(fold(r_mut.take().unwrap(), i, n));
                    r
                },
                &|mut a, b| {
                    let a_mut = a.downcast_mut::<Option<T>>().unwrap();
                    *a_mut = Some(reduce(a_mut.take().unwrap(), b.downcast::<Option<T>>().unwrap().unwrap()));
                    a
                },
            )
            .downcast::<Option<T>>()
            .unwrap()
            .unwrap()
    }

    fn len(&self) -> usize {
        self.as_ref().len_boxed()
    }

    fn boxed(self) -> BoxedUiNodeList {
        self
    }

    fn actual_type_id(&self) -> TypeId {
        self.as_ref().actual_type_id_boxed()
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.as_mut().drain_into_boxed(vec)
    }

    fn init_all(&mut self) {
        self.as_mut().init_all_boxed();
    }

    fn deinit_all(&mut self) {
        self.as_mut().deinit_all_boxed();
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.as_mut().update_all_boxed(updates, observer);
    }

    fn info_all(&mut self, info: &mut WidgetInfoBuilder) {
        self.as_mut().info_all_boxed(info);
    }

    fn event_all(&mut self, update: &EventUpdate) {
        self.as_mut().event_all_boxed(update);
    }

    fn render_all(&mut self, frame: &mut FrameBuilder) {
        self.as_mut().render_all_boxed(frame);
    }

    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        self.as_mut().render_update_all_boxed(update);
    }

    fn measure_each<F, S>(&mut self, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        self.as_mut().measure_each_boxed(wm, &measure, &fold_size)
    }

    fn layout_each<F, S>(&mut self, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        self.as_mut().layout_each_boxed(wl, &layout, &fold_size)
    }

    fn as_any(&mut self) -> &mut dyn Any
    where
        Self: Sized,
    {
        self.as_mut().as_any_boxed()
    }
}

impl<U: UiNode> UiNode for Option<U> {
    fn info(&mut self, info: &mut WidgetInfoBuilder) {
        if let Some(node) = self {
            node.info(info);
        }
    }

    fn init(&mut self) {
        if let Some(node) = self {
            node.init();
        }
    }

    fn deinit(&mut self) {
        if let Some(node) = self {
            node.deinit();
        }
    }

    fn event(&mut self, update: &EventUpdate) {
        if let Some(node) = self {
            node.event(update);
        }
    }

    fn update(&mut self, updates: &WidgetUpdates) {
        if let Some(node) = self {
            node.update(updates);
        }
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        if let Some(node) = self { node.measure(wm) } else { PxSize::zero() }
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if let Some(node) = self { node.layout(wl) } else { PxSize::zero() }
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        if let Some(node) = self {
            node.render(frame);
        }
    }

    fn render_update(&mut self, update: &mut FrameUpdate) {
        if let Some(node) = self {
            node.render_update(update);
        }
    }

    fn boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        match self {
            Some(node) => node.boxed(),
            None => NilUiNode.boxed(),
        }
    }

    fn is_widget(&self) -> bool {
        match self {
            Some(node) => node.is_widget(),
            None => false,
        }
    }

    fn is_nil(&self) -> bool {
        self.is_none()
    }

    fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        match self {
            Some(node) => node.with_context(update_mode, f),
            None => None,
        }
    }

    fn into_widget(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        match self {
            Some(node) => node.into_widget(),
            None => NilUiNode.into_widget(),
        }
    }
}

impl UiNodeList for Option<BoxedUiNode> {
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        match self {
            Some(node) => {
                assert_bounds(1, index);
                f(node)
            }
            None => {
                assert_bounds(0, index);
                unreachable!()
            }
        }
    }

    fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        if let Some(node) = self {
            f(0, node);
        }
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        if let Some(node) = self {
            f(0, node);
        }
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, _: R) -> T
    where
        T: Send,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        if let Some(node) = self {
            fold(identity(), 0, node)
        } else {
            identity()
        }
    }

    fn len(&self) -> usize {
        match self {
            Some(_) => 1,
            None => 0,
        }
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        if let Some(n) = self.take() {
            vec.push(n);
        }
    }
}

fn assert_bounds(len: usize, i: usize) {
    if i >= len {
        panic!("index `{i}` is >= len `{len}`")
    }
}

/// A UI node that does nothing and has collapsed layout (zero size).
pub struct NilUiNode;
#[super::ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        PxSize::zero()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        PxSize::zero()
    }

    fn is_nil(&self) -> bool {
        true
    }
}

/// A UI node that fills the available layout space.
///
/// The space is blank, the node does nothing other then layout to fill.
pub struct FillUiNode;
#[ui_node(none)]
impl UiNode for FillUiNode {}

/// Wraps `child` in a node that provides a unique [`ContextInitHandle`], refreshed every (re)init.
///
/// [`ContextInitHandle`]: zng_var::ContextInitHandle
pub fn with_new_context_init_id(child: impl UiNode) -> impl UiNode {
    let mut id = None;

    match_node(child, move |child, op| {
        let is_deinit = matches!(op, UiNodeOp::Deinit);
        id.get_or_insert_with(ContextInitHandle::new).with_context(|| child.op(op));

        if is_deinit {
            id = None;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn downcast_unbox() {
        fn node() -> impl UiNode {
            NilUiNode
        }

        assert!(node().downcast_unbox::<NilUiNode>().is_ok())
    }

    #[test]
    pub fn downcast_unbox_boxed() {
        fn node() -> BoxedUiNode {
            NilUiNode.boxed()
        }

        assert!(node().downcast_unbox::<NilUiNode>().is_ok())
    }

    #[test]
    pub fn downcast_unbox_to_boxed() {
        fn node() -> impl UiNode {
            NilUiNode.boxed()
        }

        assert!(node().downcast_unbox::<BoxedUiNode>().is_ok())
    }

    #[test]
    pub fn downcast_unbox_widget() {
        fn node() -> BoxedUiNode {
            NilUiNode.into_widget()
        }

        assert!(node().downcast_unbox::<BoxedUiNode>().is_ok())
    }
}
