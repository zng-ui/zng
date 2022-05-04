use std::fmt;

use parking_lot::Mutex;

use crate::{
    context::*,
    event::AnyEventUpdate,
    widget_info::{WidgetBorderInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutInfo, WidgetRenderInfo, WidgetSubscriptions},
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
    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions);

    /// Called every time the node is plugged in an Ui tree.
    fn init(&mut self, ctx: &mut WidgetContext);

    /// Called every time the node is unplugged from an Ui tree.
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
    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize);

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
}
#[doc(hidden)]
pub trait UiNodeBoxed: 'static {
    fn info_boxed(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);
    fn subscriptions_boxed(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions);
    fn init_boxed(&mut self, ctx: &mut WidgetContext);
    fn deinit_boxed(&mut self, ctx: &mut WidgetContext);
    fn update_boxed(&mut self, ctx: &mut WidgetContext);
    fn event_boxed(&mut self, ctx: &mut WidgetContext, args: &AnyEventUpdate);
    fn measure_boxed(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize;
    fn arrange_boxed(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize);
    fn render_boxed(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);
    fn render_update_boxed(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);
}

impl<U: UiNode> UiNodeBoxed for U {
    fn info_boxed(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.info(ctx, info);
    }

    fn subscriptions_boxed(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.subscriptions(ctx, subscriptions)
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

    fn measure_boxed(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.measure(ctx, available_size)
    }

    fn arrange_boxed(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        self.arrange(ctx, widget_layout, final_size);
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
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.as_ref().info_boxed(ctx, info);
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.as_ref().subscriptions_boxed(ctx, subscriptions);
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

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.as_mut().measure_boxed(ctx, available_size)
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        self.as_mut().arrange_boxed(ctx, widget_layout, final_size);
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
    fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        if let Some(node) = self {
            node.info(ctx, info);
        }
    }

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        if let Some(node) = self {
            node.subscriptions(ctx, subscriptions);
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

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        if let Some(node) = self {
            node.measure(ctx, available_size)
        } else {
            PxSize::zero()
        }
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        if let Some(node) = self {
            node.arrange(ctx, widget_layout, final_size);
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

    /// Outer-bounds layout information.
    ///
    /// The information is kept up-to-date, updating every arrange.
    fn outer_info(&self) -> &WidgetLayoutInfo;

    /// Inner-bounds layout information.
    ///
    /// The information is kept up-to-date, updating every arrange.
    fn inner_info(&self) -> &WidgetLayoutInfo;

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

    /// Run [`UiNode::measure`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_measure(&mut self, ctx: &mut TestWidgetContext, available_size: AvailableSize) -> PxSize {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());
        ctx.layout_context(
            font_size,
            font_size,
            self.outer_info().size(),
            1.0.fct(),
            96.0,
            LayoutMask::all(),
            |ctx| self.measure(ctx, available_size),
        )
    }
    /// Run [`UiNode::arrange`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_arrange(&mut self, ctx: &mut TestWidgetContext, final_size: PxSize) {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());

        ctx.layout_context(
            font_size,
            font_size,
            self.outer_info().size(),
            1.0.fct(),
            96.0,
            LayoutMask::all(),
            |ctx| {
                let outer = self.outer_info().clone();
                let inner = self.inner_info().clone();
                let border = self.border_info().clone();
                let id = self.id();
                WidgetLayout::with_root_widget(id, &outer, &inner, &border, final_size, |wl| {
                    self.arrange(ctx, wl, final_size);
                });
            },
        )
    }

    // TODO don't require user to init frame?

    /// Run [`UiNode::info`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_info(&self, ctx: &mut TestWidgetContext, info: &mut WidgetInfoBuilder) {
        ctx.info_context(|ctx| self.info(ctx, info));
    }

    /// Run [`UiNode::subscriptions`] using the [`TestWidgetContext`].
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_subscriptions(&self, ctx: &mut TestWidgetContext, subscriptions: &mut WidgetSubscriptions) {
        ctx.info_context(|ctx| self.subscriptions(ctx, subscriptions));
    }

    /// Run [`UiNode::render`] using the [`TestWidgetContext`].
    ///
    /// If the `frame` [`is_outer`] pushes inner, before calling render.
    ///
    /// [`is_outer`]: FrameBuilder::is_outer
    #[cfg(any(test, doc, feature = "test_util"))]
    #[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
    fn test_render(&self, ctx: &mut TestWidgetContext, frame: &mut FrameBuilder) {
        ctx.render_context(|ctx| {
            if frame.is_outer() {
                frame.push_inner(crate::render::FrameBinding::Value(RenderTransform::identity()), |frame| {
                    self.render(ctx, frame)
                })
            } else {
                self.render(ctx, frame)
            }
        });
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
    fn outer_info_boxed(&self) -> &WidgetLayoutInfo;
    fn inner_info_boxed(&self) -> &WidgetLayoutInfo;
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

    fn outer_info_boxed(&self) -> &WidgetLayoutInfo {
        self.outer_info()
    }

    fn inner_info_boxed(&self) -> &WidgetLayoutInfo {
        self.inner_info()
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

    fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.as_ref().subscriptions_boxed(ctx, subscriptions);
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

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.as_mut().measure_boxed(ctx, available_size)
    }

    fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        self.as_mut().arrange_boxed(ctx, widget_layout, final_size)
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

    fn outer_info(&self) -> &WidgetLayoutInfo {
        self.as_ref().outer_info_boxed()
    }

    fn inner_info(&self) -> &WidgetLayoutInfo {
        self.as_ref().inner_info_boxed()
    }

    fn border_info(&self) -> &WidgetBorderInfo {
        self.as_ref().border_info_boxed()
    }

    fn render_info(&self) -> &WidgetRenderInfo {
        self.as_ref().render_info_boxed()
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

// Used by #[impl_ui_node] to implement delegate_iter.
#[doc(hidden)]
pub mod impl_ui_node_util {
    use crate::{
        context::{InfoContext, LayoutContext, RenderContext, WidgetContext},
        event::EventUpdateArgs,
        render::{FrameBuilder, FrameUpdate},
        units::{AvailableSize, PxSize},
        widget_info::{WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
        UiNode,
    };

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
        fn arrange_all(self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize);
    }
    pub trait IterImpl {
        fn info_all(self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);
        fn subscriptions_all(self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions);
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

        fn arrange_all(self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
            for child in self {
                child.arrange(ctx, widget_layout, final_size);
            }
        }
    }

    impl<'u, U: UiNode, I: IntoIterator<Item = &'u U>> IterImpl for I {
        fn info_all(self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            for child in self {
                child.info(ctx, info);
            }
        }

        fn subscriptions_all(self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            for child in self {
                child.subscriptions(ctx, subscriptions);
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
