//! The [`WidgetBase`], nodes and properties used in most widgets.

use std::{any::TypeId, cell::RefCell, fmt};

use crate::{
    context::*,
    event::EventUpdate,
    property,
    render::{FrameBuilder, FrameUpdate, FrameValueKey},
    ui_node,
    units::{PxCornerRadius, PxRect, PxSize, PxTransform},
    var::*,
    widget_builder::*,
    widget_info::*,
    widget_instance::*,
    window::INTERACTIVITY_CHANGED_EVENT,
};

/// Base widget that implements the necessary core API.
///
/// The base widget does [`nodes::include_intrinsics`] to enable proper layout and render in all widgets that inherit from base.
///
/// The base widget also provides a default function that captures the [`id`] and handles missing child node by capturing
/// [`child`] or falling back to [`FillUiNode`].
///
/// [`id`]: WidgetBase::id
/// [`child`]: fn@child
// #[widget($crate::widget_base::WidgetBase)]
pub struct WidgetBase {
    builder: RefCell<Option<WidgetBuilder>>,
    started: bool,
    importance: Importance,
    when: RefCell<Option<WhenInfo>>,
}
impl WidgetBase {
    /// Gets the type of [`WidgetBase`].
    pub fn widget_type() -> WidgetType {
        WidgetType {
            type_id: TypeId::of::<Self>(),
            path: "$crate::widget_base::WidgetBase",
            location: source_location!(),
        }
    }

    /// Starts building a new [`WidgetBase`] instance.
    pub fn start() -> Self {
        Self::inherit(Self::widget_type())
    }

    /// Starts building a new widget derived from [`WidgetBase`].
    pub fn inherit(widget: WidgetType) -> Self {
        let builder = WidgetBuilder::new(widget);
        let mut w = Self {
            builder: RefCell::new(Some(builder)),
            started: false,
            importance: Importance::INSTANCE,
            when: RefCell::new(None),
        };
        w.on_start__();
        w
    }

    /// Direct reference the widget builder.
    pub fn builder(&mut self) -> &mut WidgetBuilder {
        self.builder.get_mut().as_mut().expect("already built")
    }

    /// Direct reference the current `when` block.
    pub fn when(&mut self) -> Option<&mut WhenInfo> {
        self.when.get_mut().as_mut()
    }

    /// Gets the widget builder.
    ///
    /// After this call trying to set a property will panic.
    pub fn take_builder(&mut self) -> WidgetBuilder {
        assert!(self.when.get_mut().is_none(), "cannot take builder with `when` pending");
        self.builder.get_mut().take().expect("builder already taken")
    }

    /// Build the widget.
    ///
    /// After this call trying to set a property will panic.
    pub fn build(&mut self) -> impl UiNode {
        let mut wgt = self.take_builder();
        wgt.push_build_action(|wgt| {
            if !wgt.has_child() {
                wgt.set_child(FillUiNode);
            }
        });
        nodes::build(wgt)
    }

    /// Properties, unsets and when conditions set after this call will have [`Importance::WIDGET`].
    pub fn start_defaults(&mut self) {
        self.importance = Importance::WIDGET;
    }

    /// Properties, unsets and when conditions set after this call will have [`Importance::INSTANCE`].
    pub fn end_defaults(&mut self) {
        self.importance = Importance::INSTANCE;
    }

    /// Start building a `when` block, all properties set after this call go on the when block.
    pub fn start_when_block(&mut self, inputs: Box<[WhenInput]>, state: BoxedVar<bool>, expr: &'static str, location: SourceLocation) {
        assert!(self.builder.get_mut().is_some(), "cannot start `when` after build");
        assert!(self.when.get_mut().is_none(), "cannot nest `when` blocks");

        *self.when.get_mut() = Some(WhenInfo {
            inputs,
            state,
            assigns: vec![],
            build_action_data: vec![],
            expr,
            location,
        });
    }

    /// End the current `when` block, all properties set after this call go on the widget.
    pub fn end_when_block(&mut self) {
        let when = self.when.get_mut().take().expect("no current `when` block to end");
        self.builder.get_mut().as_mut().unwrap().push_when(self.importance, when);
    }

    #[doc(hidden)]
    pub fn on_start__(&mut self) {
        if !self.started {
            self.started = true;
            nodes::include_intrinsics(self.builder());
        }
    }

    /// Push method property.
    #[doc(hidden)]
    pub fn mtd_property__(&self, args: Box<dyn PropertyArgs>) {
        if let Some(when) = &mut *self.when.borrow_mut() {
            when.assigns.push(args);
        } else {
            self.builder
                .borrow_mut()
                .as_mut()
                .expect("cannot set after build")
                .push_property(self.importance, args);
        }
    }

    /// Push method unset property.
    #[doc(hidden)]
    pub fn mtd_property_unset__(&self, id: PropertyId) {
        assert!(self.when.borrow().is_none(), "cannot unset in when assign");
        self.builder
            .borrow_mut()
            .as_mut()
            .expect("cannot unset after build")
            .push_unset(self.importance, id);
    }

    #[doc(hidden)]
    pub fn reexport__(&self, f: impl FnOnce(&mut Self)) {
        let mut inner = Self {
            builder: RefCell::new(self.builder.borrow_mut().take()),
            started: self.started,
            importance: self.importance,
            when: RefCell::new(self.when.borrow_mut().take()),
        };
        f(&mut inner);
        *self.builder.borrow_mut() = inner.builder.into_inner().take();
        *self.when.borrow_mut() = inner.when.into_inner().take();
        debug_assert_eq!(self.started, inner.started);
        debug_assert_eq!(self.importance, inner.importance);
    }

    #[doc(hidden)]
    pub fn push_unset_property_build_action__(&mut self, property_id: PropertyId, action_name: &'static str) {
        assert!(self.when.get_mut().is_none(), "cannot unset build actions in when assigns");

        self.builder
            .get_mut()
            .as_mut()
            .expect("cannot unset build actions after build")
            .push_unset_property_build_action(property_id, action_name, self.importance);
    }

    #[doc(hidden)]
    pub fn push_property_build_action__(
        &mut self,
        property_id: PropertyId,
        action_name: &'static str,
        input_actions: Vec<Box<dyn AnyPropertyBuildAction>>,
    ) {
        assert!(
            self.when.get_mut().is_none(),
            "cannot push property build action in `when`, use `push_when_build_action_data__`"
        );

        self.builder
            .get_mut()
            .as_mut()
            .expect("cannot unset build actions after build")
            .push_property_build_action(property_id, action_name, self.importance, input_actions);
    }

    #[doc(hidden)]
    pub fn push_when_build_action_data__(&mut self, property_id: PropertyId, action_name: &'static str, data: WhenBuildAction) {
        let when = self
            .when
            .get_mut()
            .as_mut()
            .expect("cannot push when build action data outside when blocks");
        when.build_action_data.push(((property_id, action_name), data));
    }
}

/// Trait implemented by all `#[widget]`.
pub trait WidgetImpl {
    /// The inherit function.
    fn inherit(widget: WidgetType) -> Self;

    /// Reference the parent [`WidgetBase`].
    fn base(&mut self) -> &mut WidgetBase;

    #[doc(hidden)]
    fn base_ref(&self) -> &WidgetBase;

    #[doc(hidden)]
    fn info_instance__() -> Self;
}
impl WidgetImpl for WidgetBase {
    fn inherit(widget: WidgetType) -> Self {
        Self::inherit(widget)
    }

    fn base(&mut self) -> &mut WidgetBase {
        self
    }

    fn base_ref(&self) -> &WidgetBase {
        self
    }

    fn info_instance__() -> Self {
        WidgetBase {
            builder: RefCell::new(None),
            started: false,
            importance: Importance::INSTANCE,
            when: RefCell::new(None),
        }
    }
}

#[doc(hidden)]
pub trait WidgetExt {
    #[doc(hidden)]
    fn ext_property__(&mut self, args: Box<dyn PropertyArgs>);
    #[doc(hidden)]
    fn ext_property_unset__(&mut self, id: PropertyId);
}
impl WidgetExt for WidgetBase {
    fn ext_property__(&mut self, args: Box<dyn PropertyArgs>) {
        if let Some(when) = self.when.get_mut() {
            when.assigns.push(args);
        } else {
            self.builder
                .get_mut()
                .as_mut()
                .expect("cannot set after build")
                .push_property(self.importance, args);
        }
    }

    fn ext_property_unset__(&mut self, id: PropertyId) {
        assert!(self.when.get_mut().is_none(), "cannot unset in when blocks");

        self.builder
            .get_mut()
            .as_mut()
            .expect("cannot unset after build")
            .push_unset(self.importance, id);
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! WidgetBaseMacro__ {
    ($($tt:tt)*) => {
        $crate::widget_new! {
            start { $crate::widget_base::WidgetBase::start() }
            end { wgt__.build() }
            new { $($tt)* }
        }
    }
}
#[doc(hidden)]
pub use WidgetBaseMacro__ as WidgetBase;

/// Basic nodes for widgets, some used in [`WidgetBase`].
///
/// [`WidgetBase`]: struct@WidgetBase
pub mod nodes {
    use super::*;

    /// Insert [`widget_child`] and [`widget_inner`] in the widget.
    pub fn include_intrinsics(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            wgt.push_intrinsic(NestGroup::CHILD, "widget_child", nodes::widget_child);
            wgt.push_intrinsic(NestGroup::BORDER, "widget_inner", nodes::widget_inner);
        });
    }

    /// Capture the [`id`] property and builds the base widget.
    ///
    /// Note that this function does not handle missing child node, it falls back to [`NilUiNode`]. The [`base`]
    /// widget uses the [`FillUiNode`] if none was set.
    ///
    /// [`base`]: mod@base
    /// [`id`]: fn@id
    pub fn build(mut wgt: WidgetBuilder) -> impl UiNode {
        let id = wgt.capture_value_or_else(property_id!(id), WidgetId::new_unique);
        let child = wgt.build();
        nodes::widget(child, id)
    }

    /// Returns a node that wraps `child` and potentially applies child transforms if the `child` turns out
    /// to not be a full widget or to be multiple children. This is important for making properties like *padding* or *content_align* work
    /// for any [`UiNode`] as content.
    ///
    /// This node also pass through the `child` inline layout return info if the widget and child are inlining and the
    /// widget has not set inline info before delegating measure.
    ///
    /// This node must be intrinsic at [`NestGroup::CHILD`], the [`base`] default intrinsic inserts it.
    ///
    /// [`base`]: mod@base
    pub fn widget_child(child: impl UiNode) -> impl UiNode {
        #[ui_node(struct WidgetChildNode {
                child: impl UiNode,
                key: FrameValueKey<PxTransform>,
                define_ref_frame: bool,
            })]
        impl UiNode for WidgetChildNode {
            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                let desired_size = self.child.measure(wm);

                if let Some(inline) = wm.inline() {
                    if inline.is_default() {
                        if let Some(child_inline) = self.child.with_context(|| WIDGET.bounds().measure_inline()).flatten() {
                            // pass through child inline
                            *inline = child_inline;
                        }
                    }
                }

                desired_size
            }
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                let (size, define_ref_frame) = wl.with_child(|wl| self.child.layout(wl));

                if self.define_ref_frame != define_ref_frame {
                    self.define_ref_frame = define_ref_frame;
                    WIDGET.render();
                }

                if !define_ref_frame {
                    // child maybe widget, try to copy inline
                    if let Some(inline) = wl.inline() {
                        if inline.is_default() {
                            self.child.with_context(|| {
                                let bounds = WIDGET.bounds();
                                let child_inline = bounds.inline();
                                if let Some(child_inline) = child_inline {
                                    inline.clone_from(&*child_inline);
                                }
                            });
                        }
                    }
                }

                size
            }
            fn render(&self, frame: &mut FrameBuilder) {
                let offset = WIDGET.bounds().child_offset();
                if self.define_ref_frame {
                    frame.push_reference_frame(self.key.into(), self.key.bind(offset.into(), true), true, false, |frame| {
                        self.child.render(frame)
                    });
                } else {
                    frame.push_child(offset, |frame| {
                        self.child.render(frame);
                    });
                }
            }

            fn render_update(&self, update: &mut FrameUpdate) {
                let offset = WIDGET.bounds().child_offset();
                if self.define_ref_frame {
                    update.with_transform(self.key.update(offset.into(), true), false, |update| {
                        self.child.render_update(update)
                    });
                } else {
                    update.with_child(offset, |update| self.child.render_update(update))
                }
            }
        }
        WidgetChildNode {
            child: child.cfg_boxed(),
            key: FrameValueKey::new_unique(),
            define_ref_frame: false,
        }
        .cfg_boxed()
    }

    /// Returns a node that wraps `child` and marks the [`WidgetLayout::with_inner`] and [`FrameBuilder::push_inner`].
    ///
    /// This node renders the inner transform and implements the [`HitTestMode`] for the widget.
    ///
    /// This node must be intrinsic at [`NestGroup::BORDER`], the [`base`] default intrinsic inserts it.
    ///
    /// [`base`]: mod@base
    pub fn widget_inner(child: impl UiNode) -> impl UiNode {
        #[derive(Default, PartialEq)]
        struct HitClips {
            bounds: PxSize,
            corners: PxCornerRadius,
        }
        #[ui_node(struct WidgetInnerNode {
            child: impl UiNode,
            transform_key: FrameValueKey<PxTransform>,
            clips: HitClips,
        })]
        impl UiNode for WidgetInnerNode {
            fn init(&mut self) {
                WIDGET.sub_var(&HitTestMode::var());
                self.child.init();
            }

            fn update(&mut self, updates: &WidgetUpdates) {
                if HitTestMode::var().is_new() {
                    WIDGET.layout();
                }
                self.child.update(updates);
            }

            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                self.child.measure(wm)
            }
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                let size = wl.with_inner(|wl| self.child.layout(wl));

                let mode = HitTestMode::var().get();
                let clips = if matches!(mode, HitTestMode::Bounds | HitTestMode::RoundedBounds) {
                    HitClips {
                        bounds: size,
                        corners: if matches!(mode, HitTestMode::RoundedBounds) {
                            WIDGET.border().corner_radius()
                        } else {
                            PxCornerRadius::zero()
                        },
                    }
                } else {
                    HitClips::default()
                };

                if clips != self.clips {
                    self.clips = clips;
                    WIDGET.render();
                }

                size
            }
            fn render(&self, frame: &mut FrameBuilder) {
                frame.push_inner(self.transform_key, true, |frame| {
                    frame.hit_test().push_clips(
                        |c| {
                            if let Some(inline) = WIDGET.bounds().inline() {
                                for r in inline.negative_space().iter() {
                                    c.push_clip_rect(*r, true);
                                }
                            }
                        },
                        |h| match HitTestMode::var().get() {
                            HitTestMode::RoundedBounds => {
                                h.push_rounded_rect(PxRect::from_size(self.clips.bounds), self.clips.corners);
                            }
                            HitTestMode::Bounds => {
                                h.push_rect(PxRect::from_size(self.clips.bounds));
                            }
                            _ => {}
                        },
                    );

                    self.child.render(frame);
                });
            }
            fn render_update(&self, update: &mut FrameUpdate) {
                update.update_inner(self.transform_key, true, |update| self.child.render_update(update));
            }
        }
        WidgetInnerNode {
            child: child.cfg_boxed(),
            transform_key: FrameValueKey::new_unique(),
            clips: HitClips::default(),
        }
        .cfg_boxed()
    }

    /// Create a widget node that wraps `child` and introduces a new widget context. The node defines
    /// an [`WIDGET`] context and implements the widget in each specific node method.
    ///
    /// This node must wrap the outer-most context node in the build, it is the [`base`] widget type.
    ///
    /// [`base`]: mod@base
    pub fn widget(child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl UiNode {
        struct WidgetNode<C> {
            ctx: WidgetCtx,
            child: C,

            #[cfg(debug_assertions)]
            inited: bool,
        }
        impl<C: UiNode> UiNode for WidgetNode<C> {
            fn init(&mut self) {
                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::init` called in inited widget {:?}", WIDGET.id());
                    }

                    self.child.init();
                    WIDGET.update_info().layout().render();

                    #[cfg(debug_assertions)]
                    {
                        self.inited = true;
                    }
                });
                self.ctx.take_reinit(); // ignore reinit request
            }

            fn deinit(&mut self) {
                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", WIDGET.id());
                    }

                    self.child.deinit();
                    WIDGET.update_info().layout().render();

                    #[cfg(debug_assertions)]
                    {
                        self.inited = false;
                    }
                });
                self.ctx.deinit();
            }

            fn info(&self, info: &mut WidgetInfoBuilder) {
                let rebuild = self.ctx.take_info();
                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::info` called in not inited widget {:?}", WIDGET.id());
                    }

                    if rebuild {
                        info.push_widget(WIDGET.id(), WIDGET.bounds(), WIDGET.border(), |info| {
                            self.child.info(info);
                        });
                    } else {
                        info.push_widget_reuse();
                    }
                });

                if self.ctx.is_pending_reinit() {
                    UPDATES.update(self.ctx.id());
                }
            }

            fn event(&mut self, update: &EventUpdate) {
                if self.ctx.take_reinit() {
                    self.deinit();
                    self.init();
                }

                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", update.event().name(), WIDGET.id());
                    }

                    update.with_widget(|| {
                        self.child.event(update);
                    });
                });

                if self.ctx.take_reinit() {
                    self.deinit();
                    self.init();
                }
            }

            fn update(&mut self, updates: &WidgetUpdates) {
                if self.ctx.take_reinit() {
                    self.deinit();
                    self.init();
                    return;
                }

                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::update` called in not inited widget {:?}", WIDGET.id());
                    }

                    updates.with_widget(|| {
                        self.child.update(updates);
                    });
                });

                if self.ctx.take_reinit() {
                    self.deinit();
                    self.init();
                }
            }

            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                let reuse = !self.ctx.is_pending_layout();
                let desired_size = WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::measure` called in not inited widget {:?}", WIDGET.id());
                    }

                    wm.with_widget(reuse, |wm| {
                        let child_size = self.child.measure(wm);

                        // verify that inline row segments fit in row size
                        #[cfg(debug_assertions)]
                        if let Some(inline) = wm.inline() {
                            for (name, size, segs) in [
                                ("first", inline.first, &inline.first_segs),
                                ("last", inline.last, &inline.last_segs),
                            ] {
                                let width = size.width.0 as f32;
                                let sum_width = segs.iter().map(|s| s.width).sum::<f32>();
                                if sum_width > width {
                                    tracing::error!(
                                        "widget {:?} measured inline {name} row has {width} width, but row segs sum to {sum_width} width",
                                        WIDGET.id()
                                    );
                                }
                            }
                        }

                        child_size
                    })
                });

                if self.ctx.is_pending_reinit() {
                    UPDATES.update(self.ctx.id());
                }

                desired_size
            }

            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                let reuse = !self.ctx.take_layout();
                let final_size = WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::layout` called in not inited widget {:?}", WIDGET.id());
                    }

                    wl.with_widget(reuse, |wl| {
                        let child_size = self.child.layout(wl);

                        // verify that inline row segments fit in row rectangle
                        #[cfg(debug_assertions)]
                        if let Some(inline) = wl.inline() {
                            for (name, row, segs) in inline
                                .rows
                                .first()
                                .iter()
                                .map(|r| ("first", r, &inline.first_segs))
                                .chain(inline.rows.last().iter().map(|r| ("last", r, &inline.last_segs)))
                            {
                                let width = row.width();
                                let sum_width = segs.iter().map(|s| s.width).sum::<crate::units::Px>();
                                if (sum_width - width) > crate::units::Px(1) {
                                    tracing::error!(
                                        "widget {:?} layout inline {name} row has {width} width, but row segs widths sum to {sum_width}",
                                        WIDGET.id()
                                    );
                                }
                            }
                        }

                        child_size
                    })
                });

                if self.ctx.is_pending_reinit() {
                    UPDATES.update(self.ctx.id());
                }

                final_size
            }

            fn render(&self, frame: &mut FrameBuilder) {
                let mut reuse = self.ctx.take_render();
                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::render` called in not inited widget {:?}", WIDGET.id());
                    }

                    frame.push_widget(&mut reuse, |frame| self.child.render(frame));
                });
                self.ctx.set_render_reuse(reuse);

                if self.ctx.is_pending_reinit() {
                    UPDATES.update(self.ctx.id());
                }
            }

            fn render_update(&self, update: &mut FrameUpdate) {
                let reuse = !self.ctx.take_render_update();

                WIDGET.with_context(&self.ctx, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", WIDGET.id());
                    }

                    update.update_widget(reuse, |update| self.child.render_update(update));
                });

                if self.ctx.is_pending_reinit() {
                    UPDATES.update(self.ctx.id());
                }
            }

            fn is_widget(&self) -> bool {
                true
            }

            fn with_context<R, F>(&self, f: F) -> Option<R>
            where
                F: FnOnce() -> R,
            {
                WIDGET.with_context(&self.ctx, || Some(f()))
            }

            fn into_widget(self) -> BoxedUiNode
            where
                Self: Sized,
            {
                self.boxed()
            }
        }

        WidgetNode {
            ctx: WidgetCtx::new(id.into()),
            child: child.cfg_boxed(),

            #[cfg(debug_assertions)]
            inited: false,
        }
        .cfg_boxed()
    }

    /// Create a node that disables interaction for all widget inside `node` using [`BLOCKED`].
    ///
    /// Unlike the [`interactive`] property this does not apply to the contextual widget, only `child` and descendants.
    ///
    /// The node works for both if the `child` is a widget or if it contains widgets, the performance
    /// is slightly better if the `child` is a widget directly.
    ///
    /// [`BLOCKED`]: Interactivity::BLOCKED
    /// [`interactive`]: fn@interactive
    pub fn interactive_node(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
        #[ui_node(struct BlockInteractionNode {
        child: impl UiNode,
        #[var] interactive: impl Var<bool>,
    })]
        impl UiNode for BlockInteractionNode {
            fn info(&self, info: &mut WidgetInfoBuilder) {
                if self.interactive.get() {
                    self.child.info(info);
                } else if let Some(id) = self.child.with_context(|| WIDGET.id()) {
                    // child is a widget.
                    info.push_interactivity_filter(move |args| {
                        if args.info.id() == id {
                            Interactivity::BLOCKED
                        } else {
                            Interactivity::ENABLED
                        }
                    });
                    self.child.info(info);
                } else {
                    let block_range = info.with_children_range(|info| self.child.info(info));
                    if !block_range.is_empty() {
                        // has child widgets.

                        let id = WIDGET.id();
                        info.push_interactivity_filter(move |args| {
                            if let Some(parent) = args.info.parent() {
                                if parent.id() == id {
                                    // check child range
                                    for (i, item) in parent.children().enumerate() {
                                        if item == args.info {
                                            return if !block_range.contains(&i) {
                                                Interactivity::ENABLED
                                            } else {
                                                Interactivity::BLOCKED
                                            };
                                        } else if i >= block_range.end {
                                            break;
                                        }
                                    }
                                }
                            }
                            Interactivity::ENABLED
                        });
                    }
                }
            }

            fn update(&mut self, updates: &WidgetUpdates) {
                if self.interactive.is_new() {
                    WIDGET.update_info();
                }
                self.child.update(updates);
            }
        }
        BlockInteractionNode {
            child: child.cfg_boxed(),
            interactive: interactive.into_var(),
        }
        .cfg_boxed()
    }
}

context_var! {
    static IS_ENABLED_VAR: bool = true;
}

/// Defines the widget innermost node.
///
/// # Capture Only
///
/// This property must be [captured] during widget build and redirected to [`WidgetBuilding::set_child`] in the container widget.
///
/// [captured]: crate::widget#property-capture
/// [`base`]: mod@base
#[property(CHILD, capture, default(FillUiNode))]
pub fn child(_child: impl UiNode, child: impl UiNode) -> impl UiNode {
    _child
}

/// Defines the panel widget innermost nodes.
///
/// # Capture Only
///
/// This property must be [captured] during widget build and used directly in the panel node.
///
/// [captured]: crate::widget#property-capture
#[property(CHILD, capture)]
pub fn children(_child: impl UiNode, children: impl UiNodeList) -> impl UiNode {
    _child
}

/// Defines the unique ID for the widget instance.
///
/// Note that the `id` can convert from a `&'static str` unique name.
///
/// # Capture Only
///
/// This property must be [captured] during widget build, this function only logs an error. The
/// [`base`] widget captures this property if present.
///
/// [captured]: crate::widget#property-capture
/// [`base`]: mod@base
#[property(CONTEXT, capture, default(WidgetId::new_unique()), impl(WidgetBase))]
pub fn id(_child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl UiNode {
    _child
}

/// If default interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`ENABLED`] or [`DISABLED`], to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe the a widget's state use [`interactivity`] value.
///
/// # Interactivity
///
/// Every widget has an [`interactivity`] value, it defines two *tiers* of disabled, the normal disabled blocks the default actions
/// of the widget, but still allows some interactions, such as a different cursor on hover or event an error tool-tip on click, the
/// second tier blocks all interaction with the widget. This property controls the *normal* disabled, to fully block interaction use
/// the [`interactive`] property.
///
/// # Disabled Visual
///
/// Widgets that are interactive should visually indicate when the normal interactions are disabled, you can use the [`is_disabled`]
/// state property in a when block to implement the *visually disabled* appearance of a widget.
///
/// The visual cue for the disabled state is usually a reduced contrast from content and background by *graying-out* the text and applying a
/// grayscale filter for image content. You should also consider adding *disabled interactions* that inform the user when the widget will be
/// enabled.
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`ENABLED`]: crate::widget_info::Interactivity::ENABLED
/// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
/// [`interactivity`]: crate::widget_info::WidgetInfo::interactivity
/// [`interactive`]: fn@interactive
/// [`is_enabled`]: fn@is_enabled
/// [`is_disabled`]: fn@is_disabled
#[property(CONTEXT, default(true), impl(WidgetBase))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct EnabledNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,
    })]
    impl UiNode for EnabledNode {
        fn info(&self, info: &mut WidgetInfoBuilder) {
            if !self.enabled.get() {
                info.push_interactivity(Interactivity::DISABLED);
            }
            self.child.info(info);
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if self.enabled.is_new() {
                WIDGET.update_info();
            }
            self.child.update(updates);
        }
    }

    let enabled = enabled.into_var();
    with_context_var(
        EnabledNode {
            child,
            enabled: enabled.clone(),
        },
        IS_ENABLED_VAR,
        merge_var!(IS_ENABLED_VAR, enabled, |&a, &b| a && b),
    )
}

/// If any interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`BLOCKED`] when `false`, widgets with blocked interactivity do not
/// receive any interaction event and behave like a background visual. To probe the widget state use [`interactivity`] value.
///
/// This property *enables* and *disables* interaction with the widget and its descendants without causing
/// a visual change like [`enabled`], it also blocks "disabled" interactions such as a different cursor or tool-tip for disabled buttons,
/// its use cases are more advanced then [`enabled`], it is mostly used when large parts of the screen are "not ready", hopefully with a message
/// explaining things to the user.
///
/// Note that this affects the widget where it is set and descendants, to disable interaction only in the widgets
/// inside `child` use the [`nodes::interactive_node`].
///
/// [`enabled`]: fn@enabled
/// [`BLOCKED`]: Interactivity::BLOCKED
/// [`interactivity`]: crate::widget_info::WidgetInfo::interactivity
#[property(CONTEXT, default(true))]
pub fn interactive(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct InteractiveNode {
        child: impl UiNode,
        #[var] interactive: impl Var<bool>,
    })]
    impl UiNode for InteractiveNode {
        fn info(&self, info: &mut WidgetInfoBuilder) {
            if !self.interactive.get() {
                info.push_interactivity(Interactivity::BLOCKED);
            }
            self.child.info(info);
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if self.interactive.is_new() {
                WIDGET.update_info();
            }
            self.child.update(updates);
        }
    }
    InteractiveNode {
        child,
        interactive: interactive.into_var(),
    }
}

fn vis_enabled_eq_state(child: impl UiNode, state: impl IntoVar<bool>, expected: bool) -> impl UiNode {
    event_is_state(child, state, true, INTERACTIVITY_CHANGED_EVENT, move |args| {
        if let Some((_, new)) = args.vis_enabled_change(WIDGET.id()) {
            Some(new.is_vis_enabled() == expected)
        } else {
            None
        }
    })
}

/// If the widget is enabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
/// [`WidgetInfo::allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
#[property(EVENT)]
pub fn is_enabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    vis_enabled_eq_state(child, state, true)
}
/// If the widget is disabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// This is the same as `!self.is_enabled`.
///
/// [`enabled`]: fn@enabled
#[property(EVENT)]
pub fn is_disabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    vis_enabled_eq_state(child, state, false)
}

/// Sets the widget visibility.
///
/// This property causes the widget to have the `visibility`, the widget actual visibility is computed, for example,
/// widgets that don't render anything are considered `Hidden` even if the visibility property is not set, this property
/// only forces the widget to layout and render according to the specified visibility.
///
/// To probe the visibility state of a widget in `when` clauses use [`is_visible`], [`is_hidden`] or [`is_collapsed`] in `when` clauses,
/// to probe a widget state use [`UiNode::with_context`] or [`WidgetInfo::visibility`].
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`is_visible`]: fn@is_visible
/// [`is_hidden`]: fn@is_hidden
/// [`is_collapsed`]: fn@is_collapsed
/// [`WidgetInfo::visibility`]: crate::widget_info::WidgetInfo::visibility
#[property(CONTEXT, default(true), impl(WidgetBase))]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    #[ui_node(struct VisibilityNode {
        child: impl UiNode,
        prev_vis: Visibility,
        #[var] visibility: impl Var<Visibility>,
    })]
    impl UiNode for VisibilityNode {
        fn init(&mut self) {
            self.auto_subs();
            self.prev_vis = self.visibility.get();
            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if let Some(vis) = self.visibility.get_new() {
                use Visibility::*;
                match (self.prev_vis, vis) {
                    (Collapsed, Visible) | (Visible, Collapsed) => {
                        WIDGET.layout().render();
                    }
                    (Hidden, Visible) | (Visible, Hidden) => {
                        WIDGET.render();
                    }
                    (Collapsed, Hidden) | (Hidden, Collapsed) => {
                        WIDGET.layout();
                    }
                    _ => {}
                }
                self.prev_vis = vis;
            }
            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            if Visibility::Collapsed != self.visibility.get() {
                self.child.measure(wm)
            } else {
                PxSize::zero()
            }
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            if Visibility::Collapsed != self.visibility.get() {
                self.child.layout(wl)
            } else {
                wl.collapse();
                PxSize::zero()
            }
        }

        fn render(&self, frame: &mut FrameBuilder) {
            match self.visibility.get() {
                Visibility::Visible => self.child.render(frame),
                Visibility::Hidden => frame.hide(|frame| self.child.render(frame)),
                Visibility::Collapsed => {
                    #[cfg(debug_assertions)]
                    {
                        tracing::error!("collapsed rendered, to fix, layout the widget, or `WidgetLayout::collapse_child` the widget")
                    }
                }
            }
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            match self.visibility.get() {
                Visibility::Visible => self.child.render_update(update),
                Visibility::Hidden => update.hidden(|update| self.child.render_update(update)),
                Visibility::Collapsed => {
                    #[cfg(debug_assertions)]
                    {
                        tracing::error!("collapsed rendered, to fix, layout the widget, or `WidgetLayout::collapse_child` the widget")
                    }
                }
            }
        }
    }
    VisibilityNode {
        child,
        prev_vis: Visibility::Visible,
        visibility: visibility.into_var(),
    }
}

fn visibility_eq_state(child: impl UiNode, state: impl IntoVar<bool>, expected: Visibility) -> impl UiNode {
    event_is_state(
        child,
        state,
        expected == Visibility::Visible,
        crate::window::FRAME_IMAGE_READY_EVENT,
        move |_| {
            let tree = WINDOW.widget_tree();
            let vis = tree.get(WIDGET.id()).map(|w| w.visibility()).unwrap_or(Visibility::Visible);

            Some(vis == expected)
        },
    )
}
/// If the widget is [`Visible`](Visibility::Visible).
#[property(CONTEXT)]
pub fn is_visible(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Visible)
}
/// If the widget is [`Hidden`](Visibility::Hidden).
#[property(CONTEXT)]
pub fn is_hidden(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Hidden)
}
/// If the widget is [`Collapsed`](Visibility::Collapsed).
#[property(CONTEXT)]
pub fn is_collapsed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    visibility_eq_state(child, state, Visibility::Collapsed)
}

/// Defines if and how a widget is hit-tested.
///
/// See [`hit_test_mode`](fn@hit_test_mode) for more details.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub enum HitTestMode {
    /// Widget is never hit.
    ///
    /// This mode affects the entire UI branch, if set it disables hit-testing for the widget and all its descendants.
    Disabled,
    /// Widget is hit by any point that intersects the transformed inner bounds rectangle. If the widget is inlined
    /// excludes the first row advance and the last row trailing space.
    Bounds,
    /// Default mode.
    ///
    /// Same as `Bounds`, but also excludes the outside of rounded corners.
    #[default]
    RoundedBounds,
    /// Every render primitive used for rendering the widget is hit-testable, the widget is hit only by
    /// points that intersect visible parts of the render primitives.
    ///
    /// Note that not all primitives implement pixel accurate hit-testing.
    Visual,
}
impl HitTestMode {
    /// Returns `true` if is any mode other then [`Disabled`].
    ///
    /// [`Disabled`]: Self::Disabled
    pub fn is_hit_testable(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Read-only context var with the contextual mode.
    pub fn var() -> impl Var<HitTestMode> {
        HIT_TEST_MODE_VAR.read_only()
    }
}
impl fmt::Debug for HitTestMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "HitTestMode::")?;
        }
        match self {
            Self::Disabled => write!(f, "Disabled"),
            Self::Bounds => write!(f, "Bounds"),
            Self::RoundedBounds => write!(f, "RoundedBounds"),
            Self::Visual => write!(f, "Visual"),
        }
    }
}
impl_from_and_into_var! {
    fn from(default_or_disabled: bool) -> HitTestMode {
        if default_or_disabled {
            HitTestMode::default()
        } else {
            HitTestMode::Disabled
        }
    }
}

context_var! {
    static HIT_TEST_MODE_VAR: HitTestMode = HitTestMode::default();
}

/// Defines how the widget is hit-tested.
///
/// Hit-testing determines if a point intersects with the widget, the most common hit-test point is the mouse pointer.
/// By default widgets are hit by any point inside the widget area, excluding the outer corners if [`corner_radius`] is set,
/// this is very efficient, but assumes that the widget is *filled*, if the widget has visual *holes* the user may be able
/// to see another widget underneath but be unable to click on it.
///
/// If you have a widget with a complex shape or with *holes*, set this property to [`HitTestMode::Visual`] to enable the full
/// hit-testing power where all render primitives and clips used to render the widget are considered during hit-testing.
///
/// [`hit_testable`]: fn@hit_testable
/// [`corner_radius`]: fn@crate::border::corner_radius
#[property(CONTEXT, default(HIT_TEST_MODE_VAR))]
pub fn hit_test_mode(child: impl UiNode, mode: impl IntoVar<HitTestMode>) -> impl UiNode {
    #[ui_node(struct HitTestModeNode {
        child: impl UiNode,
    })]
    impl UiNode for HitTestModeNode {
        fn init(&mut self) {
            WIDGET.sub_var(&HitTestMode::var());
            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if HitTestMode::var().is_new() {
                WIDGET.render();
            }
            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            match HitTestMode::var().get() {
                HitTestMode::Disabled => {
                    frame.with_hit_tests_disabled(|frame| self.child.render(frame));
                }
                HitTestMode::Visual => frame.with_auto_hit_test(true, |frame| self.child.render(frame)),
                _ => frame.with_auto_hit_test(false, |frame| self.child.render(frame)),
            }
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            update.with_auto_hit_test(matches!(HitTestMode::var().get(), HitTestMode::Visual), |update| {
                self.child.render_update(update)
            });
        }
    }

    with_context_var(
        HitTestModeNode { child },
        HIT_TEST_MODE_VAR,
        merge_var!(HIT_TEST_MODE_VAR, mode.into_var(), |&a, &b| match (a, b) {
            (HitTestMode::Disabled, _) | (_, HitTestMode::Disabled) => HitTestMode::Disabled,
            (_, b) => b,
        }),
    )
}

/// If the widget is visible for hit-tests.
///
/// This property is used only for probing the state. You can set the state using
/// the [`hit_test_mode`] property.
///
/// [`hit_testable`]: fn@hit_testable
/// [`hit_test_mode`]: fn@hit_test_mode
#[property(EVENT)]
pub fn is_hit_testable(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, HIT_TEST_MODE_VAR.map(|m| m.is_hit_testable()), state)
}

/// Sets if the widget only renders if [`outer_bounds`] intersects with the [`FrameBuilder::auto_hide_rect`].
///
/// The auto-hide rect is usually *one viewport* of extra space around the  viewport, so only widgets that transform
/// themselves very far need to set this, disabling auto-hide for an widget does not disable it for descendants.
///
/// # Examples
///
/// The example demonstrates a `container` that is *fixed* in the scroll viewport, it sets the `x` and `y` properties
/// to always stay in frame, but transforms set by a widget on itself always affects  the [`inner_bounds`], the
/// [`outer_bounds`] will still be the transform set by the parent so the container may end-up auto-hidden.
///
/// Note that auto-hide is not disabled for the `content` widget, but it's [`outer_bounds`] is affected by the `container`
/// so it is auto-hidden correctly.
///
/// ```
/// # macro_rules! container { ($($tt:tt)*) => { NilUiNode }}
/// # use zero_ui_core::widget_instance::*;
/// fn center_viewport(content: impl UiNode) -> impl UiNode {
///     Container! {
///         zero_ui::core::widget_base::can_auto_hide = false;
///
///         x = zero_ui::widgets::scroll::SCROLL_HORIZONTAL_OFFSET_VAR.map(|&fct| Length::Relative(fct) - 1.vw() * fct);
///         y = zero_ui::widgets::scroll::SCROLL_VERTICAL_OFFSET_VAR.map(|&fct| Length::Relative(fct) - 1.vh() * fct);
///         max_size = (1.vw(), 1.vh());
///         content_align = Align::CENTER;
///      
///         content;
///     }
/// }
/// ```
///  
/// [`outer_bounds`]: WidgetBoundsInfo::outer_bounds
/// [`inner_bounds`]: WidgetBoundsInfo::inner_bounds
#[property(CONTEXT, default(true))]
pub fn can_auto_hide(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct CanAutoHideNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,
    })]
    impl UiNode for CanAutoHideNode {
        fn update(&mut self, updates: &WidgetUpdates) {
            if let Some(new) = self.enabled.get_new() {
                if WIDGET.bounds().can_auto_hide() != new {
                    WIDGET.layout().render();
                }
            }
            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            wl.allow_auto_hide(self.enabled.get());
            self.child.layout(wl)
        }
    }
    CanAutoHideNode {
        child,
        enabled: enabled.into_var(),
    }
}

bitflags! {
    /// Node list methods that are made parallel.
    ///
    /// See [`parallel`] for more details.
    ///
    /// [`parallel`]: fn@parallel
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Parallel: u8 {
        /// Descendants [`UiNode::init`] can run in parallel.
        const INIT =   0b0000_0001;
        /// Descendants [`UiNode::info`] can run in parallel.
        const INFO =   0b0001_0000;
        /// Descendants [`UiNode::deinit`] can run in parallel.
        const DEINIT = 0b0000_0010;
        /// Descendants [`UiNode::event`] can run in parallel.
        const EVENT =  0b0000_0100;
        /// Descendants [`UiNode::update`] can run in parallel.
        const UPDATE = 0b0000_1000;
        /// Descendants [`UiNode::measure`] and [`UiNode::layout`] can run in parallel.
        const LAYOUT = 0b0010_0000;
        /// Descendants [`UiNode::render`] and [`UiNode::render_update`] can run in parallel.
        const RENDER = 0b0100_0000;
    }
}
impl Default for Parallel {
    fn default() -> Self {
        Self::all()
    }
}
context_var! {
    /// Controls what node list methods can run in parallel in an widget and descendants.
    ///
    /// This variable can be set using the [`parallel`] property.
    ///
    /// Is all enabled by default.
    ///
    /// [`parallel`]: fn@parallel
    pub static PARALLEL_VAR: Parallel = Parallel::default();
}
impl_from_and_into_var! {
    fn from(all: bool) -> Parallel {
        if all {
            Parallel::all()
        } else {
            Parallel::empty()
        }
    }
}

/// Defines what node list methods can run in parallel in the widget and descendants.
///
/// This property sets the [`PARALLEL_VAR`] that is used by [`UiNodeList`] implementers to toggle parallel processing.
///
/// See also [`WINDOWS.parallel`] to define parallelization in multi-window apps.
///
/// [`WINDOWS.parallel`]: crate::window::WINDOWS::parallel
#[property(CONTEXT, default(PARALLEL_VAR))]
pub fn parallel(child: impl UiNode, enabled: impl IntoVar<Parallel>) -> impl UiNode {
    with_context_var(child, PARALLEL_VAR, enabled)
}
