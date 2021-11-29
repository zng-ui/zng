use std::{
    cell::{Cell, RefCell},
    fmt, mem,
    rc::{Rc, Weak},
};

use parking_lot::Mutex;

use crate::{
    context::*,
    event::{AnyEventUpdate, Event},
    widget_info::{WidgetInfoBuilder, WidgetOffset},
    IdNameError,
};
use crate::{crate_util::NameIdMap, units::*};
use crate::{
    event::EventUpdateArgs,
    impl_ui_node,
    render::{FrameBuilder, FrameUpdate},
};

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
    pub fn named(name: &'static str) -> Self {
        Self::name_map().get_id_or_insert(name, Self::new_unique)
    }

    /// Calls [`named`] in a debug build and [`new_unique`] in a release build.
    ///
    /// The [`named`] function causes a hash-map lookup, but if you are only naming a widget to find
    /// it in the Inspector you don't need that lookup in a release build, so you can set the [`id`]
    /// to this function call instead.
    ///
    /// [`named`]: WidgetId::named
    /// [`new_unique`]: WidgetId::new_unique
    /// [`id`]: mod@crate::widget_base::implicit_base#wp-id
    #[inline(always)]
    pub fn debug_named(name: &'static str) -> Self {
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
    pub fn named_new(name: &'static str) -> Result<Self, IdNameError<Self>> {
        Self::name_map().new_named(name, Self::new_unique)
    }

    /// Returns the name associated with the id or `""`.
    pub fn name(self) -> &'static str {
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
    pub fn set_name(self, name: &'static str) -> Result<(), IdNameError<Self>> {
        Self::name_map().set(name, self)
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
            write!(f, "WidgetId({:?})", name)
        } else {
            write!(f, "WidgetId({:x})", self.sequential())
        }
    }
}
impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if !name.is_empty() {
            write!(f, "{}", name)
        } else {
            write!(f, "WgtId({})", self.sequential())
        }
    }
}
impl From<&'static str> for WidgetId {
    /// Calls [`WidgetId::named`].
    fn from(name: &'static str) -> Self {
        WidgetId::named(name)
    }
}

/// An Ui tree node.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait UiNode: 'static {
    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
    fn deinit(&mut self, ctx: &mut WidgetContext);

    /// Called every time an event updates.
    ///
    /// Every call to this method is for a single update of a single event type, you can listen to events
    /// using [`Event::update`]. This method is called even if [`stop_propagation`](crate::event::EventArgs::stop_propagation)
    /// was requested, or a parent widget is disabled, and it must always propagate to descendent nodes.
    ///
    /// Event propagation can be statically or dynamically typed, the way to listen to both is the same, `A` can be an
    /// [`AnyEventUpdate`] instance that is resolved dynamically or an [`EventUpdate`](crate::event::EventUpdate) instance
    /// that is resolved statically in the [`Event::update`] call. If an event matches you should **use the returned args**
    /// in the call the descendant nodes, **this upgrades from dynamic to static resolution** in descendant nodes increasing
    /// performance.
    ///
    /// If the event is handled before the call in descendant nodes it is called a *preview*, this behavior matches what
    /// happens in the [`on_pre_event`](crate::event::on_pre_event) node.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::{UiNode, impl_ui_node, context::WidgetContext, widget_base::IsEnabled, event::EventUpdateArgs, gesture::ClickEvent};
    /// struct MyNode<C> {
    ///     child: C,
    ///     click_count: usize
    /// }
    /// #[impl_ui_node(child)]
    /// impl<C: UiNode> UiNode for MyNode<C> {
    ///     fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
    ///         if let Some(args) = ClickEvent.update(args) {
    ///             if args.concerns_widget(ctx) && IsEnabled::get(ctx) && !args.stop_propagation_requested() {
    ///                 self.click_count += 1;
    ///                 args.stop_propagation();
    ///                 println!("clicks blocked {}", self.click_count);
    ///             }
    ///             self.child.event(ctx, args);
    ///         }
    ///         else {
    ///             self.child.event(ctx, args);
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// In the example the `ClickEvent` event is handled in *preview* style (before child), but only if
    /// the parent widget was clicked and the widget is enabled and stop propagation was not requested. The event
    /// is then propagated to the `child` node, `self.child.event` appears to be called twice but if the `if` call
    /// we ensured that the descendant nodes will resolve the event statically, which can not be the case in the `else`
    /// call where `A` can be the dynamic resolver.
    fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A);

    /// Called every time an update is requested.
    ///
    /// An update happens every time after a sequence of [`event`](Self::event), they also happen
    /// when variables update and any other context or service structure that can be observed updates.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is needed.
    ///
    /// # Arguments
    ///
    /// * `ctx`: Layout context.
    /// * `available_size`: The total available size for the node. Can be infinity or a maximum size.
    ///
    /// # Returns
    ///
    /// Returns the node's desired size.
    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize;

    /// Called every time a layout update is needed, after [`measure`](UiNode::measure).
    ///
    /// # Arguments
    ///
    /// * `ctx`: Layout context, allows only variable operations.
    /// * `final_size`: The size the parent node reserved for the node. Must reposition its contents
    ///   to fit this size. The value does not contain infinity or NaNs and is pixel aligned.
    fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize);

    /// Called every time there are structural changes in the UI tree such as a node added or removed.
    ///
    /// # Arguments
    ///
    /// * `info`:
    fn info(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder);

    /// Called every time a new frame must be rendered.
    ///
    /// # Arguments
    /// * `frame`: Contains the next frame draw instructions.
    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Called every time a frame can be updated without fully rebuilding.
    ///
    /// # Arguments
    /// * `update`: Contains the frame value updates.
    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Box this node, unless it is already `BoxedUiNode`.
    fn boxed(self) -> BoxedUiNode
    where
        Self: Sized,
    {
        Box::new(self)
    }
}
#[doc(hidden)]
pub trait UiNodeBoxed: 'static {
    fn init_boxed(&mut self, ctx: &mut WidgetContext);
    fn deinit_boxed(&mut self, ctx: &mut WidgetContext);
    fn update_boxed(&mut self, ctx: &mut WidgetContext);
    fn event_boxed(&mut self, ctx: &mut WidgetContext, args: &AnyEventUpdate);
    fn measure_boxed(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize;
    fn arrange_boxed(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize);
    fn info_boxed(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder);
    fn render_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
    fn render_update_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);
}

impl<U: UiNode> UiNodeBoxed for U {
    fn init_boxed(&mut self, ctx: &mut WidgetContext) {
        self.init(ctx);
    }

    fn deinit_boxed(&mut self, ctx: &mut WidgetContext) {
        self.deinit(ctx);
    }

    fn update_boxed(&mut self, ctx: &mut WidgetContext) {
        self.update(ctx);
    }

    fn event_boxed(&mut self, ctx: &mut WidgetContext, args: &AnyEventUpdate) {
        self.event(ctx, args);
    }

    fn measure_boxed(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.measure(ctx, available_size)
    }

    fn arrange_boxed(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        self.arrange(ctx, widget_offset, final_size);
    }

    fn info_boxed(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        self.info(ctx, info);
    }

    fn render_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.render(ctx, frame);
    }

    fn render_update_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.render_update(ctx, update);
    }
}

/// An [`UiNode`] in a box.
pub type BoxedUiNode = Box<dyn UiNodeBoxed>;

impl UiNode for BoxedUiNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().init_boxed(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().deinit_boxed(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().update_boxed(ctx);
    }

    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU)
    where
        Self: Sized,
    {
        let args = args.as_any();
        self.as_mut().event_boxed(ctx, &args);
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.as_mut().measure_boxed(ctx, available_size)
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        self.as_mut().arrange_boxed(ctx, widget_offset, final_size);
    }

    fn info(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        self.as_ref().info_boxed(ctx, info);
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
}

impl<U: UiNode> UiNode for Option<U> {
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

    fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
        if let Some(node) = self {
            node.event(ctx, args);
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(node) = self {
            node.update(ctx);
        }
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        if let Some(node) = self {
            node.measure(ctx, available_size)
        } else {
            PxSize::zero()
        }
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        if let Some(node) = self {
            node.arrange(ctx, widget_offset, final_size);
        }
    }

    fn info(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        if let Some(node) = self {
            node.info(ctx, info);
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
}

macro_rules! declare_widget_test_calls {
    ($(
        $method:ident
    ),+) => {$(paste::paste! {
        #[doc = "Run [`UiNode::"$method "`] using the [`TestWidgetContext`]."]
        #[cfg(any(test, doc, feature = "test_util"))]
        #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
        fn [<test_ $method>](&mut self, ctx: &mut TestWidgetContext) {
            // `self` already creates an `widget_context`, we assume, so this
            // call is for a dummy parent of `self`.
            ctx.widget_context(|ctx| {
                self.$method(ctx);
            });
        }
    })+};
}

/// Represents an widget [`UiNode`].
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Widget: UiNode {
    /// Id of the widget.
    fn id(&self) -> WidgetId;

    /// Reference the widget lazy state.
    fn state(&self) -> &StateMap;
    /// Exclusive borrow the widget lazy state.
    fn state_mut(&mut self) -> &mut StateMap;

    /// Last arranged size.
    fn size(&self) -> PxSize;

    /// Box this widget node, unless it is already `BoxedWidget`.
    fn boxed_widget(self) -> BoxedWidget
    where
        Self: Sized,
    {
        Box::new(self)
    }

    declare_widget_test_calls! {
        init, deinit, update
    }

    /// Run [`UiNode::measure`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_measure(&mut self, ctx: &mut TestWidgetContext, available_size: AvailableSize) -> PxSize {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());
        ctx.layout_context(font_size, font_size, self.size(), 1.0.fct(), 96.0, LayoutMask::all(), |ctx| {
            self.measure(ctx, available_size)
        })
    }
    /// Run [`UiNode::arrange`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_arrange(&mut self, ctx: &mut TestWidgetContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());
        ctx.layout_context(font_size, font_size, self.size(), 1.0.fct(), 96.0, LayoutMask::all(), |ctx| {
            self.arrange(ctx, widget_offset, final_size)
        })
    }

    // TODO don't require user to init frame?

    /// Run [`UiNode::info`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_info(&self, ctx: &mut TestWidgetContext, info: &mut WidgetInfoBuilder) {
        ctx.render_context(|ctx| self.info(ctx, info));
    }

    /// Run [`UiNode::render`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_render(&self, ctx: &mut TestWidgetContext, frame: &mut FrameBuilder) {
        ctx.render_context(|ctx| self.render(ctx, frame));
    }

    /// Run [`UiNode::render_update`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_render_update(&self, ctx: &mut TestWidgetContext, update: &mut FrameUpdate) {
        ctx.render_context(|ctx| self.render_update(ctx, update));
    }
}

#[doc(hidden)]
pub trait WidgetBoxed: UiNodeBoxed {
    fn id_boxed(&self) -> WidgetId;
    fn state_boxed(&self) -> &StateMap;
    fn state_mut_boxed(&mut self) -> &mut StateMap;
    fn size_boxed(&self) -> PxSize;
}
impl<W: Widget> WidgetBoxed for W {
    fn id_boxed(&self) -> WidgetId {
        self.id()
    }

    fn state_boxed(&self) -> &StateMap {
        self.state()
    }

    fn state_mut_boxed(&mut self) -> &mut StateMap {
        self.state_mut()
    }

    fn size_boxed(&self) -> PxSize {
        self.size()
    }
}

/// An [`Widget`] in a box.
pub type BoxedWidget = Box<dyn WidgetBoxed>;

impl UiNode for BoxedWidget {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().init_boxed(ctx);
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().deinit_boxed(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.as_mut().update_boxed(ctx);
    }

    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU)
    where
        Self: Sized,
    {
        let args = args.as_any();
        self.as_mut().event_boxed(ctx, &args);
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.as_mut().measure_boxed(ctx, available_size)
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        self.as_mut().arrange_boxed(ctx, widget_offset, final_size)
    }

    fn info(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        self.as_ref().info_boxed(ctx, info);
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.as_ref().render_boxed(ctx, frame);
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.as_ref().render_update_boxed(ctx, update);
    }
}
impl Widget for BoxedWidget {
    fn id(&self) -> WidgetId {
        self.as_ref().id_boxed()
    }

    fn state(&self) -> &StateMap {
        self.as_ref().state_boxed()
    }

    fn state_mut(&mut self) -> &mut StateMap {
        self.as_mut().state_mut_boxed()
    }

    fn size(&self) -> PxSize {
        self.as_ref().size_boxed()
    }
}

/// A UI node that does not contain any other node, does not take any space and renders nothing.
pub struct NilUiNode;
#[impl_ui_node(none)]
impl UiNode for NilUiNode {
    fn measure(&mut self, _: &mut LayoutContext, _: AvailableSize) -> PxSize {
        PxSize::zero()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

// Used by #[impl_ui_node] to validate custom delegation.
#[doc(hidden)]
pub mod impl_ui_node_util {
    use crate::{
        context::{LayoutContext, RenderContext, WidgetContext},
        event::EventUpdateArgs,
        render::{FrameBuilder, FrameUpdate},
        units::{AvailableSize, PxSize},
        widget_info::{WidgetInfoBuilder, WidgetOffset},
        UiNode, UiNodeList,
    };

    #[inline]
    pub fn delegate<U: UiNode + ?Sized>(d: &U) -> &U {
        d
    }
    #[inline]
    pub fn delegate_mut<U: UiNode + ?Sized>(d: &mut U) -> &mut U {
        d
    }

    #[inline]
    pub fn delegate_list<U: UiNodeList + ?Sized>(d: &U) -> &U {
        d
    }
    #[inline]
    pub fn delegate_list_mut<U: UiNodeList + ?Sized>(d: &mut U) -> &mut U {
        d
    }

    #[inline]
    pub fn delegate_iter<'a>(d: impl IntoIterator<Item = &'a impl UiNode>) -> impl IterImpl {
        d
    }
    #[inline]
    pub fn delegate_iter_mut<'a>(d: impl IntoIterator<Item = &'a mut impl UiNode>) -> impl IterMutImpl {
        d
    }

    pub trait IterMutImpl {
        fn init_all(self, ctx: &mut WidgetContext);
        fn deinit_all(self, ctx: &mut WidgetContext);
        fn update_all(self, ctx: &mut WidgetContext);
        fn event_all<EU: EventUpdateArgs>(self, ctx: &mut WidgetContext, args: &EU);
        fn measure_all(self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize;
        fn arrange_all(self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize);
    }
    pub trait IterImpl {
        fn info_all(self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder);
        fn render_all(self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
        fn render_update_all(self, ctx: &mut RenderContext, update: &mut FrameUpdate);
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u mut U>> IterMutImpl for I {
        fn init_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.init(ctx);
            }
        }

        fn deinit_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.deinit(ctx);
            }
        }

        fn update_all(self, ctx: &mut WidgetContext) {
            for child in self {
                child.update(ctx);
            }
        }

        fn event_all<EU: EventUpdateArgs>(self, ctx: &mut WidgetContext, args: &EU) {
            for child in self {
                child.event(ctx, args);
            }
        }

        fn measure_all(self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let mut size = PxSize::zero();
            for child in self {
                size = child.measure(ctx, available_size).max(size);
            }
            size
        }

        fn arrange_all(self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
            for child in self {
                child.arrange(ctx, widget_offset, final_size);
            }
        }
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u U>> IterImpl for I {
        fn info_all(self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
            for child in self {
                child.info(ctx, info);
            }
        }

        fn render_all(self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            for child in self {
                child.render(ctx, frame);
            }
        }

        fn render_update_all(self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            for child in self {
                child.render_update(ctx, update);
            }
        }
    }
}

/// A reference counted [`UiNode`].
///
/// Nodes can only appear in one place of the UI tree at a time, this `struct` allows the
/// creation of ***slots*** that are [`UiNode`] implementers that can *exclusive take* the
/// referenced node as its child.
///
/// When a slot takes the node it is deinited in the previous UI tree place and reinited in the slot place.
///
/// Slots hold a strong reference to the node when they have it as their child and a weak reference when they don't.
pub struct RcNode<U: UiNode>(Rc<RcNodeData<U>>);
impl<U: UiNode> Clone for RcNode<U> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<U: UiNode> RcNode<U> {
    /// New rc node.
    ///
    /// The `node` is assumed to not be inited.
    pub fn new(node: U) -> Self {
        Self(Rc::new(RcNodeData::new(Some(node))))
    }

    /// New rc node that contains a weak reference to itself.
    ///
    /// **Note** the weak reference cannot be [upgraded](WeakNode::upgrade) during the call to `node`.
    pub fn new_cyclic(node: impl FnOnce(WeakNode<U>) -> U) -> Self {
        // Note: Rewrite this method with `Rc::new_cyclic` when
        // https://github.com/rust-lang/rust/issues/75861 stabilizes
        let r = Self(Rc::new(RcNodeData::new(None)));
        let n = node(r.downgrade());
        *r.0.node.borrow_mut() = Some(n);
        r
    }

    /// Creates an [`UiNode`] implementer that can *exclusive take* the referenced node as its child when
    /// signaled by `take_signal`.
    pub fn slot<S: RcNodeTakeSignal>(&self, take_signal: S) -> impl UiNode {
        SlotNode {
            slot_id: self.0.next_id(),
            take_signal,
            event_signal: false,
            state: if S::TAKE_ON_INIT {
                SlotNodeState::TakeOnInit(Rc::clone(&self.0))
            } else {
                SlotNodeState::Inactive(Rc::downgrade(&self.0))
            },
        }
    }

    /// Creates a new [`WeakNode`] that points to this node.
    #[inline]
    pub fn downgrade(&self) -> WeakNode<U> {
        WeakNode(Rc::downgrade(&self.0))
    }
}

/// `Weak` version of [`RcNode`].
pub struct WeakNode<U: UiNode>(Weak<RcNodeData<U>>);
impl<U: UiNode> Clone for WeakNode<U> {
    fn clone(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl<U: UiNode> WeakNode<U> {
    /// Attempts to upgrade to a [`RcNode`].
    pub fn upgrade(&self) -> Option<RcNode<U>> {
        if let Some(rc) = self.0.upgrade() {
            if rc.node.borrow().is_some() {
                return Some(RcNode(rc));
            }
        }
        None
    }
}

/// Signal an [`RcNode`] slot to take the referenced node as its child.
///
/// This trait is implemented for all `bool` variables, you can also use [`take_on_init`] to
/// be the first slot to take the widget, [`take_on`] to take when an event updates or [`take_if`]
/// to use a custom delegate to signal.
pub trait RcNodeTakeSignal: 'static {
    /// If slot node must take the node when it is created.
    const TAKE_ON_INIT: bool = false;

    /// Returns `true` when the slot must take the node as its child.
    fn update_take(&mut self, ctx: &mut WidgetContext) -> bool {
        let _ = ctx;
        false
    }

    /// Returns `true` when the slot must take the node as its child.
    fn event_take<E: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &E) -> bool {
        let _ = (ctx, args);
        false
    }
}
impl<V> RcNodeTakeSignal for V
where
    V: crate::var::Var<bool>,
{
    /// Takes the widget when the var value is `true`.
    fn update_take(&mut self, ctx: &mut WidgetContext) -> bool {
        *self.get(ctx)
    }
}
/// An [`RcNodeTakeSignal`] that takes the widget when `custom` returns `true`.
pub fn take_if<F: FnMut(&mut WidgetContext) -> bool + 'static>(custom: F) -> impl RcNodeTakeSignal {
    struct TakeIf<F>(F);
    impl<F: FnMut(&mut WidgetContext) -> bool + 'static> RcNodeTakeSignal for TakeIf<F> {
        fn update_take(&mut self, ctx: &mut WidgetContext) -> bool {
            (self.0)(ctx)
        }
    }
    TakeIf(custom)
}
/// An [`RcNodeTakeSignal`] that takes the widget every time the `event` updates and passes the filter.
pub fn take_on<E, F>(event: E, filter: F) -> impl RcNodeTakeSignal
where
    E: Event,
    F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
{
    struct TakeOn<E, F>(E, F);
    impl<E, F> RcNodeTakeSignal for TakeOn<E, F>
    where
        E: Event,
        F: FnMut(&mut WidgetContext, &E::Args) -> bool + 'static,
    {
        fn event_take<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) -> bool {
            self.0.update(args).map(|a| (self.1)(ctx, a)).unwrap_or_default()
        }
    }
    TakeOn(event, filter)
}
/// An [`RcNodeTakeSignal`] that takes the widget once on init.
pub fn take_on_init() -> impl RcNodeTakeSignal {
    struct TakeOnInit;
    impl RcNodeTakeSignal for TakeOnInit {
        const TAKE_ON_INIT: bool = true;
    }
    TakeOnInit
}

struct RcNodeData<U: UiNode> {
    next_id: Cell<u32>,
    owner_id: Cell<u32>,
    waiting_deinit: Cell<bool>,
    inited: Cell<bool>,
    node: RefCell<Option<U>>,
}
impl<U: UiNode> RcNodeData<U> {
    pub fn new(node: Option<U>) -> Self {
        Self {
            next_id: Cell::new(1),
            owner_id: Cell::new(0),
            waiting_deinit: Cell::new(false),
            inited: Cell::new(false),
            node: RefCell::new(node),
        }
    }

    pub fn next_id(&self) -> u32 {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));
        id
    }
}

enum SlotNodeState<U: UiNode> {
    TakeOnInit(Rc<RcNodeData<U>>),
    /// Slot is not the owner of the child node.
    Inactive(Weak<RcNodeData<U>>),
    /// Slot is the next owner of the child node, awaiting previous slot deinit.
    Activating(Rc<RcNodeData<U>>),
    /// Slot is the owner of the child node.
    Active(Rc<RcNodeData<U>>),
    /// Slot deinited itself when it was the owner of the child node.
    ActiveDeinited(Rc<RcNodeData<U>>),
    /// Tried to activate but the weak reference in `Inactive` is dead.
    Dropped,
}
impl<U: UiNode> fmt::Debug for SlotNodeState<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlotNodeState::TakeOnInit(_) => write!(f, "TakeOnInit"),
            SlotNodeState::Inactive(wk) => {
                write!(f, "Inactive(can_upgrade: {})", wk.upgrade().is_some())
            }
            SlotNodeState::Activating(_) => write!(f, "Activating"),
            SlotNodeState::Active(_) => write!(f, "Active"),
            SlotNodeState::ActiveDeinited(_) => write!(f, "ActiveDeinited"),
            SlotNodeState::Dropped => write!(f, "Dropped"),
        }
    }
}

struct SlotNode<S: RcNodeTakeSignal, U: UiNode> {
    slot_id: u32,
    take_signal: S,
    event_signal: bool,
    state: SlotNodeState<U>,
}
impl<S: RcNodeTakeSignal, U: UiNode> UiNode for SlotNode<S, U> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        match &self.state {
            SlotNodeState::TakeOnInit(rc) => {
                if rc.inited.get() {
                    rc.waiting_deinit.set(true);
                    self.state = SlotNodeState::Activating(Rc::clone(rc));
                    ctx.updates.update(); // notify the other slot to deactivate.
                } else {
                    // node already free to take.
                    rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                    rc.inited.set(true);
                    rc.owner_id.set(self.slot_id);
                    self.state = SlotNodeState::Active(Rc::clone(rc));
                }
            }
            SlotNodeState::Inactive(wk) => {
                if self.take_signal.update_take(ctx) {
                    if let Some(rc) = wk.upgrade() {
                        if rc.inited.get() {
                            rc.waiting_deinit.set(true);
                            self.state = SlotNodeState::Activating(rc);
                            ctx.updates.update(); // notify the other slot to deactivate.
                        } else {
                            // node already free to take.
                            rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                            rc.inited.set(true);
                            rc.owner_id.set(self.slot_id);
                            self.state = SlotNodeState::Active(rc);
                        }
                    } else {
                        self.state = SlotNodeState::Dropped;
                    }
                }
            }
            SlotNodeState::ActiveDeinited(rc) => {
                if rc.owner_id.get() == self.slot_id {
                    // still the owner
                    assert!(!rc.inited.get());
                    assert!(!rc.waiting_deinit.get());

                    rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                    rc.inited.set(true);

                    self.state = SlotNodeState::Active(Rc::clone(rc));
                } else {
                    // TODO check signal?
                }
            }
            SlotNodeState::Activating(_) => {
                panic!("`SlotNode` in `Activating` state on init")
            }
            SlotNodeState::Active(_) => {
                panic!("`SlotNode` in `Active` state on init")
            }
            SlotNodeState::Dropped => {}
        }
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        if let SlotNodeState::Active(rc) = &self.state {
            assert!(rc.inited.take());

            rc.node.borrow_mut().as_mut().unwrap().deinit(ctx);
            rc.waiting_deinit.set(false); // just in case?

            self.state = SlotNodeState::ActiveDeinited(Rc::clone(rc));
        }
    }

    fn event<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU)
    where
        Self: Sized,
    {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().event(ctx, args);
        } else if let SlotNodeState::Inactive(_) = &self.state {
            if self.take_signal.event_take(ctx, args) {
                self.event_signal = true;
            }
        }
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        match &self.state {
            SlotNodeState::Inactive(wk) => {
                if mem::take(&mut self.event_signal) || self.take_signal.update_take(ctx) {
                    if let Some(rc) = wk.upgrade() {
                        if rc.inited.get() {
                            rc.waiting_deinit.set(true);
                            self.state = SlotNodeState::Activating(rc);
                            ctx.updates.update(); // notify the other slot to deactivate.
                        } else {
                            // node already free to take.
                            rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                            rc.inited.set(true);
                            rc.owner_id.set(self.slot_id);
                            self.state = SlotNodeState::Active(rc);
                            ctx.updates.info_layout_and_render();
                        }
                    } else {
                        self.state = SlotNodeState::Dropped
                    }
                }
            }
            SlotNodeState::Activating(rc) => {
                if !rc.inited.get() {
                    // node now free to take.
                    rc.node.borrow_mut().as_mut().unwrap().init(ctx);
                    rc.inited.set(true);
                    self.state = SlotNodeState::Active(Rc::clone(rc));
                    ctx.updates.info_layout_and_render();
                }
            }
            SlotNodeState::Active(rc) => {
                if rc.waiting_deinit.take() {
                    if rc.inited.take() {
                        rc.node.borrow_mut().as_mut().unwrap().deinit(ctx);
                    }
                    ctx.updates.update(); // notify the other slot to activate.
                    self.state = SlotNodeState::Inactive(Rc::downgrade(rc));
                    ctx.updates.info_layout_and_render();
                } else {
                    rc.node.borrow_mut().as_mut().unwrap().update(ctx);
                }
            }
            SlotNodeState::ActiveDeinited(_) => {
                panic!("`SlotNode` in `ActiveDeinited` state on update")
            }
            SlotNodeState::TakeOnInit(_) => {
                panic!("`SlotNode` in `TakeOnInit` state on update")
            }
            SlotNodeState::Dropped => {}
        }
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().measure(ctx, available_size)
        } else {
            PxSize::zero()
        }
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow_mut().as_mut().unwrap().arrange(ctx, widget_offset, final_size);
        }
    }

    fn info(&self, ctx: &mut RenderContext, info: &mut WidgetInfoBuilder) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().info(ctx, info);
        }
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().render(ctx, frame);
        }
    }

    fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        if let SlotNodeState::Active(rc) = &self.state {
            rc.node.borrow().as_ref().unwrap().render_update(ctx, update);
        }
    }
}
