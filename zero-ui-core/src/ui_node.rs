use std::fmt;

use parking_lot::Mutex;

use crate::{
    context::*,
    event::AnyEventUpdate,
    widget_info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetRenderInfo, WidgetSubscriptions},
    IdNameError,
};
use crate::{crate_util::NameIdMap, units::*};
use crate::{
    event::EventUpdateArgs,
    impl_ui_node,
    render::{FrameBuilder, FrameUpdate},
};

mod rc_node;
pub use rc_node::*;

mod instrument;
pub use instrument::InstrumentedNode;

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
            write!(f, "WidgetId({name:?})")
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

crate::var::impl_from_and_into_var! {
    /// Calls [`WidgetId::named`].
    fn from(name: &'static str) -> WidgetId {
        WidgetId::named(name)
    }
}

/// An Ui tree node.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait UiNode: 'static {
    /// Called every time there are structural changes in the UI tree such as a node added or removed.
    ///
    /// # Arguments
    ///
    /// * `ctx`: Limited context access.
    /// * `info`: Widget info tree builder.
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);

    /// Called every time the set of variables and events monitored by the widget changes.
    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions);

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

    /// Called every time an event updates.
    ///
    /// Every call to this method is for a single update of a single event type, you can listen to events
    /// using the [`Event::update`] method. This method is called even if [`stop_propagation`]
    /// was requested, or the widget is disabled, and it must always propagate to descendent nodes.
    ///
    /// Event propagation can be statically or dynamically typed, the way to listen to both is the same, `A` can be an
    /// [`AnyEventUpdate`] instance that is resolved dynamically or an [`EventUpdate`] instance
    /// that is resolved statically in the [`Event::update`] call. If an event matches you should **use the returned args**
    /// in the call the descendant nodes, **this upgrades from dynamic to static resolution** in descendant nodes increasing
    /// performance.
    ///
    /// If the event is handled before the call in descendant nodes it is called a *preview*, this behavior matches what
    /// happens in the [`on_pre_event`] node.
    ///
    /// # Examples
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
    ///             if args.concerns_widget(ctx) && !args.stop_propagation_requested() {
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
    /// the widget was clicked and stop propagation was not requested, the click arguments `concerns_widget` also
    /// checks if the widget is enabled. The event is then propagated to the `child` node, `self.child.event` appears to be
    /// duplicated, but the call inside the `if` block ensured that the descendant nodes will resolve the event statically,
    /// which may not be the case in the `else` block call where `A` can be the dynamic resolver.
    ///
    /// [`stop_propagation`]: crate::event::EventArgs::stop_propagation
    /// [`EventUpdate`]: crate::event::EventUpdate
    /// [`on_pre_event`]: crate::event::on_pre_event
    /// [`Event::update`]: crate::event::Event::update
    fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A);

    /// Called every time an update is requested.
    ///
    /// An update happens every time after a sequence of [`event`](Self::event), they also happen
    /// when variables update and any other context or service structure that can be observed updates.
    fn update(&mut self, ctx: &mut WidgetContext);

    /// Called every time a layout update is requested.
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

    /// Gets if this node is a [`Widget`] implementer.
    fn is_widget(&self) -> bool {
        false
    }

    /// Gets the [`Widget::id`] if this node [`is_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn try_id(&self) -> Option<WidgetId> {
        None
    }

    /// Gets the [`Widget::state`] if this node [`is_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn try_state(&self) -> Option<&StateMap> {
        None
    }

    /// Gets the [`Widget::state_mut`] if this node [`is_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn try_state_mut(&mut self) -> Option<&mut StateMap> {
        None
    }

    /// Gets the [`Widget::bounds_info`] if this node [`is_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn try_bounds_info(&self) -> Option<&WidgetBoundsInfo> {
        None
    }

    /// Gets the [`Widget::border_info`] if this node [`is_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn try_border_info(&self) -> Option<&WidgetBorderInfo> {
        None
    }

    /// Gets the [`Widget::render_info`] if this node [`is_widget`].
    ///
    /// [`is_widget`]: UiNode::is_widget
    fn try_render_info(&self) -> Option<&WidgetRenderInfo> {
        None
    }

    /// Gets this node as a [`BoxedWidget`], if the node [`is_widget`] this is the same as
    /// [`Widget::boxed_wgt`], otherwise a new widget is generated with the node as the *inner*.
    ///
    /// [`is_widget`]: Self::is_widget
    fn into_widget(self) -> BoxedWidget
    where
        Self: Sized,
    {
        use crate::widget_base::implicit_base::nodes;

        let node = nodes::inner(self.cfg_boxed());
        let wgt = nodes::widget(node, WidgetId::new_unique());
        wgt.boxed_wgt()
    }
}

#[doc(hidden)]
pub trait UiNodeBoxed: 'static {
    fn info_boxed(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);
    fn subscriptions_boxed(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions);
    fn init_boxed(&mut self, ctx: &mut WidgetContext);
    fn deinit_boxed(&mut self, ctx: &mut WidgetContext);
    fn update_boxed(&mut self, ctx: &mut WidgetContext);
    fn event_boxed(&mut self, ctx: &mut WidgetContext, args: &AnyEventUpdate);
    fn layout_boxed(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize;
    fn render_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
    fn render_update_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    fn is_widget_boxed(&self) -> bool;
    fn try_id_boxed(&self) -> Option<WidgetId>;
    fn try_state_boxed(&self) -> Option<&StateMap>;
    fn try_state_mut_boxed(&mut self) -> Option<&mut StateMap>;
    fn try_bounds_info_boxed(&self) -> Option<&WidgetBoundsInfo>;
    fn try_border_info_boxed(&self) -> Option<&WidgetBorderInfo>;
    fn try_render_info_boxed(&self) -> Option<&WidgetRenderInfo>;
    fn into_widget_boxed(self: Box<Self>) -> BoxedWidget;
}

impl<U: UiNode> UiNodeBoxed for U {
    fn info_boxed(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.info(ctx, info);
    }

    fn subscriptions_boxed(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        self.subscriptions(ctx, subs)
    }

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

    fn try_id_boxed(&self) -> Option<WidgetId> {
        self.try_id()
    }

    fn try_state_boxed(&self) -> Option<&StateMap> {
        self.try_state()
    }

    fn try_state_mut_boxed(&mut self) -> Option<&mut StateMap> {
        self.try_state_mut()
    }

    fn try_bounds_info_boxed(&self) -> Option<&WidgetBoundsInfo> {
        self.try_bounds_info()
    }

    fn try_border_info_boxed(&self) -> Option<&WidgetBorderInfo> {
        self.try_border_info()
    }

    fn try_render_info_boxed(&self) -> Option<&WidgetRenderInfo> {
        self.try_render_info()
    }

    fn into_widget_boxed(self: Box<Self>) -> BoxedWidget {
        self.into_widget()
    }
}

/// An [`UiNode`] in a box.
pub type BoxedUiNode = Box<dyn UiNodeBoxed>;

impl UiNode for BoxedUiNode {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.as_ref().info_boxed(ctx, info);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        self.as_ref().subscriptions_boxed(ctx, subs);
    }

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

    fn is_widget(&self) -> bool {
        self.as_ref().is_widget_boxed()
    }

    fn try_id(&self) -> Option<WidgetId> {
        self.as_ref().try_id_boxed()
    }

    fn try_state(&self) -> Option<&StateMap> {
        self.as_ref().try_state_boxed()
    }

    fn try_state_mut(&mut self) -> Option<&mut StateMap> {
        self.as_mut().try_state_mut_boxed()
    }

    fn try_bounds_info(&self) -> Option<&WidgetBoundsInfo> {
        self.as_ref().try_bounds_info_boxed()
    }

    fn try_border_info(&self) -> Option<&WidgetBorderInfo> {
        self.as_ref().try_border_info_boxed()
    }

    fn try_render_info(&self) -> Option<&WidgetRenderInfo> {
        self.as_ref().try_render_info_boxed()
    }

    fn into_widget(self) -> BoxedWidget
    where
        Self: Sized,
    {
        self.into_widget_boxed()
    }
}

impl<U: UiNode> UiNode for Option<U> {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        if let Some(node) = self {
            node.info(ctx, info);
        }
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        if let Some(node) = self {
            node.subscriptions(ctx, subs);
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

    fn try_id(&self) -> Option<WidgetId> {
        match self {
            Some(node) => node.try_id(),
            None => None,
        }
    }

    fn try_state(&self) -> Option<&StateMap> {
        match self {
            Some(node) => node.try_state(),
            None => None,
        }
    }

    fn try_state_mut(&mut self) -> Option<&mut StateMap> {
        match self {
            Some(node) => node.try_state_mut(),
            None => None,
        }
    }

    fn try_bounds_info(&self) -> Option<&WidgetBoundsInfo> {
        match self {
            Some(node) => node.try_bounds_info(),
            None => None,
        }
    }

    fn try_border_info(&self) -> Option<&WidgetBorderInfo> {
        match self {
            Some(node) => node.try_border_info(),
            None => None,
        }
    }

    fn try_render_info(&self) -> Option<&WidgetRenderInfo> {
        match self {
            Some(node) => node.try_render_info(),
            None => None,
        }
    }

    fn into_widget(self) -> BoxedWidget
    where
        Self: Sized,
    {
        match self {
            Some(node) => node.into_widget(),
            None => NilUiNode.into_widget(),
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

    /// Bounds layout information.
    ///
    /// The information is kept up-to-date, updating every arrange.
    fn bounds_info(&self) -> &WidgetBoundsInfo;

    /// Border and corner radius information.
    ///
    /// The information is kept up-to-date, updating every arrange.
    fn border_info(&self) -> &WidgetBorderInfo;

    /// Render information.
    ///
    /// The information is kept up-to-date, updating every render
    fn render_info(&self) -> &WidgetRenderInfo;

    /// Box this widget node, unless it is already `BoxedWidget`.
    fn boxed_wgt(self) -> BoxedWidget
    where
        Self: Sized,
    {
        Box::new(self)
    }

    /// Helper for complying with the `dyn_widget` feature, boxes the widget or just returns it depending of the
    /// compile time feature.
    ///
    /// If the `dyn_widget` feature is enabled widget should be nested using [`BoxedUiNode`] instead of
    /// generating a new type. The `#[widget(..)]` attribute macro auto-implements this for widget new functions,
    /// other functions in the format `fn() -> impl Widget` can use this method to achieve the same.
    #[cfg(dyn_widget)]
    fn cfg_boxed_wgt(self) -> BoxedWidget
    where
        Self: Sized,
    {
        self.boxed_wgt()
    }

    /// Helper for complying with the `dyn_widget` feature, boxes the widget or just returns it depending of the
    /// compile time feature.
    ///
    /// If the `dyn_widget` feature is enabled widget should be nested using [`BoxedUiNode`] instead of
    /// generating a new type. The `#[widget(..)]` attribute macro auto-implements this for widget new functions,
    /// other functions in the format `fn() -> impl Widget` can use this method to achieve the same.
    #[cfg(not(dyn_widget))]
    fn cfg_boxed_wgt(self) -> Self
    where
        Self: Sized,
    {
        self
    }

    declare_widget_test_calls! {
        init, deinit, update
    }

    /// Run [`UiNode::layout`] using the [`TestWidgetContext`].
    ///
    /// If `constrains` is set it is used for the layout context.
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_layout(&mut self, ctx: &mut TestWidgetContext, constrains: Option<PxConstrains2d>) -> PxSize {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());
        ctx.layout_context(
            font_size,
            font_size,
            self.bounds_info().outer_size(),
            1.0.fct(),
            96.0,
            LayoutMask::all(),
            |ctx| {
                ctx.with_constrains(
                    |c| constrains.unwrap_or(c),
                    |ctx| WidgetLayout::with_root_widget(ctx, |ctx, wl| wl.with_inner(ctx, |ctx, wl| self.layout(ctx, wl))),
                )
            },
        )
    }

    /// Run [`UiNode::info`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_info(&self, ctx: &mut TestWidgetContext, info: &mut WidgetInfoBuilder) {
        ctx.info_context(|ctx| self.info(ctx, info));
    }

    /// Run [`UiNode::subscriptions`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_subscriptions(&self, ctx: &mut TestWidgetContext, subs: &mut WidgetSubscriptions) {
        ctx.info_context(|ctx| self.subscriptions(ctx, subs));
    }

    /// Run [`UiNode::render`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_render(&self, ctx: &mut TestWidgetContext, frame: &mut FrameBuilder) {
        let key = ctx.root_translation_key;
        ctx.render_context(|ctx| frame.push_inner(ctx, key, |ctx, frame| self.render(ctx, frame)));
    }

    /// Run [`UiNode::render_update`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_render_update(&self, ctx: &mut TestWidgetContext, update: &mut FrameUpdate) {
        let key = ctx.root_translation_key;
        ctx.render_context(|ctx| update.update_inner(ctx, key, |ctx, update| self.render_update(ctx, update)));
    }

    /// Run [`UiNode::event`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_event<A: EventUpdateArgs>(&mut self, ctx: &mut TestWidgetContext, args: &A) {
        ctx.widget_context(|ctx| self.event(ctx, args))
    }
}

#[doc(hidden)]
pub trait WidgetBoxed: UiNodeBoxed {
    fn id_boxed(&self) -> WidgetId;
    fn state_boxed(&self) -> &StateMap;
    fn state_mut_boxed(&mut self) -> &mut StateMap;
    fn bounds_info_boxed(&self) -> &WidgetBoundsInfo;
    fn border_info_boxed(&self) -> &WidgetBorderInfo;
    fn render_info_boxed(&self) -> &WidgetRenderInfo;
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

    fn bounds_info_boxed(&self) -> &WidgetBoundsInfo {
        self.bounds_info()
    }

    fn border_info_boxed(&self) -> &WidgetBorderInfo {
        self.border_info()
    }

    fn render_info_boxed(&self) -> &WidgetRenderInfo {
        self.render_info()
    }
}

/// An [`Widget`] in a box.
pub type BoxedWidget = Box<dyn WidgetBoxed>;

impl UiNode for BoxedWidget {
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.as_ref().info_boxed(ctx, info);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
        self.as_ref().subscriptions_boxed(ctx, subs);
    }

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
        Box::new(self)
    }

    fn is_widget(&self) -> bool {
        true
    }

    fn try_id(&self) -> Option<WidgetId> {
        Some(self.id())
    }

    fn try_state(&self) -> Option<&StateMap> {
        Some(self.state())
    }

    fn try_state_mut(&mut self) -> Option<&mut StateMap> {
        Some(self.state_mut())
    }

    fn try_bounds_info(&self) -> Option<&WidgetBoundsInfo> {
        Some(self.bounds_info())
    }

    fn try_border_info(&self) -> Option<&WidgetBorderInfo> {
        Some(self.border_info())
    }

    fn try_render_info(&self) -> Option<&WidgetRenderInfo> {
        Some(self.render_info())
    }

    fn into_widget(self) -> BoxedWidget
    where
        Self: Sized,
    {
        self
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

    fn bounds_info(&self) -> &WidgetBoundsInfo {
        self.as_ref().bounds_info_boxed()
    }

    fn border_info(&self) -> &WidgetBorderInfo {
        self.as_ref().border_info_boxed()
    }

    fn render_info(&self) -> &WidgetRenderInfo {
        self.as_ref().render_info_boxed()
    }

    fn boxed_wgt(self) -> BoxedWidget
    where
        Self: Sized,
    {
        self
    }
}

/// A UI node that does not contain any other node, only takes the minimum space and renders nothing.
pub struct NilUiNode;
#[impl_ui_node(none)]
impl UiNode for NilUiNode {
    fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
        ctx.constrains().min_size()
    }
}

/// A UI node that does not contain any other node, fills the available space, but renders nothing.
pub struct FillUiNode;
#[impl_ui_node(none)]
impl UiNode for FillUiNode {}

// Used by #[impl_ui_node] to implement delegate_iter.
#[doc(hidden)]
pub mod impl_ui_node_util {
    use crate::{
        context::{InfoContext, LayoutContext, RenderContext, WidgetContext},
        event::EventUpdateArgs,
        render::{FrameBuilder, FrameUpdate},
        units::PxSize,
        widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
        UiNode,
    };

    pub fn delegate_iter<'a>(d: impl IntoIterator<Item = &'a impl UiNode>) -> impl IterImpl {
        d
    }

    pub fn delegate_iter_mut<'a>(d: impl IntoIterator<Item = &'a mut impl UiNode>) -> impl IterMutImpl {
        d
    }

    pub trait IterMutImpl {
        fn init_all(self, ctx: &mut WidgetContext);
        fn deinit_all(self, ctx: &mut WidgetContext);
        fn update_all(self, ctx: &mut WidgetContext);
        fn event_all<EU: EventUpdateArgs>(self, ctx: &mut WidgetContext, args: &EU);
        fn layout_all(self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize;
    }
    pub trait IterImpl {
        fn info_all(self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);
        fn subscriptions_all(self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions);
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

        fn layout_all(self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let mut size = PxSize::zero();
            for child in self {
                size = child.layout(ctx, wl).max(size);
            }
            size
        }
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u U>> IterImpl for I {
        fn info_all(self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            for child in self {
                child.info(ctx, info);
            }
        }

        fn subscriptions_all(self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            for child in self {
                child.subscriptions(ctx, subs);
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
