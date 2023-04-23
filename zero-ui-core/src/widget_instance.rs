//! Widget instance types, [`UiNode`], [`UiNodeList`] and others.

use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt,
};

use parking_lot::Mutex;

use crate::{
    context::*,
    event::EventUpdate,
    var::{impl_from_and_into_var, Var},
    widget_base::{Parallel, PARALLEL_VAR},
    widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
};
use crate::{crate_util::NameIdMap, units::*};
use crate::{
    render::{FrameBuilder, FrameUpdate},
    text::Txt,
    ui_node,
};

mod adopt;
pub use adopt::*;

mod arc;
pub use arc::*;

mod match_node;
pub use match_node::*;

mod when;
pub use when::*;

mod list;
pub use list::*;

mod trace;
pub use trace::TraceNode;

pub use crate::crate_util::IdNameError;

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
impl WidgetId {
    fn name_map() -> parking_lot::MappedMutexGuard<'static, NameIdMap<Self>> {
        static NAME_MAP: Mutex<Option<NameIdMap<WidgetId>>> = parking_lot::const_mutex(None);
        parking_lot::MutexGuard::map(NAME_MAP.lock(), |m| m.get_or_insert_with(NameIdMap::new))
    }

    /// Get or generate an id with associated name.
    ///
    /// If the `name` is already associated with an id, returns it.
    /// If the `name` is new, generates a new id and associated it with the name.
    /// If `name` is an empty string just returns a new id.
    pub fn named(name: impl Into<Txt>) -> Self {
        Self::name_map().get_id_or_insert(name.into(), Self::new_unique)
    }

    /// Calls [`named`] in a debug build and [`new_unique`] in a release build.
    ///
    /// The [`named`] function causes a hash-map lookup, but if you are only naming a widget to find
    /// it in the Inspector you don't need that lookup in a release build, so you can set the [`id`]
    /// to this function call instead.
    ///
    /// [`named`]: WidgetId::named
    /// [`new_unique`]: WidgetId::new_unique
    /// [`id`]: fn@crate::widget_base::id
    pub fn debug_named(name: impl Into<Txt>) -> Self {
        #[cfg(debug_assertions)]
        return Self::named(name);

        #[cfg(not(debug_assertions))]
        {
            let _ = name;
            Self::new_unique()
        }
    }

    /// Generate a new id with associated name.
    ///
    /// If the `name` is already associated with an id, returns the [`NameUsed`] error.
    /// If the `name` is an empty string just returns a new id.
    ///
    /// [`NameUsed`]: IdNameError::NameUsed
    pub fn named_new(name: impl Into<Txt>) -> Result<Self, IdNameError<Self>> {
        Self::name_map().new_named(name.into(), Self::new_unique)
    }

    /// Returns the name associated with the id or `""`.
    pub fn name(self) -> Txt {
        Self::name_map().get_name(self)
    }

    /// Associate a `name` with the id, if it is not named.
    ///
    /// If the `name` is already associated with a different id, returns the [`NameUsed`] error.
    /// If the id is already named, with a name different from `name`, returns the [`AlreadyNamed`] error.
    /// If the `name` is an empty string or already is the name of the id, does nothing.
    ///
    /// [`NameUsed`]: IdNameError::NameUsed
    /// [`AlreadyNamed`]: IdNameError::AlreadyNamed
    pub fn set_name(self, name: impl Into<Txt>) -> Result<(), IdNameError<Self>> {
        Self::name_map().set(name.into(), self)
    }
}
impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if f.alternate() {
            f.debug_struct("WidgetId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .field("name", &name)
                .finish()
        } else if !name.is_empty() {
            write!(f, r#"WidgetId("{name}")"#)
        } else {
            write!(f, "WidgetId({})", self.sequential())
        }
    }
}
impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if !name.is_empty() {
            write!(f, "{name}")
        } else {
            write!(f, "WgtId({})", self.sequential())
        }
    }
}
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
}
impl crate::var::IntoVar<Option<WidgetId>> for WidgetId {
    type Var = crate::var::LocalVar<Option<WidgetId>>;

    fn into_var(self) -> Self::Var {
        crate::var::LocalVar(Some(self))
    }
}
impl crate::var::IntoValue<Option<WidgetId>> for WidgetId {}
impl fmt::Debug for StaticWidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.get(), f)
    }
}
impl crate::var::IntoValue<WidgetId> for &'static StaticWidgetId {}

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
    /// [`is_widget`]: UiNode::is_widget
    fn with_context<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        let _ = f;
        None
    }

    /// Gets this node as a [`BoxedUiNode`], if the node [`is_widget`] this is the same node, otherwise a
    /// new widget is generated with the node as the *inner*.
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn into_widget(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        use crate::widget_base::nodes;

        if self.is_widget() {
            return self.boxed();
        }

        let node = nodes::widget_inner(self.cfg_boxed());
        let wgt = nodes::widget(node, WidgetId::new_unique());
        wgt.boxed()
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

    /// Wraps the node in a [`TraceNode`] that, before delegating each method, calls a closure with
    /// the method name as a `&'static str`, the closure can return a *span* that is dropped after the method delegation.
    ///
    /// You can use  the [`tracing`](https://docs.rs/tracing) crate to create the span.
    fn trace<E, S>(self, enter_mtd: E) -> TraceNode<Self, E>
    where
        Self: Sized,
        E: Fn(&'static str) -> S + Send + 'static,
    {
        TraceNode::new(self, enter_mtd)
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

    /// Calls `f` for each node in the list with the index in parallel, folds the results of `f` using the `identity` initial value
    /// and `fold` operation.
    ///
    /// Note that the `identity` may be used more than one time, it should produce a *neutral* value such that it does not affect the
    /// result of a `fold` call with a real value.
    fn par_fold<T, F, I, O>(&mut self, f: F, identity: I, fold: O) -> T
    where
        T: Send,
        F: Fn(usize, &mut BoxedUiNode) -> T + Send + Sync,
        I: Fn() -> T + Send + Sync,
        O: Fn(T, T) -> T + Send + Sync;

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
    fn measure_each<F, S>(&mut self, wm: &mut WidgetMeasure, measure: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetMeasure) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
            self.par_fold(|i, n| measure(i, n, &mut WidgetMeasure::new()), PxSize::zero, fold_size)
        } else {
            let mut size = PxSize::zero();
            self.for_each(|i, n| {
                let b = measure(i, n, wm);
                size = fold_size(size, b);
            });
            size
        }
    }

    /// Call `layout` for each node and combines the final size using `fold_size`.
    ///
    /// The call to `layout` can be parallel if [`Parallel::LAYOUT`] is enabled.
    fn layout_each<F, S>(&mut self, wl: &mut WidgetLayout, layout: F, fold_size: S) -> PxSize
    where
        F: Fn(usize, &mut BoxedUiNode, &mut WidgetLayout) -> PxSize + Send + Sync,
        S: Fn(PxSize, PxSize) -> PxSize + Send + Sync,
    {
        if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::LAYOUT) {
            // fold a tuple of `(wl, size)`
            let swl = &wl;
            let (pwl, size) = self.par_fold(
                move |i, n| {
                    let mut pwl = swl.start_par();
                    let size = layout(i, n, &mut pwl);
                    (pwl, size)
                },
                || (swl.start_par(), PxSize::zero()),
                move |(awl, asize), (bwl, bsize)| (awl.fold(bwl), fold_size(asize, bsize)),
            );
            wl.finish_par(pwl);
            size
        } else {
            let mut size = PxSize::zero();
            self.for_each(|i, n| {
                let b = layout(i, n, wl);
                size = fold_size(size, b);
            });
            size
        }
    }

    /// Render all nodes.
    ///
    /// The correct behavior of some list implementations depend on this call, using [`for_each`] to render nodes can
    /// break it, for example, the [`PanelList`] render nodes in a different order.
    ///
    /// [`for_each`]: UiNodeList::for_each
    fn render_all(&mut self, frame: &mut FrameBuilder) {
        // if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
        //     todo!("parallel render");
        // }
        self.for_each(|_, c| {
            c.render(frame);
        })
    }

    /// Render all nodes.
    ///
    /// The correct behavior of some list implementations depend on this call, using [`for_each`] to render nodes can
    /// break it, for example, the [`PanelList`] render nodes in a different order.
    ///
    /// [`for_each`]: UiNodeList::for_each
    fn render_update_all(&mut self, update: &mut FrameUpdate) {
        // if self.len() > 1 && PARALLEL_VAR.get().contains(Parallel::RENDER) {
        //     todo!("parallel render_update");
        // }
        self.for_each(|_, c| {
            c.render_update(update);
        })
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
        // if list.len() > 1 && PARALLEL_VAR.get().contains(Parallel::INFO) {
        //     todo!("parallel info");
        // }
        list.for_each(|_, c| {
            c.info(info);
        });
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
    fn with_context_boxed(&self, f: &mut dyn FnMut());
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

    fn with_context_boxed(&self, f: &mut dyn FnMut()) {
        self.with_context(f);
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

    fn with_context<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_ref().with_context_boxed(&mut || r = Some((f.take().unwrap())()));
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

    fn par_fold<T, F, I, O>(&mut self, f: F, identity: I, fold: O) -> T
    where
        T: Send,
        F: Fn(usize, &mut BoxedUiNode) -> T + Send + Sync,
        I: Fn() -> T + Send + Sync,
        O: Fn(T, T) -> T + Send + Sync,
    {
        let res = Mutex::new(Some(identity()));

        self.par_each(|i, item| {
            let b = f(i, item);

            let mut res = res.lock();
            let a = res.take().unwrap();

            let r = fold(a, b);
            *res = Some(r);
        });

        res.into_inner().unwrap()
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

    fn with_context<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        match self {
            Some(node) => node.with_context(f),
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

    fn par_fold<T, F, I, O>(&mut self, f: F, identity: I, _: O) -> T
    where
        T: Send,
        F: Fn(usize, &mut BoxedUiNode) -> T + Send + Sync,
        I: Fn() -> T + Send + Sync,
        O: Fn(T, T) -> T + Send + Sync,
    {
        if let Some(node) = self {
            f(0, node)
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
