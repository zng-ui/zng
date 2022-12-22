//! The [`base`](mod@base) and properties used in most widgets.

use std::{fmt, mem};

use crate::{
    context::*,
    event::{EventHandles, EventUpdate},
    property,
    render::{FrameBuilder, FrameUpdate, FrameValueKey, ReuseRange, SpatialFrameId},
    ui_node,
    units::{PxCornerRadius, PxPoint, PxRect, PxSize, PxTransform},
    var::*,
    widget,
    widget_builder::*,
    widget_info::*,
    widget_instance::*,
    window::WIDGET_INFO_CHANGED_EVENT,
};

use parking_lot::Mutex;

/// Base widget that implements the necessary core API.
///
/// The base widget does [`nodes::include_intrinsics`] to enable proper layout and render in all widgets that inherit from base.
///
/// The base widget also provides a default function that captures the [`id`] and handles missing child node by capturing
/// [`child`] or falling back to [`FillUiNode`].
///
/// [`id`]: fn@id
/// [`child`]: fn@child
#[widget($crate::widget_base::base)]
pub mod base {
    use super::*;

    properties! {
        pub super::id;
        pub super::enabled;
        pub super::visibility;
    }

    fn include(wgt: &mut WidgetBuilder) {
        nodes::include_intrinsics(wgt);
    }

    fn build(mut wgt: WidgetBuilder) -> impl UiNode {
        wgt.push_build_action(|wgt| {
            if !wgt.has_child() {
                wgt.set_child(FillUiNode);
            }
        });
        nodes::build(wgt)
    }
}

/// Basic nodes for widgets, some used in [`base`].
///
/// [`base`]: mod@base
pub mod nodes {
    use super::*;

    /// Insert [`child_layout`] and [`inner`] in the widget.
    pub fn include_intrinsics(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT, "child_layout", nodes::child_layout);
            wgt.push_intrinsic(NestGroup::BORDER, "inner", nodes::inner);
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

    /// Returns a node that wraps `panel` and applies *child_layout* transforms to it.
    ///
    /// This node should wrap the inner most *child* node of panel widgets, and that in turn should layout the [`children`].
    ///
    /// [`children`]: fn@crate::widget_base::children
    pub fn children_layout(panel: impl UiNode) -> impl UiNode {
        #[ui_node(struct ChildrenLayoutNode {
                child: impl UiNode,
                spatial_id: SpatialFrameId,
                translation_key: FrameValueKey<PxTransform>,
            })]
        impl UiNode for ChildrenLayoutNode {
            fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
                self.child.measure(ctx, wm)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                wl.with_children(ctx, |ctx, wl| self.child.layout(ctx, wl))
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                frame.push_reference_frame(self.spatial_id, self.translation_key.bind(transform, true), true, false, |frame| {
                    self.child.render(ctx, frame)
                });
            }
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                update.with_transform(self.translation_key.update(transform, true), false, |update| {
                    self.child.render_update(ctx, update);
                });
            }
        }
        ChildrenLayoutNode {
            child: panel.cfg_boxed(),
            spatial_id: SpatialFrameId::new_unique(),
            translation_key: FrameValueKey::new_unique(),
        }
        .cfg_boxed()
    }

    /// Returns a node that wraps `child` and potentially applies child transforms if the `child` turns out
    /// to not be a full widget. This is important for making properties like *padding* or *content_align* work
    /// for any [`UiNode`] as content.
    ///
    /// This node must be intrinsic at [`NestGroup::CHILD_LAYOUT`], the [`base`] default intrinsic inserts it.
    ///
    /// [`base`]: mod@base
    pub fn child_layout(child: impl UiNode) -> impl UiNode {
        #[ui_node(struct ChildLayoutNode {
                child: impl UiNode,
                id: Option<(SpatialFrameId, FrameValueKey<PxTransform>)>,
            })]
        impl UiNode for ChildLayoutNode {
            fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
                self.child.measure(ctx, wm)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                let (size, needed) = wl.with_child(ctx, |ctx, wl| self.child.layout(ctx, wl));
                if self.id.is_none() {
                    if needed {
                        // start rendering.
                        self.id = Some((SpatialFrameId::new_unique(), FrameValueKey::new_unique()));
                        ctx.updates.render();
                    }
                } else if !needed {
                    self.id = None;
                    ctx.updates.render();
                }
                size
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                if let Some((id, key)) = &self.id {
                    let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                    frame.push_reference_frame(*id, key.bind(transform, true), true, false, |frame| self.child.render(ctx, frame));
                } else {
                    self.child.render(ctx, frame);
                }
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                if let Some((_, key)) = &self.id {
                    let transform = PxTransform::from(ctx.widget_info.bounds.child_offset());
                    update.with_transform(key.update(transform, true), false, |update| self.child.render_update(ctx, update));
                } else {
                    self.child.render_update(ctx, update);
                }
            }
        }
        ChildLayoutNode {
            child: child.cfg_boxed(),
            id: None,
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
    pub fn inner(child: impl UiNode) -> impl UiNode {
        #[derive(Default, PartialEq)]
        struct HitClips {
            bounds: PxSize,
            corners: PxCornerRadius,
            inline: (PxPoint, PxPoint),
        }
        #[ui_node(struct InnerNode {
            child: impl UiNode,
            transform_key: FrameValueKey<PxTransform>,
            clips: HitClips,
        })]
        impl UiNode for InnerNode {
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.sub_var(&HitTestMode::var());
                self.child.init(ctx);
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if HitTestMode::var().is_new(ctx) {
                    ctx.updates.layout();
                }
                self.child.update(ctx, updates);
            }

            fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
                self.child.measure(ctx, wm)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                let size = wl.with_inner(ctx, |ctx, wl| self.child.layout(ctx, wl));

                let mode = HitTestMode::var().get();
                let clips = if matches!(mode, HitTestMode::Bounds | HitTestMode::RoundedBounds) {
                    HitClips {
                        bounds: size,
                        corners: if matches!(mode, HitTestMode::RoundedBounds) {
                            ctx.widget_info.border.corner_radius()
                        } else {
                            PxCornerRadius::zero()
                        },
                        inline: if let Some(inline) = wl.inline() {
                            (inline.first_row, inline.last_row)
                        } else {
                            Default::default()
                        },
                    }
                } else {
                    HitClips::default()
                };

                if clips != self.clips {
                    self.clips = clips;
                    ctx.updates.render();
                }

                size
            }
            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                frame.push_inner(ctx, self.transform_key, true, |ctx, frame| {
                    frame.hit_test().push_clips(
                        |c| {
                            if self.clips.inline.1 != PxPoint::zero() {
                                if self.clips.inline.0 != PxPoint::zero() {
                                    c.push_clip_rect(PxRect::from_size(self.clips.inline.0.to_vector().to_size()), true)
                                }

                                let mut last = PxRect::from_size(self.clips.bounds).to_box2d();
                                last.min += self.clips.inline.1.to_vector();
                                let last = last.to_rect();
                                c.push_clip_rect(last, true);
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

                    self.child.render(ctx, frame);
                });
            }
            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                update.update_inner(ctx, self.transform_key, true, |ctx, update| self.child.render_update(ctx, update));
            }
        }
        InnerNode {
            child: child.cfg_boxed(),
            transform_key: FrameValueKey::new_unique(),
            clips: HitClips::default(),
        }
        .cfg_boxed()
    }

    /// Create a widget node that wraps `child` and introduces a new widget context. The node calls
    /// [`WidgetContext::widget_context`], [`LayoutContext::with_widget`] and [`FrameBuilder::push_widget`]
    /// to define the widget.
    ///
    /// This node must wrap the outer-most context node in the build, it is the [`base`] widget type.
    ///
    /// [`base`]: mod@base
    pub fn widget(child: impl UiNode, id: impl IntoValue<WidgetId>) -> impl UiNode {
        #[derive(Default)]
        struct MtxData {
            pending_updates: InfoLayoutRenderUpdates,
            offsets_pass: LayoutPassId,
            reuse: Option<ReuseRange>,
        }
        struct WidgetNode<C> {
            id: WidgetId,
            state: OwnedStateMap<state_map::Widget>,
            child: C,
            info: WidgetContextInfo,

            var_handles: VarHandles,
            event_handles: crate::event::EventHandles,

            #[cfg(debug_assertions)]
            inited: bool,
            m: Mutex<MtxData>,
        }
        impl<C: UiNode> UiNode for WidgetNode<C> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::init` called in already inited widget {:?}", self.id);
                }

                ctx.widget_context(
                    self.id,
                    &self.info,
                    &mut self.state,
                    &mut self.var_handles,
                    &mut self.event_handles,
                    |ctx| self.child.init(ctx),
                );
                self.m.get_mut().pending_updates = InfoLayoutRenderUpdates::all();

                #[cfg(debug_assertions)]
                {
                    self.inited = true;
                }
            }

            fn deinit(&mut self, ctx: &mut WidgetContext) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::deinit` called in not inited widget {:?}", self.id);
                }

                ctx.widget_context(
                    self.id,
                    &self.info,
                    &mut self.state,
                    &mut self.var_handles,
                    &mut self.event_handles,
                    |ctx| self.child.deinit(ctx),
                );
                self.m.get_mut().pending_updates = InfoLayoutRenderUpdates::none();
                self.var_handles.clear();
                self.var_handles.clear();
                self.m.get_mut().reuse = None;

                #[cfg(debug_assertions)]
                {
                    self.inited = false;
                }
            }

            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::info` called in not inited widget {:?}", self.id);
                }

                ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                    if mem::take(&mut self.m.lock().pending_updates.info) {
                        info.push_widget(self.id, self.info.bounds.clone(), self.info.border.clone(), |info| {
                            self.child.info(ctx, info)
                        });
                    } else {
                        info.push_widget_reuse(ctx);
                    }
                });
            }

            fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::event::<{}>` called in not inited widget {:?}", update.event().name(), self.id);
                }

                let (_, (updates, reinit)) = ctx.widget_context(
                    self.id,
                    &self.info,
                    &mut self.state,
                    &mut self.var_handles,
                    &mut self.event_handles,
                    |ctx| {
                        update.with_widget(ctx, |ctx, update| {
                            self.child.event(ctx, update);
                        });
                    },
                );
                self.m.get_mut().pending_updates |= updates;

                if reinit {
                    self.deinit(ctx);
                    self.init(ctx);
                }
            }

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::update` called in not inited widget {:?}", self.id);
                }

                let (_, (updates, reinit)) = ctx.widget_context(
                    self.id,
                    &self.info,
                    &mut self.state,
                    &mut self.var_handles,
                    &mut self.event_handles,
                    |ctx| {
                        updates.with_widget(ctx, |ctx, updates| {
                            self.child.update(ctx, updates);
                        });
                    },
                );
                self.m.get_mut().pending_updates |= updates;

                if reinit {
                    self.deinit(ctx);
                    self.init(ctx);
                }
            }

            fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::measure` called in not inited widget {:?}", self.id);
                }

                let reuse = !self.m.lock().pending_updates.layout;

                ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                    wm.with_widget(ctx, reuse, |ctx, wm| self.child.measure(ctx, wm))
                })
            }

            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::layout` called in not inited widget {:?}", self.id);
                }

                let reuse = !mem::take(&mut self.m.get_mut().pending_updates.layout);

                let (child_size, updates) = ctx.with_widget(self.id, &self.info, &mut self.state, |ctx| {
                    wl.with_widget(ctx, reuse, |ctx, wl| self.child.layout(ctx, wl))
                });
                self.m.get_mut().pending_updates |= updates;

                child_size
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::render` called in not inited widget {:?}", self.id);
                }

                let mut m = self.m.lock();
                if !m.pending_updates.render.take().is_none() || m.offsets_pass != self.info.bounds.offsets_pass() {
                    // cannot reuse.
                    m.reuse = None;
                    m.offsets_pass = self.info.bounds.offsets_pass();
                }

                ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                    frame.push_widget(ctx, &mut m.reuse, |ctx, frame| self.child.render(ctx, frame));
                });
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                #[cfg(debug_assertions)]
                if !self.inited {
                    tracing::error!(target: "widget_base", "`UiNode::render_update` called in not inited widget {:?}", self.id);
                }

                let mut m = self.m.lock();
                let mut reuse = true;
                if !m.pending_updates.render.take().is_none() || m.offsets_pass != self.info.bounds.offsets_pass() {
                    reuse = false;
                    m.offsets_pass = self.info.bounds.offsets_pass();
                }

                ctx.with_widget(self.id, &self.info, &self.state, |ctx| {
                    update.update_widget(ctx, reuse, |ctx, update| self.child.render_update(ctx, update));
                });
            }

            fn is_widget(&self) -> bool {
                true
            }

            fn with_context<R, F>(&self, f: F) -> Option<R>
            where
                F: FnOnce(&mut WidgetNodeContext) -> R,
            {
                Some(f(&mut WidgetNodeContext {
                    id: self.id,
                    widget_info: &self.info,
                    widget_state: self.state.borrow(),
                }))
            }

            fn with_context_mut<R, F>(&mut self, f: F) -> Option<R>
            where
                F: FnOnce(&mut WidgetNodeMutContext) -> R,
            {
                Some(f(&mut WidgetNodeMutContext {
                    id: self.id,
                    widget_info: &self.info,
                    widget_state: self.state.borrow_mut(),
                    handles: WidgetHandles {
                        var_handles: &mut self.var_handles,
                        event_handles: &mut self.event_handles,
                    },
                }))
            }

            fn into_widget(self) -> BoxedUiNode
            where
                Self: Sized,
            {
                self.boxed()
            }
        }
        WidgetNode {
            id: id.into(),
            state: OwnedStateMap::default(),
            child: child.cfg_boxed(),
            info: WidgetContextInfo::default(),
            var_handles: VarHandles::default(),
            event_handles: EventHandles::default(),
            #[cfg(debug_assertions)]
            inited: false,
            m: Default::default(),
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
            fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                if self.interactive.get() {
                    self.child.info(ctx, info);
                } else if let Some(id) = self.child.with_context(|ctx| ctx.id) {
                    // child is a widget.
                    info.push_interactivity_filter(move |args| {
                        if args.info.widget_id() == id {
                            Interactivity::BLOCKED
                        } else {
                            Interactivity::ENABLED
                        }
                    });
                    self.child.info(ctx, info);
                } else {
                    let block_range = info.with_children_range(|info| self.child.info(ctx, info));
                    if !block_range.is_empty() {
                        // has child widgets.

                        let id = ctx.path.widget_id();
                        info.push_interactivity_filter(move |args| {
                            if let Some(parent) = args.info.parent() {
                                if parent.widget_id() == id {
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

            fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
                if self.interactive.is_new(ctx) {
                    ctx.updates.info();
                }
                self.child.update(ctx, updates);
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
#[property(CHILD_LAYOUT, capture, default(FillUiNode))]
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
#[property(CHILD_LAYOUT, capture)]
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
#[property(CONTEXT, capture, default(WidgetId::new_unique()))]
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
#[property(CONTEXT, default(true))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct EnabledNode {
        child: impl UiNode,
        #[var] enabled: impl Var<bool>,
    })]
    impl UiNode for EnabledNode {
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if !self.enabled.get() {
                info.push_interactivity(Interactivity::DISABLED);
            }
            self.child.info(ctx, info);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.enabled.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
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
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if !self.interactive.get() {
                info.push_interactivity(Interactivity::BLOCKED);
            }
            self.child.info(ctx, info);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.interactive.is_new(ctx) {
                ctx.updates.info();
            }
            self.child.update(ctx, updates);
        }
    }
    InteractiveNode {
        child,
        interactive: interactive.into_var(),
    }
}

fn vis_enabled_eq_state(child: impl UiNode, state: impl IntoVar<bool>, expected: bool) -> impl UiNode {
    event_is_state(child, state, true, WIDGET_INFO_CHANGED_EVENT, move |ctx, _| {
        let is_enabled = ctx
            .info_tree
            .get(ctx.path.widget_id())
            .unwrap()
            .interactivity()
            .is_visually_enabled();

        Some(is_enabled == expected)
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
#[property(CONTEXT, default(true))]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    #[ui_node(struct VisibilityNode {
        child: impl UiNode,
        prev_vis: Visibility,
        #[var] visibility: impl Var<Visibility>,
    })]
    impl UiNode for VisibilityNode {
        fn info(&self, ctx: &mut InfoContext, widget: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);
            self.prev_vis = self.visibility.get();
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(vis) = self.visibility.get_new(ctx) {
                use Visibility::*;
                match (self.prev_vis, vis) {
                    (Collapsed, Visible) | (Visible, Collapsed) => ctx.updates.layout_render(),
                    (Hidden, Visible) | (Visible, Hidden) => ctx.updates.render(),
                    (Collapsed, Hidden) | (Hidden, Collapsed) => ctx.updates.layout(),
                    _ => {}
                }
                self.prev_vis = vis;
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            if Visibility::Collapsed != self.visibility.get() {
                self.child.measure(ctx, wm)
            } else {
                PxSize::zero()
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if Visibility::Collapsed != self.visibility.get() {
                self.child.layout(ctx, wl)
            } else {
                wl.collapse(ctx);
                PxSize::zero()
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            match self.visibility.get() {
                Visibility::Visible => self.child.render(ctx, frame),
                Visibility::Hidden => frame.hide(|frame| self.child.render(ctx, frame)),
                Visibility::Collapsed => unreachable!("collapsed rendered"),
            }
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            match self.visibility.get() {
                Visibility::Visible => self.child.render_update(ctx, update),
                Visibility::Hidden => update.hidden(|update| self.child.render_update(ctx, update)),
                Visibility::Collapsed => unreachable!("collapsed rendered"),
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
        move |ctx, _| {
            let vis = ctx
                .info_tree
                .get(ctx.path.widget_id())
                .map(|w| w.visibility())
                .unwrap_or(Visibility::Visible);

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
#[derive(Copy, Clone, PartialEq, Eq)]
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
impl Default for HitTestMode {
    fn default() -> Self {
        HitTestMode::RoundedBounds
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
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&HitTestMode::var());
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if HitTestMode::var().is_new(ctx) {
                ctx.updates.render();
            }
            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            match HitTestMode::var().get() {
                HitTestMode::Disabled => {
                    frame.with_hit_tests_disabled(|frame| self.child.render(ctx, frame));
                }
                HitTestMode::Visual => frame.with_auto_hit_test(true, |frame| self.child.render(ctx, frame)),
                _ => frame.with_auto_hit_test(false, |frame| self.child.render(ctx, frame)),
            }
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            update.with_auto_hit_test(matches!(HitTestMode::var().get(), HitTestMode::Visual), |update| {
                self.child.render_update(ctx, update)
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
///     container! {
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
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if let Some(new) = self.enabled.get_new(ctx) {
                if ctx.widget_info.bounds.can_auto_hide() != new {
                    ctx.updates.layout_render();
                }
            }
            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(ctx, wm)
        }

        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            wl.allow_auto_hide(self.enabled.get());
            self.child.layout(ctx, wl)
        }
    }
    CanAutoHideNode {
        child,
        enabled: enabled.into_var(),
    }
}
