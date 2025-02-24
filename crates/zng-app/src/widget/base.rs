//! The widget base, nodes and properties used in most widgets.

use std::{any::TypeId, cell::RefCell, fmt};

use crate::{source_location, widget::WidgetId};

use super::{
    builder::{
        AnyPropertyBuildAction, Importance, PropertyArgs, PropertyId, SourceLocation, WhenBuildAction, WhenInfo, WhenInput, WidgetBuilder,
        WidgetType,
    },
    node::{FillUiNode, UiNode, UiNodeOp},
    WIDGET,
};

use crate::widget::{
    builder::{property_id, NestGroup},
    node::match_node,
    property,
};
use zng_var::{context_var, impl_from_and_into_var, BoxedVar, IntoValue};

/// Base widget.
///
/// The base widget implements the [`id`] property, and uses [`node::include_intrinsics`] and [`node::widget`] to
/// implement the minimum required intrinsics for a widget to be a part of the UI tree.
///
/// See also [`NonWidgetBase`] to declare types that are build like a widget but are never used in the UI tree.
///
/// [`id`]: WidgetBase::id
pub struct WidgetBase {
    builder: RefCell<Option<WidgetBuilder>>,
    importance: Importance,
    when: RefCell<Option<WhenInfo>>,
}
impl WidgetBase {
    /// Gets the type of [`WidgetBase`](struct@WidgetBase).
    pub fn widget_type() -> WidgetType {
        WidgetType {
            type_id: TypeId::of::<Self>(),
            path: "$crate::widget::base::WidgetBase",
            location: source_location!(),
        }
    }

    /// Starts building a new [`WidgetBase`](struct@WidgetBase) instance.
    pub fn widget_new() -> Self {
        Self::inherit(Self::widget_type())
    }

    /// Returns a mutable reference to the widget builder.
    pub fn widget_builder(&mut self) -> &mut WidgetBuilder {
        self.builder.get_mut().as_mut().expect("already built")
    }

    /// Returns a mutable reference to the `when` block if called inside a when block.
    pub fn widget_when(&mut self) -> Option<&mut WhenInfo> {
        self.when.get_mut().as_mut()
    }

    /// Takes the widget builder, finishing the widget macro build.
    ///
    /// After this call trying to set a property using `self` will panic,
    /// the returned builder can still be manipulated directly.
    pub fn widget_take(&mut self) -> WidgetBuilder {
        assert!(self.when.get_mut().is_none(), "cannot take builder with `when` pending");
        self.builder.get_mut().take().expect("builder already taken")
    }

    /// Build the widget.
    ///
    /// After this call trying to set a property will panic.
    pub fn widget_build(&mut self) -> impl UiNode {
        let mut wgt = self.widget_take();
        wgt.push_build_action(|wgt| {
            if !wgt.has_child() {
                wgt.set_child(FillUiNode);
            }
        });
        node::build(wgt)
    }

    /// Returns a mutable reference to the importance of the next property assigns, unsets or when blocks.
    ///
    /// Note that during the `widget_intrinsic` call this is [`Importance::WIDGET`] and after it is [`Importance::INSTANCE`].
    pub fn widget_importance(&mut self) -> &mut Importance {
        &mut self.importance
    }

    /// Start building a `when` block, all properties set after this call are pushed in the when block.
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

    /// End the current `when` block, all properties set after this call are pushed in the widget.
    pub fn end_when_block(&mut self) {
        let when = self.when.get_mut().take().expect("no current `when` block to end");
        self.builder.get_mut().as_mut().unwrap().push_when(self.importance, when);
    }

    fn widget_intrinsic(&mut self) {
        node::include_intrinsics(self.widget_builder());
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
            importance: self.importance,
            when: RefCell::new(self.when.borrow_mut().take()),
        };
        f(&mut inner);
        *self.builder.borrow_mut() = inner.builder.into_inner();
        *self.when.borrow_mut() = inner.when.into_inner();
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
///
/// This trait is used in widget mix-in implementations to constraint `P`, it is also used by the
/// the generated widget code. You do not need to implement it directly.
#[diagnostic::on_unimplemented(note = "`{Self}` is not an `#[widget]`")]
pub trait WidgetImpl {
    /// The inherit function.
    fn inherit(widget: WidgetType) -> Self;

    /// Reference the parent [`WidgetBase`](struct@WidgetBase).
    fn base(&mut self) -> &mut WidgetBase;

    #[doc(hidden)]
    fn base_ref(&self) -> &WidgetBase;

    #[doc(hidden)]
    fn info_instance__() -> Self;

    #[doc(hidden)]
    fn widget_intrinsic(&mut self) {}
}
impl WidgetImpl for WidgetBase {
    fn inherit(widget: WidgetType) -> Self {
        let builder = WidgetBuilder::new(widget);
        let mut w = Self {
            builder: RefCell::new(Some(builder)),
            importance: Importance::WIDGET,
            when: RefCell::new(None),
        };
        w.widget_intrinsic();
        w.importance = Importance::INSTANCE;
        w
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
        $crate::widget::widget_new! {
            new {
                let mut wgt__ = $crate::widget::base::WidgetBase::widget_new();
                let wgt__ = &mut wgt__;
            }
            build { wgt__.widget_build() }
            set { $($tt)* }
        }
    }
}
#[doc(hidden)]
pub use WidgetBaseMacro__ as WidgetBase;

/// Base *widget* for types that build to a custom type that is not used as a part of the UI tree.
///
/// This type can be used as base instead of [`WidgetBase`](struct@WidgetBase) for types that provide
/// a custom build that outputs an instance that is not used as a widget in the UI tree.
pub struct NonWidgetBase {
    base: WidgetBase,
}
impl NonWidgetBase {
    /// Gets the type of [`NonWidgetBase`](struct@NonWidgetBase).
    pub fn widget_type() -> WidgetType {
        WidgetType {
            type_id: TypeId::of::<Self>(),
            path: "$crate::widget::base::NonWidgetBase",
            location: source_location!(),
        }
    }

    /// Starts building a new [`NonWidgetBase`](struct@NonWidgetBase) instance.
    pub fn widget_new() -> Self {
        Self::inherit(Self::widget_type())
    }

    /// Returns a mutable reference to the widget builder.
    pub fn widget_builder(&mut self) -> &mut WidgetBuilder {
        self.base.widget_builder()
    }

    /// Returns a mutable reference to the `when` block if called inside a when block.
    pub fn widget_when(&mut self) -> Option<&mut WhenInfo> {
        self.base.widget_when()
    }

    /// Takes the widget builder, finishing the widget macro build.
    ///
    /// After this call trying to set a property using `self` will panic,
    /// the returned builder can still be manipulated directly.
    pub fn widget_take(&mut self) -> WidgetBuilder {
        self.base.widget_take()
    }

    /// Finishes the build.
    ///
    /// This is the fallback build that simply returns the builder, inheritors should override this method.
    pub fn widget_build(&mut self) -> WidgetBuilder {
        self.widget_take()
    }

    /// Returns a mutable reference to the importance of the next property assigns, unsets or when blocks.
    ///
    /// Note that during the `widget_intrinsic` call this is [`Importance::WIDGET`] and after it is [`Importance::INSTANCE`].
    pub fn widget_importance(&mut self) -> &mut Importance {
        self.base.widget_importance()
    }

    /// Start building a `when` block, all properties set after this call are pushed in the when block.
    pub fn start_when_block(&mut self, inputs: Box<[WhenInput]>, state: BoxedVar<bool>, expr: &'static str, location: SourceLocation) {
        self.base.start_when_block(inputs, state, expr, location)
    }

    /// End the current `when` block, all properties set after this call are pushed in the widget.
    pub fn end_when_block(&mut self) {
        self.base.end_when_block()
    }

    fn widget_intrinsic(&mut self) {}

    /// Push method property.
    #[doc(hidden)]
    pub fn mtd_property__(&self, args: Box<dyn PropertyArgs>) {
        self.base.mtd_property__(args)
    }

    /// Push method unset property.
    #[doc(hidden)]
    pub fn mtd_property_unset__(&self, id: PropertyId) {
        self.base.mtd_property_unset__(id)
    }

    #[doc(hidden)]
    pub fn reexport__(&self, f: impl FnOnce(&mut WidgetBase)) {
        self.base.reexport__(f)
    }

    #[doc(hidden)]
    pub fn push_unset_property_build_action__(&mut self, property_id: PropertyId, action_name: &'static str) {
        self.base.push_unset_property_build_action__(property_id, action_name)
    }

    #[doc(hidden)]
    pub fn push_property_build_action__(
        &mut self,
        property_id: PropertyId,
        action_name: &'static str,
        input_actions: Vec<Box<dyn AnyPropertyBuildAction>>,
    ) {
        self.base.push_property_build_action__(property_id, action_name, input_actions)
    }

    #[doc(hidden)]
    pub fn push_when_build_action_data__(&mut self, property_id: PropertyId, action_name: &'static str, data: WhenBuildAction) {
        self.base.push_when_build_action_data__(property_id, action_name, data)
    }
}
impl WidgetImpl for NonWidgetBase {
    fn inherit(widget: WidgetType) -> Self {
        let builder = WidgetBuilder::new(widget);
        let mut w = Self {
            base: WidgetBase {
                builder: RefCell::new(Some(builder)),
                importance: Importance::WIDGET,
                when: RefCell::new(None),
            },
        };
        w.widget_intrinsic();
        w.base.importance = Importance::INSTANCE;
        w
    }

    fn base(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn base_ref(&self) -> &WidgetBase {
        &self.base
    }

    fn info_instance__() -> Self {
        Self {
            base: WidgetBase {
                builder: RefCell::new(None),
                importance: Importance::INSTANCE,
                when: RefCell::new(None),
            },
        }
    }
}
impl WidgetExt for NonWidgetBase {
    fn ext_property__(&mut self, args: Box<dyn PropertyArgs>) {
        self.base.ext_property__(args)
    }

    fn ext_property_unset__(&mut self, id: PropertyId) {
        self.base.ext_property_unset__(id)
    }
}

/// Basic nodes for widgets, some used in [`WidgetBase`].
///
/// [`WidgetBase`]: struct@WidgetBase
pub mod node {
    use zng_layout::unit::{PxCornerRadius, PxRect, PxSize};
    use zng_var::Var;

    use crate::{
        render::{FrameBuilder, FrameUpdate, FrameValueKey},
        update::{EventUpdate, WidgetUpdates},
        widget::{
            info::{WidgetInfoBuilder, WidgetLayout, WidgetMeasure},
            node::BoxedUiNode,
            WidgetCtx, WidgetUpdateMode,
        },
    };

    use super::*;

    /// Insert [`widget_child`] and [`widget_inner`] in the widget.
    pub fn include_intrinsics(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            wgt.push_intrinsic(NestGroup::CHILD, "widget_child", node::widget_child);
            wgt.push_intrinsic(NestGroup::WIDGET_INNER, "widget_inner", node::widget_inner);
        });
    }

    /// Capture the [`id`] property and builds the base widget.
    ///
    /// Note that this function does not handle missing child node, it falls back to [`FillUiNode`]. The [`WidgetBase`]
    /// widget uses the [`FillUiNode`] if none was set.
    ///
    /// [`WidgetBase`]: struct@WidgetBase
    /// [`id`]: fn@id
    pub fn build(mut wgt: WidgetBuilder) -> impl UiNode {
        let id = wgt.capture_value_or_else(property_id!(id), WidgetId::new_unique);
        let child = wgt.build();
        node::widget(child, id)
    }

    /// Returns a node that wraps `child` and potentially applies child transforms if the `child` turns out
    /// to not be a full widget or to be multiple children. This is important for making properties like *padding* or *content_align* work
    /// for any [`UiNode`] as content.
    ///
    /// This node also pass through the `child` inline layout return info if the widget and child are inlining and the
    /// widget has not set inline info before delegating measure.
    ///
    /// This node must be intrinsic at [`NestGroup::CHILD`], the [`WidgetBase`] default intrinsic inserts it.
    ///
    /// [`WidgetBase`]: struct@WidgetBase
    pub fn widget_child(child: impl UiNode) -> impl UiNode {
        let key = FrameValueKey::new_unique();
        let mut define_ref_frame = false;

        match_node(child, move |child, op| match op {
            UiNodeOp::Measure { wm, desired_size } => {
                *desired_size = child.measure(wm);

                if let Some(inline) = wm.inline() {
                    if inline.is_default() {
                        if let Some(child_inline) = child
                            .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_inline())
                            .flatten()
                        {
                            // pass through child inline
                            *inline = child_inline;
                        }
                    }
                }
            }
            UiNodeOp::Layout { wl, final_size } => {
                let (s, d) = wl.with_child(|wl| child.layout(wl));
                *final_size = s;

                if d != define_ref_frame {
                    define_ref_frame = d;
                    WIDGET.render();
                }

                if !define_ref_frame {
                    // child maybe widget, try to copy inline
                    if let Some(inline) = wl.inline() {
                        if inline.is_default() {
                            child.with_context(WidgetUpdateMode::Ignore, || {
                                let bounds = WIDGET.bounds();
                                let child_inline = bounds.inline();
                                if let Some(child_inline) = child_inline {
                                    inline.clone_from(&*child_inline);
                                }
                            });
                        }
                    }
                }
            }

            UiNodeOp::Render { frame } => {
                let offset = WIDGET.bounds().child_offset();
                if define_ref_frame {
                    frame.push_reference_frame(key.into(), key.bind(offset.into(), true), true, false, |frame| child.render(frame));
                } else {
                    frame.push_child(offset, |frame| {
                        child.render(frame);
                    });
                }
            }
            UiNodeOp::RenderUpdate { update } => {
                let offset = WIDGET.bounds().child_offset();
                if define_ref_frame {
                    update.with_transform(key.update(offset.into(), true), false, |update| child.render_update(update));
                } else {
                    update.with_child(offset, |update| child.render_update(update))
                }
            }
            _ => {}
        })
    }

    /// Returns a node that wraps `child` and marks the [`WidgetLayout::with_inner`] and [`FrameBuilder::push_inner`].
    ///
    /// This node renders the inner transform and implements the [`HitTestMode`] for the widget.
    ///
    /// This node must be intrinsic at [`NestGroup::BORDER`], the [`WidgetBase`] default intrinsic inserts it.
    ///
    /// [`WidgetBase`]: struct@WidgetBase
    pub fn widget_inner(child: impl UiNode) -> impl UiNode {
        #[derive(Default, PartialEq)]
        struct HitClips {
            bounds: PxSize,
            corners: PxCornerRadius,
        }

        let transform_key = FrameValueKey::new_unique();
        let mut clips = HitClips::default();

        match_node(child, move |child, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_var_layout(&HIT_TEST_MODE_VAR);
            }
            UiNodeOp::Layout { wl, final_size } => {
                *final_size = wl.with_inner(|wl| child.layout(wl));

                let mode = HIT_TEST_MODE_VAR.get();
                let c = if matches!(mode, HitTestMode::Bounds | HitTestMode::RoundedBounds) {
                    HitClips {
                        bounds: *final_size,
                        corners: if matches!(mode, HitTestMode::RoundedBounds) {
                            WIDGET.border().corner_radius()
                        } else {
                            PxCornerRadius::zero()
                        },
                    }
                } else {
                    HitClips::default()
                };

                if c != clips {
                    clips = c;
                    WIDGET.render();
                }
            }
            UiNodeOp::Render { frame } => {
                frame.push_inner(transform_key, true, |frame| {
                    frame.hit_test().push_clips(
                        |c| {
                            if let Some(inline) = WIDGET.bounds().inline() {
                                for r in inline.negative_space().iter() {
                                    c.push_clip_rect(*r, true);
                                }
                            }
                        },
                        |h| match HIT_TEST_MODE_VAR.get() {
                            HitTestMode::RoundedBounds => {
                                h.push_rounded_rect(PxRect::from_size(clips.bounds), clips.corners);
                            }
                            HitTestMode::Bounds => {
                                h.push_rect(PxRect::from_size(clips.bounds));
                            }
                            _ => {}
                        },
                    );

                    child.render(frame);
                });
            }
            UiNodeOp::RenderUpdate { update } => {
                update.update_inner(transform_key, true, |update| child.render_update(update));
            }
            _ => {}
        })
    }

    /// Create a widget node that wraps `child` and introduces a new widget context. The node defines
    /// an [`WIDGET`] context and implements the widget in each specific node method.
    ///
    /// This node must wrap the outer-most context node in the build, it is the [`WidgetBase`] widget type.
    ///
    /// The node retains the widget state if build with `cfg(any(test, feature = "test_util")))`, otherwise
    /// the state is cleared.
    ///
    /// [`WidgetBase`]: struct@WidgetBase
    pub fn widget(child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl UiNode {
        struct WidgetNode<C> {
            ctx: WidgetCtx,
            child: C,

            #[cfg(debug_assertions)]
            inited: bool,
            #[cfg(debug_assertions)]
            info_built: bool,
        }
        impl<C: UiNode> UiNode for WidgetNode<C> {
            fn init(&mut self) {
                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::init` called in inited widget {:?}", WIDGET.id());
                    }

                    self.child.init();
                    WIDGET.update_info().layout().render();

                    #[cfg(debug_assertions)]
                    {
                        self.inited = true;
                        self.info_built = false;
                    }
                });
                self.ctx.take_reinit(); // ignore reinit request
            }

            fn deinit(&mut self) {
                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", WIDGET.id());
                    }

                    self.child.deinit();
                    WIDGET.update_info().layout().render();

                    #[cfg(debug_assertions)]
                    {
                        self.inited = false;
                        self.info_built = false;
                    }
                });
                self.ctx.deinit(cfg!(any(test, feature = "test_util")));
            }

            fn info(&mut self, info: &mut WidgetInfoBuilder) {
                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::info` called in not inited widget {:?}", WIDGET.id());
                    }

                    #[cfg(debug_assertions)]
                    {
                        self.info_built = true;
                    }

                    info.push_widget(|info| {
                        self.child.info(info);
                    });
                });

                if self.ctx.is_pending_reinit() {
                    WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
                }
            }

            fn event(&mut self, update: &EventUpdate) {
                if self.ctx.take_reinit() {
                    self.deinit();
                    self.init();
                }

                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", update.event().name(), WIDGET.id());
                    } else if !self.info_built {
                        tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in widget {:?} before first info build", update.event().name(), WIDGET.id());
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

                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::update` called in not inited widget {:?}", WIDGET.id());
                    } else if !self.info_built {
                        tracing::error!(target: "widget_base", "`UiNode::update` called in widget {:?} before first info build", WIDGET.id());
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

            fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
                let desired_size = WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Ignore, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::measure` called in not inited widget {:?}", WIDGET.id());
                    } else if !self.info_built {
                        tracing::error!(target: "widget_base", "`UiNode::measure` called in widget {:?} before first info build", WIDGET.id());
                    }

                    wm.with_widget(|wm| {
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
                                if sum_width > width + 0.1 {
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

                // ignore
                let _ = self.ctx.take_reinit();

                desired_size
            }

            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                let final_size = WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::layout` called in not inited widget {:?}", WIDGET.id());
                    } else if !self.info_built {
                        tracing::error!(target: "widget_base", "`UiNode::layout` called in widget {:?} before first info build", WIDGET.id());
                    }

                    wl.with_widget(|wl| {
                        let child_size = self.child.layout(wl);

                        // verify that inline row segments fit in row rectangle
                        #[cfg(debug_assertions)]
                        if let Some(inline) = wl.inline() {
                            use zng_layout::unit::Px;

                            for (name, row, segs) in inline
                                .rows
                                .first()
                                .iter()
                                .map(|r| ("first", r, &inline.first_segs))
                                .chain(inline.rows.last().iter().map(|r| ("last", r, &inline.last_segs)))
                            {
                                let width = row.width();
                                let sum_width = segs.iter().map(|s| s.width).sum::<Px>();
                                if (sum_width - width) > Px(1) {
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
                    WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
                }

                final_size
            }

            fn render(&mut self, frame: &mut FrameBuilder) {
                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::render` called in not inited widget {:?}", WIDGET.id());
                    } else if !self.info_built {
                        tracing::error!(target: "widget_base", "`UiNode::render` called in widget {:?} before first info build", WIDGET.id());
                    }

                    frame.push_widget(|frame| self.child.render(frame));
                });

                if self.ctx.is_pending_reinit() {
                    WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
                }
            }

            fn render_update(&mut self, update: &mut FrameUpdate) {
                WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || {
                    #[cfg(debug_assertions)]
                    if !self.inited {
                        tracing::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", WIDGET.id());
                    } else if !self.info_built {
                        tracing::error!(target: "widget_base", "`UiNode::render_update` called in widget {:?} before first info build", WIDGET.id());
                    }

                    update.update_widget(|update| self.child.render_update(update));
                });

                if self.ctx.is_pending_reinit() {
                    WIDGET.with_context(&mut self.ctx, WidgetUpdateMode::Bubble, || WIDGET.update());
                }
            }

            fn is_widget(&self) -> bool {
                true
            }

            fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
            where
                F: FnOnce() -> R,
            {
                WIDGET.with_context(&mut self.ctx, update_mode, || Some(f()))
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
            #[cfg(debug_assertions)]
            info_built: false,
        }
        .cfg_boxed()
    }
}

/// Unique ID of the widget instance.
///
/// Note that the `id` can convert from a `&'static str` unique name.
#[property(CONTEXT, capture, default(WidgetId::new_unique()), widget_impl(WidgetBase))]
pub fn id(id: impl IntoValue<WidgetId>) {}

/// Defines if and how a widget is hit-tested.
#[derive(Copy, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum HitTestMode {
    /// Widget is never hit.
    ///
    /// This mode affects the entire UI branch, if set it disables hit-testing for the widget and all its descendants,
    /// even if they set explicitly set their hit-test mode to something else.
    Disabled,
    /// Widget is hit by any point that intersects the transformed inner bounds rectangle. If the widget is inlined
    /// excludes the first row advance and the last row trailing space.
    Bounds,
    /// Same as `Bounds`, but also excludes the outside of rounded corners.
    ///
    /// This is the default mode.
    #[default]
    RoundedBounds,
    /// Widget is hit by any point that intersects the hit-test shape defined on render by
    /// [`FrameBuilder::hit_test`] and auto hit-test.
    ///
    /// [`FrameBuilder::hit_test`]: crate::render::FrameBuilder::hit_test
    Detailed,
}
impl HitTestMode {
    /// Returns `true` if is any mode other then [`Disabled`].
    ///
    /// [`Disabled`]: Self::Disabled
    pub fn is_hit_testable(&self) -> bool {
        !matches!(self, Self::Disabled)
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
            Self::Detailed => write!(f, "Detailed"),
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

bitflags::bitflags! {
    /// Node list methods that are made parallel.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct Parallel: u8 {
        /// Descendants [`UiNode::init`] can run in parallel.
        const INIT   = 0b0000_0001;
        /// Descendants [`UiNode::info`] can run in parallel.
        const INFO   = 0b0001_0000;
        /// Descendants [`UiNode::deinit`] can run in parallel.
        const DEINIT = 0b0000_0010;
        /// Descendants [`UiNode::event`] can run in parallel.
        const EVENT  = 0b0000_0100;
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
    /// Defines what node list methods can run in parallel in a widget and descendants.
    ///
    /// This variable can be set using the `parallel` property.
    ///
    /// Is all enabled by default.
    pub static PARALLEL_VAR: Parallel = Parallel::default();

    /// Defines the hit-test mode for a widget and descendants.
    ///
    /// This variable can be set using the `hit_test_mode` property.
    ///
    /// Note that hit-test is disabled for the entire sub-tree, even if a child sets to a
    /// different mode again, the `hit_test_mode` property already enforces this, custom
    /// nodes should avoid overriding `Disabled`, as the hit-test will still be disabled,
    /// but other custom code that depend on this variable will read an incorrect state.
    pub static HIT_TEST_MODE_VAR: HitTestMode = HitTestMode::default();
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
