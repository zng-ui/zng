//! Widget instance types, [`UiNode`], [`UiNodeList`] and others.

use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt,
};

use parking_lot::Mutex;

use crate::units::*;
use crate::{
    context::*,
    event::EventUpdate,
    var::{impl_from_and_into_var, Var},
    widget_base::{Parallel, PARALLEL_VAR},
    widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
};
use crate::{
    render::{FrameBuilder, FrameUpdate},
    text::Txt,
    ui_node,
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

pub use crate::crate_util::{unique_id_64, IdNameError};

unique_id_64! {
    /// Unique id of a widget.
    ///
    /// # Name
    ///
    /// Widget ids are very fast but are just a number that is only unique for the same process that generated then.
    /// You can associate a [`name`] with an id to give it a persistent identifier.
    ///
    /// [`name`]: WidgetId::name
    pub struct WidgetId;
}
zero_ui_unique_id::impl_unique_id_name!(WidgetId);
zero_ui_unique_id::impl_unique_id_fmt!(WidgetId);

impl_from_and_into_var! {
    /// Calls [`WidgetId::named`].
    fn from(name: &'static str) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: String) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: Cow<'static, str>) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: char) -> WidgetId {
        WidgetId::named(name)
    }
    /// Calls [`WidgetId::named`].
    fn from(name: Txt) -> WidgetId {
        WidgetId::named(name)
    }
    fn from(id: WidgetId) -> zero_ui_view_api::access::AccessNodeId {
        zero_ui_view_api::access::AccessNodeId(id.get())
    }
}
impl_from_and_into_var! {
    fn from(some: WidgetId) -> Option<WidgetId>;
}

impl fmt::Debug for StaticWidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}
impl crate::var::IntoValue<WidgetId> for &'static StaticWidgetId {}
impl serde::Serialize for WidgetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unammed `WidgetId`"));
        }
        name.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for WidgetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = Txt::deserialize(deserializer)?;
        Ok(WidgetId::named(name))
    }
}

/// An Ui tree node.
pub trait UiNode: Any + Send {
    /// Called every time the node is plugged into the UI tree.
    ///
    /// If the node [`is_widget`] an info, layout and render request must be made, other nodes need only to init
    /// descendants.
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn init(&mut self);

    /// Called every time the node is unplugged from the UI tree.
    ///
    /// If the node [`is_widget`] an info, layout and render request must be made, other nodes need only to deinit
    /// descendants.
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn deinit(&mut self);

    /// Called every time there are structural changes in the UI tree such as a node added or removed.
    ///
    /// Note that info rebuild has higher priority over event, update, layout and render, this means that
    /// [`WIDGET.info`] is always available in those methods, but it also means that if you set a variable
    /// and request info update the next info rebuild will still observe the old variable value, you can
    /// work around this issue by only requesting info rebuild after the variable updates.
    ///
    /// [`WIDGET.info`]: crate::context::WIDGET::info
    fn info(&mut self, info: &mut WidgetInfoBuilder);

    /// Called every time an event updates.
    ///
    /// Every call to this method is for a single update of a single event type, you can listen to events
    /// using the [`Event::on`] method or other methods of the [`Event`] type.
    ///
    /// [`Event::on`]: crate::event::Event::on
    /// [`Event`]: crate::event::Event
    fn event(&mut self, update: &EventUpdate);

    /// Called every time an update is requested.
    ///
    /// An update can be requested using the context [`WIDGET`], after each request, they also happen
    /// when variables update and any other context or service structure that can be observed updates.
    fn update(&mut self, updates: &WidgetUpdates);

    /// Compute the widget size given the contextual layout metrics.
    ///
    /// Implementers must return the same size [`layout`] returns for the given [`LayoutMetrics`], without
    /// affecting the actual widget render.
    ///
    /// # Arguments
    ///
    /// * `ctx`: Limited layout context access.
    /// * `wm`: Limited version of [`WidgetLayout`], includes inline layout, but not offsets.
    ///
    /// # Returns
    ///
    /// Returns the computed node size, this will probably influencing the actual constraints that will be used
    /// on a subsequent [`layout`] call.
    ///
    /// [`layout`]: Self::layout
    #[must_use]
    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize;

    /// Called every time a layout update is requested or the constraints used have changed.
    ///
    /// Implementers must try to fit their size inside the [`constraints`] as best as it can and return an accurate final size. If
    /// the size breaks the constraints the widget may end-up clipped.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Limited layout context, allows causing updates, but no access to services or events, also provides access to the [`LayoutMetrics`].
    /// * `wl` - Layout state, helps coordinate the final transforms of the widget outer and inner bounds, border widths and corner
    /// radius.
    ///
    /// # Returns
    ///
    /// Returns the computed node size, this will end-up influencing the size of the widget inner or outer bounds.
    ///
    /// [`constraints`]: LayoutMetrics::constraints
    #[must_use]
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize;

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    ///
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&mut self, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    ///
    /// * `update`: Contains the frame value updates.
    fn render_update(&mut self, update: &mut FrameUpdate);

    /// Box this node, unless it is already `BoxedUiNode`.
    fn boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        debug_assert_ne!(self.type_id(), TypeId::of::<BoxedUiNode>());
        Box::new(self)
    }

    /// Helper for complying with the `dyn_node` feature, boxes the node or just returns it depending of the
    /// compile time feature.
    ///
    /// If the `dyn_node` feature is enabled nodes should be nested using [`BoxedUiNode`] instead of
    /// generating a new type. The `#[property(..)]` attribute macro auto-implements this for property functions,
    /// other functions in the format `fn(impl UiNode, ..) -> impl UiNode` can use this method to achieve the same.
    #[cfg(dyn_node)]
    fn cfg_boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        self.boxed()
    }

    /// Helper for complying with the `dyn_node` feature, boxes the node or just returns it depending of the
    /// compile time feature.
    ///
    /// If the `dyn_node` feature is enabled nodes should be nested using [`BoxedUiNode`] instead of
    /// generating a new type. The `#[property(..)]` attribute macro auto-implements this for property functions,
    /// other functions in the format `fn(impl UiNode, ..) -> impl UiNode` can use this method to achieve the same.
    #[cfg(not(dyn_node))]
    fn cfg_boxed(self) -> Self
    where
        Self: Sized,
    {
        self
    }

    /// Gets if this node represents a full widget.
    ///
    /// If this is `true` the [`with_context`] method can be used to get the widget state.
    ///
    /// [`with_context`]: UiNode::with_context
    fn is_widget(&self) -> bool {
        false
    }

    /// Calls `f` with the [`WIDGET`] context of the node if it [`is_widget`].
    ///
    /// Returns `None` if the node does not represent an widget.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the `ctx` after `f` will be copied to the
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
            crate::widget_base::child = self;
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
    /// [`ArcNode::take_on_init`]: crate::widget_instance::ArcNode::take_on_init
    fn init_widget(mut self) -> (BoxedUiNode, crate::var::ResponseVar<WidgetId>)
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
    /// You can use  the [`tracing`](https://docs.rs/tracing) crate to create the span.
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
#[allow(non_camel_case_types)]
#[crate::widget($crate::widget_instance::into_widget)]
struct into_widget(crate::widget_base::WidgetBase);
impl into_widget {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|b| {
            let child = b.capture_ui_node(crate::property_id!(crate::widget_base::child)).unwrap();
            b.set_child(child);
        });
    }
}

/// Represents a list of [`UiNode`] instances.
///
/// Panel implementers must delegate every [`UiNode`] method to every node in the children list, in particular the
/// [`init_all`], [`deinit_all`] and [`update_all`] methods must be used to support reactive lists, and the [`render_all`]
/// and [`render_update_all`] must be used to render, to support the [`PanelList`]. Other [`UiNode`] methods must
/// be delegated using [`for_each`] or [`par_each`]. The [`#[ui_node(children)]`] attribute macro auto-generates
/// delegations for each method.
///
/// Note that node lists can be [`ArcNodeList`] that is always empty before init, and captured properties always use this
/// type to share the list, so attempting to access the items before the first call to [`init_all`] will not work.
///
/// [`init_all`]: UiNodeList::init_all
/// [`deinit_all`]: UiNodeList::deinit_all
/// [`update_all`]: UiNodeList::update_all
/// [`render_all`]: UiNodeList::render_all
/// [`render_update_all`]: UiNodeList::render_update_all
/// [`for_each`]: UiNodeList::for_each
/// [`par_each`]: UiNodeList::par_each
pub trait UiNodeList: UiNodeListBoxed {
    /// Visit the specific node, panic if `index` is out of bounds.
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R;

    /// Calls `f` for each node in the list with the index.
    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode);

    /// Calls `f` for each node in the list with the index in parallel.
    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync;

    /// Calls `fold` for each node in the list in parallel, with fold accumulators produced by `identity`, then merges the folded results
    /// using `reduce` to produce the final value also in parallel.
    ///
    /// If `reduce` is [associative] the order is preserved in the result, this example will collect the node indexes in order:
    ///
    /// ```
    /// # use zero_ui_core::widget_instance::UiNodeList;
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
    ///     })
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

    /// Returns `true` if the list does not contain any node.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Boxed the list, does not double box.
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

    /// Rebuilds the list in a context, all node info is rebuild.
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
    /// break it, for example, the [`PanelList`] render nodes in a different order.
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
    /// break it, for example, the [`PanelList`] render nodes in a different order.
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
    /// Downcast to `L`, if `self` is `L` or `self` is a [`BoxedUiNodeList`] that is `L`.
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
    #[cfg(dyn_closure)]
    let measure: Box<dyn Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync> = Box::new(measure);
    #[cfg(dyn_closure)]
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
            |(mut awm, asize), i, n| {
                let bsize = measure(i, n, &mut awm);
                (awm, fold_size(asize, bsize))
            },
            |(mut awm, asize), (bwm, bsize)| {
                (
                    {
                        awm.parallel_fold(bwm);
                        awm
                    },
                    fold_size(asize, bsize),
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
    #[cfg(dyn_closure)]
    let layout: Box<dyn Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync> = Box::new(layout);
    #[cfg(dyn_closure)]
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
            |(mut awl, asize), i, n| {
                let bsize = layout(i, n, &mut awl);
                (awl, fold_size(asize, bsize))
            },
            |(mut awl, asize), (bwl, bsize)| {
                (
                    {
                        awl.parallel_fold(bwl);
                        awl
                    },
                    fold_size(asize, bsize),
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
        if let Some(node) = self {
            node.measure(wm)
        } else {
            PxSize::zero()
        }
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if let Some(node) = self {
            node.layout(wl)
        } else {
            PxSize::zero()
        }
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

/// A UI node that does not contain any other node, only takes the minimum space and renders nothing.
pub struct NilUiNode;
#[ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: &mut WidgetMeasure) -> PxSize {
        LAYOUT.constraints().min_size()
    }

    fn layout(&mut self, _: &mut WidgetLayout) -> PxSize {
        LAYOUT.constraints().min_size()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[ui_node(none)]
impl UiNode for FillUiNode {}

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
