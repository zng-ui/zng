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
    var::impl_from_and_into_var,
    widget_info::{WidgetInfoBuilder, WidgetLayout},
};
use crate::{crate_util::NameIdMap, units::*};
use crate::{
    render::{FrameBuilder, FrameUpdate},
    text::Text,
    ui_node,
};

mod adopt;
pub use adopt::*;

mod rc_node;
pub use rc_node::*;

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
    pub fn named(name: impl Into<Text>) -> Self {
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
    /// [`id`]: mod@crate::widget_base::base#wp-id
    pub fn debug_named(name: impl Into<Text>) -> Self {
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
    pub fn named_new(name: impl Into<Text>) -> Result<Self, IdNameError<Self>> {
        Self::name_map().new_named(name.into(), Self::new_unique)
    }

    /// Returns the name associated with the id or `""`.
    pub fn name(self) -> Text {
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
    pub fn set_name(self, name: impl Into<Text>) -> Result<(), IdNameError<Self>> {
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
    fn from(name: Text) -> WidgetId {
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
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait UiNode: Any {
    /// Called every time the node is plugged into the UI tree.
    ///
    /// The parent node that calls this method must make an info, subscriptions, layout and render update request, the initializing node it self
    /// does not need to request these updates, it needs only to initialize self and descendants.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from the UI tree.
    ///
    /// The parent node that calls this method must make an info, subscriptions, layout and render update request, the de-initializing node it self
    /// does not need to request these updates, it needs only to de-initialize self and descendants.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time there are structural changes in the UI tree such as a node added or removed.
    ///
    /// # Arguments
    ///
    /// * `ctx`: Limited context access.
    /// * `info`: Widget info tree builder.
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);

    /// Called every time an event updates.
    ///
    /// Every call to this method is for a single update of a single event type, you can listen to events
    /// using the [`Event::on`] method or other methods of the [`Event`] type.
    ///
    /// [`Event::on`]: crate::event::Event::on
    /// [`Event`]: crate::event::Event
    fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate);

    /// Called every time an update is requested.
    ///
    /// An update can be requested using the context [`Updates`], after each request, they also happen
    /// when variables update and any other context or service structure that can be observed updates.
    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates);

    /// Compute the widget size given the contextual layout metrics.
    ///
    /// Implementers must return the same size [`layout`] returns for the given [`LayoutMetrics`], without
    /// updating the widget state.
    ///
    /// # Arguments
    ///
    /// * `ctx`: Limited layout context access.
    ///
    /// # Returns
    ///
    /// Returns the computed node size, this will probably influencing the actual constrains that will be used
    /// on a subsequent [`layout`] call.
    ///
    /// [`layout`]: Self::layout
    fn measure(&self, ctx: &mut MeasureContext) -> PxSize;

    /// Called every time a layout update is requested or the constrains used have changed.
    ///
    /// Implementers must try to fit their size inside the [`constrains`] as best as it can and return an accurate final size. If
    /// the size breaks the constrains the widget may end-up clipped.
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
    /// [`constrains`]: LayoutMetrics::constrains
    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize;

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    ///
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    ///
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

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

    /// Calls `f` with the widget context of the node if it [`is_widget`].
    ///
    /// Returns `None` if the node does not represent an widget.
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn with_context<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetNodeContext) -> R,
    {
        let _ = f;
        None
    }

    /// Calls `f` with the widget context of the node if it [`is_widget`].
    ///
    /// Returns `None` if the node does not represent an widget.
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn with_context_mut<R, F>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetNodeMutContext) -> R,
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

        let node = nodes::inner(self.cfg_boxed());
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
            Ok(*boxed.as_any_boxed().downcast().unwrap())
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

    /// Wraps the node in a [`TraceNode`] that, before delegating each method, calls a closure with an [`InfoContext`]
    /// and the method name as a `&'static str`, the closure can return a *span* that is dropped after the method delegation.
    ///
    /// You can use  the [`tracing`](https://docs.rs/tracing) crate to create the span.
    fn trace<E, S>(self, enter_mtd: E) -> TraceNode<Self, E>
    where
        Self: Sized,
        E: Fn(&mut InfoContext, &'static str) -> S + 'static,
    {
        TraceNode::new(self, enter_mtd)
    }
}

/// Represents a list of [`UiNode`] instances.
///
/// Panel implementers must delegate every [`UiNode`] method to every node in the children list, in particular the
/// [`init_all`], [`deinit_all`] and [`update_all`] methods must be used to support reactive lists, and the [`render_all`]
/// and [`render_update_all`] must be used to render, to support the [`ZSortingList`]. Other [`UiNode`] methods must
/// be delegated using [`for_each`] and [`for_each_mut`].
///
/// The [`#[ui_node(children)]`] attribute macro auto-generates delegations for each method.
///
/// [`init_all`]: UiNodeList::init_all
/// [`deinit_all`]: UiNodeList::deinit_all
/// [`update_all`]: UiNodeList::update_all
/// [`render_all`]: UiNodeList::render_all
/// [`render_update_all`]: UiNodeList::render_update_all
/// [`for_each`]: UiNodeList::for_each
/// [`for_each_mut`]: UiNodeList::for_each_mut
pub trait UiNodeList: UiNodeListBoxed {
    /// Visit the specific node, panic if `index` is out of bounds.
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R;

    /// Visit the specific node, panic if `index` is out of bounds.
    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R;

    /// Calls `f` for each node in the list with the index, return `true` to continue iterating, return `false` to stop.
    fn for_each<F>(&self, f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool;

    /// Calls `f` for each node in the list with the index, return `true` to continue iterating, return `false` to stop.
    fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool;

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
    /// The functionality of some list implementations depend on this call, using [`for_each_mut`] to init nodes is an error.
    ///
    /// [`for_each_mut`]: UiNodeList::for_each_mut
    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.for_each_mut(|_, c| {
            c.init(ctx);
            true
        });
    }

    /// Deinit the list in a context, all nodes are also deinited.
    ///
    /// The functionality of some list implementations depend on this call, using [`for_each_mut`] to deinit nodes is an error.
    ///
    /// [`for_each_mut`]: UiNodeList::for_each_mut
    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.for_each_mut(|_, c| {
            c.deinit(ctx);
            true
        });
    }

    /// Receive updates for the list in a context, all nodes are also updated.
    ///
    /// The functionality of some list implementations depend on this call, using [`for_each_mut`] to update nodes is an error.
    ///
    /// [`for_each_mut`]: UiNodeList::for_each_mut
    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let _ = observer;
        self.for_each_mut(|_, c| {
            c.update(ctx, updates);
            true
        });
    }

    /// Receive an event for the list in a context, all nodes are also notified.
    ///
    /// The functionality of some list implementations depend on this call, using [`for_each_mut`] to notify nodes is an error.
    ///
    /// [`for_each_mut`]: UiNodeList::for_each_mut
    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.for_each_mut(|_, c| {
            c.event(ctx, update);
            true
        })
    }

    /// Render all nodes.
    ///
    /// The correct functionality of some list implementations depend on this call, using [`for_each`] to render nodes can
    /// break then, for example, the [`ZSortingList`] render nodes in a different order.
    ///
    /// [`for_each`]: UiNodeList::for_each
    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.for_each(|_, c| {
            c.render(ctx, frame);
            true
        })
    }

    /// Render all nodes.
    ///
    /// The correct functionality of some list implementations depend on this call, using [`for_each`] to render nodes can
    /// break then, for example, the [`ZSortingList`] render nodes in a different order.
    ///
    /// [`for_each`]: UiNodeList::for_each
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.for_each(|_, c| {
            c.render_update(ctx, update);
            true
        })
    }
}

#[doc(hidden)]
pub mod ui_node_list_default {
    use super::*;

    pub fn init_all(list: &mut impl UiNodeList, ctx: &mut WidgetContext) {
        list.init_all(ctx);
    }

    pub fn deinit_all(list: &mut impl UiNodeList, ctx: &mut WidgetContext) {
        list.deinit_all(ctx);
    }

    pub fn info_all(list: &impl UiNodeList, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        list.for_each(|_, c| {
            c.info(ctx, info);
            true
        });
    }

    pub fn event_all(list: &mut impl UiNodeList, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        list.event_all(ctx, update);
    }

    pub fn update_all(list: &mut impl UiNodeList, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        let mut changed = false;

        list.update_all(ctx, updates, &mut changed);

        if changed {
            ctx.updates.layout_and_render();
        }
    }

    pub fn measure_all(list: &impl UiNodeList, ctx: &mut MeasureContext) -> PxSize {
        let mut r = PxSize::zero();
        list.for_each(|_, n| {
            r = r.max(n.measure(ctx));
            true
        });
        r
    }

    pub fn layout_all(list: &mut impl UiNodeList, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        let mut r = PxSize::zero();
        list.for_each_mut(|_, n| {
            r = r.max(n.layout(ctx, wl));
            true
        });
        r
    }

    pub fn render_all(list: &impl UiNodeList, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        list.render_all(ctx, frame);
    }

    pub fn render_update_all(list: &impl UiNodeList, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        list.render_update_all(ctx, update)
    }
}

#[doc(hidden)]
pub trait UiNodeBoxed: Any {
    fn info_boxed(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);
    fn init_boxed(&mut self, ctx: &mut WidgetContext);
    fn deinit_boxed(&mut self, ctx: &mut WidgetContext);
    fn update_boxed(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates);
    fn event_boxed(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate);
    fn measure_boxed(&self, ctx: &mut MeasureContext) -> PxSize;
    fn layout_boxed(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize;
    fn render_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
    fn render_update_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    fn is_widget_boxed(&self) -> bool;
    fn with_context_boxed(&self, f: &mut dyn FnMut(&mut WidgetNodeContext));
    fn with_context_mut_boxed(&mut self, f: &mut dyn FnMut(&mut WidgetNodeMutContext));
    fn into_widget_boxed(self: Box<Self>) -> BoxedUiNode;

    fn actual_type_id_boxed(&self) -> TypeId;
    fn as_any_boxed(self: Box<Self>) -> Box<dyn Any>;
}

impl<U: UiNode> UiNodeBoxed for U {
    fn info_boxed(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.info(ctx, info);
    }

    fn init_boxed(&mut self, ctx: &mut WidgetContext) {
        self.init(ctx);
    }

    fn deinit_boxed(&mut self, ctx: &mut WidgetContext) {
        self.deinit(ctx);
    }

    fn update_boxed(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        self.update(ctx, updates);
    }

    fn event_boxed(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.event(ctx, update);
    }

    fn measure_boxed(&self, ctx: &mut MeasureContext) -> PxSize {
        self.measure(ctx)
    }

    fn layout_boxed(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        self.layout(ctx, wl)
    }

    fn render_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.render(ctx, frame);
    }

    fn render_update_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.render_update(ctx, update);
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

    fn as_any_boxed(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn with_context_boxed(&self, f: &mut dyn FnMut(&mut WidgetNodeContext)) {
        self.with_context(f);
    }

    fn with_context_mut_boxed(&mut self, f: &mut dyn FnMut(&mut WidgetNodeMutContext)) {
        self.with_context_mut(f);
    }
}

#[doc(hidden)]
pub trait UiNodeListBoxed: Any {
    fn with_node_boxed(&self, index: usize, f: &mut dyn FnMut(&BoxedUiNode));
    fn with_node_mut_boxed(&mut self, index: usize, f: &mut dyn FnMut(&mut BoxedUiNode));
    fn for_each_boxed(&self, f: &mut dyn FnMut(usize, &BoxedUiNode) -> bool);
    fn for_each_mut_boxed(&mut self, f: &mut dyn FnMut(usize, &mut BoxedUiNode) -> bool);
    fn len_boxed(&self) -> usize;
    fn drain_into_boxed(&mut self, vec: &mut Vec<BoxedUiNode>);
    fn init_all_boxed(&mut self, ctx: &mut WidgetContext);
    fn deinit_all_boxed(&mut self, ctx: &mut WidgetContext);
    fn event_all_boxed(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate);
    fn update_all_boxed(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver);
    fn render_all_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
    fn render_update_all_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);
}
impl<L: UiNodeList> UiNodeListBoxed for L {
    fn with_node_boxed(&self, index: usize, f: &mut dyn FnMut(&BoxedUiNode)) {
        self.with_node(index, f)
    }

    fn with_node_mut_boxed(&mut self, index: usize, f: &mut dyn FnMut(&mut BoxedUiNode)) {
        self.with_node_mut(index, f)
    }

    fn for_each_boxed(&self, f: &mut dyn FnMut(usize, &BoxedUiNode) -> bool) {
        self.for_each(f);
    }

    fn for_each_mut_boxed(&mut self, f: &mut dyn FnMut(usize, &mut BoxedUiNode) -> bool) {
        self.for_each_mut(f);
    }

    fn len_boxed(&self) -> usize {
        self.len()
    }

    fn drain_into_boxed(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.drain_into(vec)
    }

    fn init_all_boxed(&mut self, ctx: &mut WidgetContext) {
        self.init_all(ctx);
    }

    fn deinit_all_boxed(&mut self, ctx: &mut WidgetContext) {
        self.deinit_all(ctx);
    }

    fn event_all_boxed(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.event_all(ctx, update);
    }

    fn update_all_boxed(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.update_all(ctx, updates, observer);
    }

    fn render_all_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.render_all(ctx, frame);
    }

    fn render_update_all_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.render_update_all(ctx, update);
    }
}

/// An [`UiNode`] in a box.
pub type BoxedUiNode = Box<dyn UiNodeBoxed>;

/// An [`UiNodeList`] in a box.
pub type BoxedUiNodeList = Box<dyn UiNodeListBoxed>;

impl UiNode for BoxedUiNode {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.as_ref().info_boxed(ctx, info);
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().init_boxed(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().deinit_boxed(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        self.as_mut().update_boxed(ctx, updates);
    }

    fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.as_mut().event_boxed(ctx, update);
    }

    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        self.as_ref().measure_boxed(ctx)
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        self.as_mut().layout_boxed(ctx, wl)
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.as_ref().render_boxed(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.as_ref().render_update_boxed(ctx, update);
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
        F: FnOnce(&mut WidgetNodeContext) -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_ref().with_context_boxed(&mut |ctx| r = Some((f.take().unwrap())(ctx)));
        r
    }

    fn with_context_mut<R, F>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetNodeMutContext) -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_mut().with_context_mut_boxed(&mut |ctx| r = Some((f.take().unwrap())(ctx)));
        r
    }

    fn into_widget(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        self.into_widget_boxed()
    }
}

impl UiNodeList for BoxedUiNodeList {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_ref().with_node_boxed(index, &mut |n| r = Some((f.take().unwrap())(n)));
        r.unwrap()
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        let mut f = Some(f);
        let mut r = None;
        self.as_mut().with_node_mut_boxed(index, &mut |n| r = Some((f.take().unwrap())(n)));
        r.unwrap()
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        self.as_ref().for_each_boxed(&mut f);
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        self.as_mut().for_each_mut_boxed(&mut f)
    }

    fn len(&self) -> usize {
        self.as_ref().len_boxed()
    }

    fn boxed(self) -> BoxedUiNodeList {
        self
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.as_mut().drain_into_boxed(vec)
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().init_all_boxed(ctx);
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().deinit_all_boxed(ctx);
    }

    fn update_all(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        self.as_mut().update_all_boxed(ctx, updates, observer);
    }

    fn event_all(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        self.as_mut().event_all_boxed(ctx, update);
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.as_ref().render_all_boxed(ctx, frame);
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.as_ref().render_update_all_boxed(ctx, update);
    }
}

impl<U: UiNode> UiNode for Option<U> {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        if let Some(node) = self {
            node.info(ctx, info);
        }
    }

    fn init(&mut self, ctx: &mut WidgetContext) {
        if let Some(node) = self {
            node.init(ctx);
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if let Some(node) = self {
            node.deinit(ctx);
        }
    }

    fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
        if let Some(node) = self {
            node.event(ctx, update);
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
        if let Some(node) = self {
            node.update(ctx, updates);
        }
    }

    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        if let Some(node) = self {
            node.measure(ctx)
        } else {
            PxSize::zero()
        }
    }

    fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
        if let Some(node) = self {
            node.layout(ctx, wl)
        } else {
            PxSize::zero()
        }
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        if let Some(node) = self {
            node.render(ctx, frame);
        }
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        if let Some(node) = self {
            node.render_update(ctx, update);
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
        F: FnOnce(&mut WidgetNodeContext) -> R,
    {
        match self {
            Some(node) => node.with_context(f),
            None => None,
        }
    }

    fn with_context_mut<R, F>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetNodeMutContext) -> R,
    {
        match self {
            Some(node) => node.with_context_mut(f),
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
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
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

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
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

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        if let Some(node) = self {
            f(0, node);
        }
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        if let Some(node) = self {
            f(0, node);
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

impl UiNodeList for Vec<BoxedUiNode> {
    fn with_node<R, F>(&self, index: usize, f: F) -> R
    where
        F: FnOnce(&BoxedUiNode) -> R,
    {
        f(&self[index])
    }

    fn with_node_mut<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        f(&mut self[index])
    }

    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(usize, &BoxedUiNode) -> bool,
    {
        for (i, node) in self.iter().enumerate() {
            if !f(i, node) {
                break;
            }
        }
    }

    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode) -> bool,
    {
        for (i, node) in self.iter_mut().enumerate() {
            if !f(i, node) {
                break;
            }
        }
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        vec.append(self)
    }
}

/// A UI node that does not contain any other node, only takes the minimum space and renders nothing.
pub struct NilUiNode;
#[ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
        ctx.constrains().min_size()
    }

    fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
        ctx.constrains().min_size()
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
