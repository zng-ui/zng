//! Window layers.

use crate::core::{
    mouse::MOUSE,
    task::parking_lot::Mutex,
    timer::{DeadlineHandle, TIMERS},
    units::DipToPx,
    window::WIDGET_INFO_CHANGED_EVENT,
};
use crate::prelude::new_property::*;

use std::{fmt, mem, ops};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use crate::crate_util::RunOnDrop;

struct LayersCtx {
    items: EditableUiNodeListRef,
}

/// Windows layers.
///
/// The window layers is z-order stacking panel that fills the window content area, widgets can be inserted
/// with a *z-index* that is the [`LayerIndex`]. The inserted widgets parent is the window root widget and
/// it is affected by the context properties set on the window only.
///
/// # Layout & Render
///
/// Layered widgets are measured and arranged using the same constraints as the window root widget, the desired
/// size is discarded, only the root widget desired size can affect the window size.
///
/// Layered widgets are all layout and rendered after the window content, this means that the [`WidgetBoundsInfo`]
/// of normal widgets are always up-to-date when the layered widget is arranged and rendered, so if you
/// implement custom layouts that align the layered widget with a normal widget using the info values it will always be in sync with
/// a single layout pass, see [`insert_anchored`] for more details.
///
/// [`insert_anchored`]: Self::insert_anchored
pub struct LAYERS;
impl LAYERS {
    /// Insert the `widget` in the layer identified by a [`LayerIndex`].
    ///
    /// If the `layer` variable updates the widget is moved to the new layer, if multiple widgets
    /// are inserted in the same layer the later inserts are on top of the previous.
    ///
    /// If the `widget` is not a full widget after init it is immediately deinited and removed. Only full
    /// widgets are allowed, and ideally widgets with known IDs, so that they can be removed.
    pub fn insert(&self, layer: impl IntoVar<LayerIndex>, widget: impl UiNode) {
        let layer = layer.into_var();
        let widget = match_widget(widget.boxed(), move |widget, op| match op {
            UiNodeOp::Init => {
                widget.init();

                if !widget.is_widget() {
                    *widget.child() = NilUiNode.boxed();
                    LAYERS.cleanup();
                }

                // widget may only become a full widget after init (ArcNode)
                widget.with_context(WidgetUpdateMode::Bubble, || {
                    WIDGET.set_state(&LAYER_INDEX_ID, layer.get());
                    WIDGET.sub_var(&layer);
                });
            }
            UiNodeOp::Update { .. } => {
                if let Some(index) = layer.get_new() {
                    widget.with_context(WidgetUpdateMode::Bubble, || {
                        WIDGET.set_state(&LAYER_INDEX_ID, index);
                        SortingListParent::invalidate_sort();
                    });
                }
            }
            _ => {}
        })
        .boxed();

        let r = WINDOW.with_state(|s| match s.get(&WINDOW_LAYERS_ID) {
            Some(open) => {
                // window already open
                open.items.push(widget);
                Ok(())
            }
            None => Err(widget),
        });
        if let Err(widget) = r {
            WINDOW.with_state_mut(|mut s| {
                // window not open yet, `widget` will be inited with the window
                s.entry(&WINDOW_PRE_INIT_LAYERS_ID).or_default().push(Mutex::new(widget));
            });
        }
    }

    /// Insert the `widget` in the layer and *anchor* it to the offset/transform of another widget.
    ///
    /// The `anchor` is the ID of another widget, the inserted `widget` will be offset/transform so that it aligns
    /// with the `anchor` widget top-left. The `mode` is a value of [`AnchorMode`] that defines if the `widget` will
    /// receive the full transform or just the offset.
    ///
    /// If the `anchor` widget is not found the `widget` is not rendered (visibility `Collapsed`). If the `widget`
    /// is not a full widget after init it is immediately deinited and removed.
    pub fn insert_anchored(
        &self,
        layer: impl IntoVar<LayerIndex>,
        anchor: impl IntoVar<WidgetId>,
        mode: impl IntoVar<AnchorMode>,

        widget: impl UiNode,
    ) {
        let layer = layer.into_var();
        let anchor = anchor.into_var();
        let mode = mode.into_var();

        let mut _info_changed_handle = None;
        let mut mouse_pos_handle = None;

        let mut cursor_once_pending = false;
        let mut anchor_info = None;
        let mut offset = (PxPoint::zero(), PxPoint::zero());
        let mut interactivity = false;

        let transform_key = FrameValueKey::new_unique();
        let mut corner_radius_ctx_handle = None;

        let widget = match_widget(widget.boxed(), move |widget, op| match op {
            UiNodeOp::Init => {
                widget.init();

                if !widget.is_widget() {
                    widget.deinit();
                    *widget.child() = NilUiNode.boxed();
                    // cleanup requested by the `insert` node.
                }

                widget.with_context(WidgetUpdateMode::Bubble, || {
                    WIDGET.sub_var(&anchor).sub_var(&mode);

                    let tree = WINDOW.widget_tree();
                    if let Some(w) = tree.get(anchor.get()) {
                        anchor_info = Some((w.bounds_info(), w.border_info()));
                    }

                    interactivity = mode.with(|m| m.interactivity);
                    _info_changed_handle = Some(WIDGET_INFO_CHANGED_EVENT.subscribe(WIDGET.id()));

                    if mode.with(|m| matches!(&m.transform, AnchorTransform::Cursor(_))) {
                        mouse_pos_handle = Some(MOUSE.position().subscribe(UpdateOp::Update, WIDGET.id()));
                    } else if mode.with(|m| matches!(&m.transform, AnchorTransform::CursorOnce(_))) {
                        cursor_once_pending = true;
                    }
                });
            }
            UiNodeOp::Deinit => {
                widget.deinit();

                anchor_info = None;
                _info_changed_handle = None;
                mouse_pos_handle = None;
                corner_radius_ctx_handle = None;
                cursor_once_pending = false;
            }
            UiNodeOp::Info { info } => {
                if interactivity {
                    if let Some(widget) = widget.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                        let anchor = anchor.get();
                        let querying = AtomicBool::new(false);
                        info.push_interactivity_filter(move |args| {
                            if args.info.id() == widget {
                                if querying.swap(true, Ordering::Relaxed) {
                                    return Interactivity::ENABLED; // avoid recursion.
                                }
                                let _q = RunOnDrop::new(|| querying.store(false, Ordering::Relaxed));
                                args.info
                                    .tree()
                                    .get(anchor)
                                    .map(|a| a.interactivity())
                                    .unwrap_or(Interactivity::BLOCKED)
                            } else {
                                Interactivity::ENABLED
                            }
                        });
                    }
                }
            }
            UiNodeOp::Event { update } => {
                if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
                    if args.window_id == WINDOW.id() {
                        anchor_info = WINDOW.widget_tree().get(anchor.get()).map(|w| (w.bounds_info(), w.border_info()));
                    }
                }
            }
            UiNodeOp::Update { .. } => {
                widget.with_context(WidgetUpdateMode::Bubble, || {
                    if let Some(anchor) = anchor.get_new() {
                        anchor_info = WINDOW.widget_tree().get(anchor).map(|w| (w.bounds_info(), w.border_info()));
                        if mode.with(|m| m.interactivity) {
                            WIDGET.update_info();
                        }
                        WIDGET.layout().render();
                    }
                    if let Some(mode) = mode.get_new() {
                        if mode.interactivity != interactivity {
                            interactivity = mode.interactivity;
                            WIDGET.update_info();
                        }
                        if matches!(&mode.transform, AnchorTransform::Cursor(_)) {
                            if mouse_pos_handle.is_none() {
                                mouse_pos_handle = Some(MOUSE.position().subscribe(UpdateOp::Update, WIDGET.id()));
                            }
                            cursor_once_pending = false;
                        } else {
                            cursor_once_pending = matches!(&mode.transform, AnchorTransform::CursorOnce(_));
                            mouse_pos_handle = None;
                        }
                        WIDGET.layout().render();
                    } else if mouse_pos_handle.is_some() && MOUSE.position().is_new() {
                        WIDGET.layout();
                    }
                });
            }
            UiNodeOp::Measure { wm, desired_size } => {
                widget.delegated();

                if let Some((bounds, border)) = &anchor_info {
                    let mode = mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        *desired_size = LAYOUT.with_constraints(
                            match mode.size {
                                AnchorSize::Unbounded => PxConstraints2d::new_unbounded(),
                                AnchorSize::Window => LAYOUT.constraints().with_fill(false, false),
                                AnchorSize::InnerSize => PxConstraints2d::new_exact_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => PxConstraints2d::new_exact_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => PxConstraints2d::new_exact_size(bounds.outer_size()),
                            },
                            || widget.measure(wm),
                        );
                    }
                }
            }
            UiNodeOp::Layout { wl, final_size } => {
                widget.delegated();

                if let Some((bounds, border)) = &anchor_info {
                    let mode = mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        // if we don't link visibility or anchor is not collapsed.

                        let layer_size = LAYOUT.with_constraints(
                            match mode.size {
                                AnchorSize::Unbounded => PxConstraints2d::new_unbounded(),
                                AnchorSize::Window => LAYOUT.constraints().with_fill(false, false),
                                AnchorSize::InnerSize => PxConstraints2d::new_exact_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => PxConstraints2d::new_exact_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => PxConstraints2d::new_exact_size(bounds.outer_size()),
                            },
                            || {
                                if mode.corner_radius {
                                    let mut cr = border.corner_radius();
                                    if let AnchorSize::InnerBorder = mode.size {
                                        cr = cr.deflate(border.offsets());
                                    }
                                    CORNER_RADIUS_VAR.with_context_var(
                                        corner_radius_ctx_handle.get_or_insert_with(ContextInitHandle::new).clone(),
                                        cr,
                                        || BORDER.with_corner_radius(|| widget.layout(wl)),
                                    )
                                } else {
                                    widget.layout(wl)
                                }
                            },
                        );

                        if let Some((p, update)) = match &mode.transform {
                            AnchorTransform::Cursor(p) => Some((p, true)),
                            AnchorTransform::CursorOnce(p) => Some((p, mem::take(&mut cursor_once_pending))),
                            _ => None,
                        } {
                            // cursor transform mode, only visible if cursor over window
                            const NO_POS_X: Px = Px::MIN;
                            if update {
                                if let Some(pos) = MOUSE
                                    .position()
                                    .get()
                                    .and_then(|(w_id, pos)| if w_id == WINDOW.id() { Some(pos) } else { None })
                                {
                                    let fct = LAYOUT.scale_factor().0;
                                    let (cursor_size, cursor_spot) =
                                        WINDOW.vars().cursor().get().map(|c| c.size_and_spot()).unwrap_or_default();
                                    let cursor_rect = DipRect::new((pos - cursor_spot).to_point(), cursor_size).to_px(fct);
                                    let place = cursor_rect.origin
                                        + LAYOUT
                                            .with_constraints(PxConstraints2d::new_exact_size(cursor_rect.size), || p.place.layout())
                                            .to_vector();
                                    let origin = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(layer_size), || p.origin.layout());

                                    let o = (place, origin);
                                    if offset != o {
                                        offset = o;
                                        WIDGET.render_update();
                                    }

                                    *final_size = layer_size;
                                    return;
                                } else {
                                    // collapsed signal (permanent if `CursorOnce`)
                                    offset.0.x = NO_POS_X;
                                }
                            } else {
                                // offset already set
                                if offset.0.x != NO_POS_X {
                                    // and it is not collapsed `CursorOnce`
                                    *final_size = layer_size;
                                    return;
                                }
                            }
                        } else {
                            // other transform modes, will be visible
                            let o = match &mode.transform {
                                AnchorTransform::InnerOffset(p) => {
                                    let place =
                                        LAYOUT.with_constraints(PxConstraints2d::new_exact_size(bounds.inner_size()), || p.place.layout());
                                    let origin = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(layer_size), || p.origin.layout());
                                    (place, origin)
                                }
                                AnchorTransform::InnerBorderOffset(p) => {
                                    let place = LAYOUT
                                        .with_constraints(PxConstraints2d::new_exact_size(border.inner_size(bounds)), || p.place.layout());
                                    let origin = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(layer_size), || p.origin.layout());
                                    (place, origin)
                                }
                                AnchorTransform::OuterOffset(p) => {
                                    let place =
                                        LAYOUT.with_constraints(PxConstraints2d::new_exact_size(bounds.outer_size()), || p.place.layout());
                                    let origin = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(layer_size), || p.origin.layout());
                                    (place, origin)
                                }
                                _ => (PxPoint::zero(), PxPoint::zero()),
                            };
                            if offset != o {
                                offset = o;
                                WIDGET.render_update();
                            }

                            *final_size = layer_size;
                            return;
                        }
                    }
                }

                widget.with_context(WidgetUpdateMode::Bubble, || {
                    wl.collapse();
                });
            }
            UiNodeOp::Render { frame } => {
                widget.delegated();

                if let Some((bounds_info, border_info)) = &anchor_info {
                    let mode = mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        let mut push_reference_frame = |mut transform: PxTransform, is_translate_only: bool| {
                            if mode.viewport_bound {
                                transform = adjust_viewport_bound(transform, widget);
                            }
                            frame.push_reference_frame(
                                transform_key.into(),
                                transform_key.bind(transform, true),
                                is_translate_only,
                                false,
                                |frame| widget.render(frame),
                            );
                        };

                        match mode.transform {
                            AnchorTransform::InnerOffset(_) => {
                                let place_in_window = bounds_info.inner_transform().transform_point(offset.0).unwrap_or_default();
                                let offset = place_in_window - offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::InnerBorderOffset(_) => {
                                let place_in_window = border_info
                                    .inner_transform(bounds_info)
                                    .transform_point(offset.0)
                                    .unwrap_or_default();
                                let offset = place_in_window - offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::OuterOffset(_) => {
                                let place_in_window = bounds_info.outer_transform().transform_point(offset.0).unwrap_or_default();
                                let offset = place_in_window - offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::Cursor(_) | AnchorTransform::CursorOnce(_) => {
                                let offset = offset.0 - offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::InnerTransform => {
                                push_reference_frame(bounds_info.inner_transform(), false);
                            }
                            AnchorTransform::InnerBorderTransform => {
                                push_reference_frame(border_info.inner_transform(bounds_info), false);
                            }
                            AnchorTransform::OuterTransform => {
                                push_reference_frame(bounds_info.outer_transform(), false);
                            }
                            _ => widget.render(frame),
                        }
                    }
                }
            }
            UiNodeOp::RenderUpdate { update } => {
                if let Some((bounds_info, border_info)) = &anchor_info {
                    let mode = mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        let mut with_transform = |mut transform: PxTransform| {
                            if mode.viewport_bound {
                                transform = adjust_viewport_bound(transform, widget);
                            }
                            update.with_transform(transform_key.update(transform, true), false, |update| widget.render_update(update));
                        };

                        match mode.transform {
                            AnchorTransform::InnerOffset(_) => {
                                let place_in_window = bounds_info.inner_transform().transform_point(offset.0).unwrap_or_default();
                                let offset = place_in_window - offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::InnerBorderOffset(_) => {
                                let place_in_window = border_info
                                    .inner_transform(bounds_info)
                                    .transform_point(offset.0)
                                    .unwrap_or_default();
                                let offset = place_in_window - offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::OuterOffset(_) => {
                                let place_in_window = bounds_info.outer_transform().transform_point(offset.0).unwrap_or_default();
                                let offset = place_in_window - offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::Cursor(_) | AnchorTransform::CursorOnce(_) => {
                                let offset = offset.0 - offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::InnerTransform => {
                                with_transform(bounds_info.inner_transform());
                            }
                            AnchorTransform::InnerBorderTransform => {
                                with_transform(border_info.inner_transform(bounds_info));
                            }
                            AnchorTransform::OuterTransform => {
                                with_transform(bounds_info.outer_transform());
                            }
                            _ => widget.render_update(update),
                        }
                    }
                }
            }
            _ => {}
        });
        self.insert(layer, widget);
    }

    /// Remove the widget in the next update.
    ///
    /// The `id` must the widget id of a previous inserted widget, nothing happens if the widget is not found.
    pub fn remove(&self, id: impl Into<WidgetId>) {
        WINDOW.with_state(|s| {
            s.req(&WINDOW_LAYERS_ID).items.remove(id);
        });
    }

    fn cleanup(&self) {
        WINDOW.with_state(|s| {
            s.req(&WINDOW_LAYERS_ID).items.retain(|n| n.is_widget());
        });
    }
}

fn adjust_viewport_bound(transform: PxTransform, widget: &mut impl UiNode) -> PxTransform {
    let window_bounds = WINDOW.vars().actual_size_px().get();
    let wgt_bounds = PxBox::from(
        widget
            .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().outer_size())
            .unwrap_or_else(PxSize::zero),
    );
    let wgt_bounds = transform.outer_transformed(wgt_bounds).unwrap_or_default();

    let x_underflow = -wgt_bounds.min.x.min(Px(0));
    let x_overflow = (wgt_bounds.max.x - window_bounds.width).max(Px(0));
    let y_underflow = -wgt_bounds.min.y.min(Px(0));
    let y_overflow = (wgt_bounds.max.y - window_bounds.height).max(Px(0));

    let x = x_underflow - x_overflow;
    let y = y_underflow - y_overflow;

    let correction = PxVector::new(x, y);

    transform.then_translate(correction.cast())
}

static WINDOW_PRE_INIT_LAYERS_ID: StaticStateId<Vec<Mutex<BoxedUiNode>>> = StaticStateId::new_unique();
static WINDOW_LAYERS_ID: StaticStateId<LayersCtx> = StaticStateId::new_unique();
static LAYER_INDEX_ID: StaticStateId<LayerIndex> = StaticStateId::new_unique();

static HAS_LAYER_REMOVE_HANDLERS_ID: StaticStateId<()> = StaticStateId::new_unique();
event_args! {
    /// Arguments for [`on_layer_remove_requested`].
    pub struct LayerRemoveRequestedArgs {
        list: EditableUiNodeListRef,
        ..
        /// No target, only the layered widget receives the event.
        fn delivery_list(&self, _delivery_list: &mut UpdateDeliveryList) {}
    }
}
event! {
    static LAYER_REMOVE_REQUESTED_EVENT: LayerRemoveRequestedArgs;
}
context_var! {
    static IS_LAYER_REMOVING_VAR: bool = false;
    static LAYER_REMOVE_CANCELLABLE_VAR: bool = true;
}

/// If layer remove can be cancelled by this widget.
///
/// Layer remove is cancellable by  default, if this is set to `false` handlers of [`on_layer_remove_requested`]
/// cannot cancel the layer remove by stopping propagation and the [`layer_remove_delay`] is not applied.
///
/// Widget implementers can set this property as a node of high-priority to override control of the layer remove cancel
/// feature.
///
/// [`layer_remove_delay`]: fn@layer_remove_delay
/// [`on_layer_remove_requested`]: fn@on_layer_remove_requested
#[property(CONTEXT, default(LAYER_REMOVE_CANCELLABLE_VAR))]
pub fn layer_remove_cancellable(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, LAYER_REMOVE_CANCELLABLE_VAR, enabled)
}

/// Event that a layered widget receives when it is about to be removed.
///
/// You can stop the [`LayerRemoveRequestedArgs::propagation`] to cancel the remove. Note that after cancel
/// you can request remove again. Also note that remove cancellation can be disabled by the widget by
/// setting [`layer_remove_cancellable`] to false.
///
/// This event property must be set on the outer-most widget inserted in [`LAYERS`], the event does not propagate
/// to descendants of the layered widget.
///
/// [`layer_remove_cancellable`]: fn@layer_remove_cancellable
#[property(EVENT)]
pub fn on_layer_remove_requested(child: impl UiNode, handler: impl WidgetHandler<LayerRemoveRequestedArgs>) -> impl UiNode {
    let mut handler = handler.cfg_boxed();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.flag_state(&HAS_LAYER_REMOVE_HANDLERS_ID);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = LAYER_REMOVE_REQUESTED_EVENT.on(update) {
                if LAYER_REMOVE_CANCELLABLE_VAR.get() {
                    handler.event(args);
                } else {
                    handler.event(&LayerRemoveRequestedArgs::new(
                        args.timestamp,
                        EventPropagationHandle::new(),
                        EditableUiNodeListRef::dummy(),
                    ));
                }
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();
        }
        _ => {}
    })
}

/// Awaits `delay` before actually removing the layered widget after remove is requested.
///
/// Note that layered widgets will still be removed instantly if [`layer_remove_cancellable`] is false,
/// some widgets may disable it when they need to be removed immediately, as an example, tooltip widgets
/// will ignore the delay when another tooltip is already opening.
///
/// [`layer_remove_cancellable`]: fn@layer_remove_cancellable
#[property(EVENT, default(Duration::ZERO))]
pub fn layer_remove_delay(child: impl UiNode, delay: impl IntoVar<Duration>) -> impl UiNode {
    let delay = delay.into_var();
    let mut timer = None::<DeadlineHandle>;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.flag_state(&HAS_LAYER_REMOVE_HANDLERS_ID);
        }
        UiNodeOp::Deinit => {
            timer = None;
            let _ = IS_LAYER_REMOVING_VAR.set(false);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = LAYER_REMOVE_REQUESTED_EVENT.on(update) {
                if !LAYER_REMOVE_CANCELLABLE_VAR.get() {
                    // allow
                    return;
                }

                if let Some(timer) = &timer {
                    if timer.has_executed() {
                        // allow
                        return;
                    } else {
                        args.propagation().stop();
                        // timer already running.
                        return;
                    }
                }

                let delay = delay.get();
                if delay != Duration::ZERO {
                    args.propagation().stop();

                    let list = args.list.clone();
                    let id = WIDGET.id();

                    let _ = IS_LAYER_REMOVING_VAR.set(true);

                    timer = Some(TIMERS.on_deadline(
                        delay,
                        app_hn_once!(|_| {
                            list.remove(id);
                        }),
                    ));
                }
            }
        }
        _ => {}
    })
}

/// If remove was requested for this layered widget and it is just awaiting for the [`layer_remove_delay`].
///
/// [`layer_remove_delay`]: fn@layer_remove_delay
#[property(CONTEXT)]
pub fn is_layer_removing(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    // reverse context var, is set by `layer_remove_delay`.
    with_context_var(child, IS_LAYER_REMOVING_VAR, state)
}

/// Represents a layer in a window.
///
/// See [`LAYERS`] for more information.
#[derive(Default, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct LayerIndex(pub u32);
impl LayerIndex {
    /// The top-most layer.
    ///
    /// Only widgets that are pretending to be a child window should use this layer, including menus,
    /// drop-downs, pop-ups and tool-tips.
    ///
    /// This is the [`u32::MAX`] value.
    pub const TOP_MOST: LayerIndex = LayerIndex(u32::MAX);

    /// The layer for *adorner* display items.
    ///
    /// Adorner widgets are related to another widget but not as a visual part of it, examples of adorners
    /// are resize handles in a widget visual editor, or an interactive help/guide feature.
    ///
    /// This is the [`TOP_MOST - u16::MAX`] value.
    pub const ADORNER: LayerIndex = LayerIndex(Self::TOP_MOST.0 - u16::MAX as u32);

    /// The default layer, just above the normal window content.
    ///
    /// This is the `0` value.
    pub const DEFAULT: LayerIndex = LayerIndex(0);

    /// Compute `self + other` saturating at the [`TOP_MOST`] bound instead of overflowing.
    ///
    /// [`TOP_MOST`]: Self::TOP_MOST
    pub fn saturating_add(self, other: impl Into<LayerIndex>) -> Self {
        Self(self.0.saturating_add(other.into().0))
    }

    /// Compute `self - other` saturating at the [`DEFAULT`] bound instead of overflowing.
    ///
    /// [`DEFAULT`]: Self::DEFAULT
    pub fn saturating_sub(self, other: impl Into<LayerIndex>) -> Self {
        Self(self.0.saturating_sub(other.into().0))
    }

    /// Gets the const name of this value.
    pub fn name(self) -> Option<&'static str> {
        if self == Self::DEFAULT {
            Some("DEFAULT")
        } else if self == Self::TOP_MOST {
            Some("TOP_MOST")
        } else if self == Self::ADORNER {
            Some("ADORNER")
        } else {
            None
        }
    }
}
impl fmt::Debug for LayerIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.name() {
            if f.alternate() {
                write!(f, "LayerIndex::")?;
            }
            write!(f, "{}", name)
        } else {
            write!(f, "LayerIndex({})", self.0)
        }
    }
}
impl std::str::FromStr for LayerIndex {
    type Err = <u32 as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DEFAULT" => Ok(Self::DEFAULT),
            "TOP_MOST" => Ok(Self::TOP_MOST),
            "ADORNER" => Ok(Self::ADORNER),
            n => Ok(Self(n.parse()?)),
        }
    }
}
impl_from_and_into_var! {
    fn from(index: u32) -> LayerIndex {
        LayerIndex(index)
    }
}
/// Calls [`LayerIndex::saturating_add`].
impl<T: Into<Self>> ops::Add<T> for LayerIndex {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.saturating_add(rhs)
    }
}
/// Calls [`LayerIndex::saturating_sub`].
impl<T: Into<Self>> ops::Sub<T> for LayerIndex {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        self.saturating_sub(rhs)
    }
}
/// Calls [`LayerIndex::saturating_add`].
impl<T: Into<Self>> ops::AddAssign<T> for LayerIndex {
    fn add_assign(&mut self, rhs: T) {
        *self = *self + rhs;
    }
}
/// Calls [`LayerIndex::saturating_sub`].
impl<T: Into<Self>> ops::SubAssign<T> for LayerIndex {
    fn sub_assign(&mut self, rhs: T) {
        *self = *self - rhs;
    }
}
impl ops::Mul<Factor> for LayerIndex {
    type Output = Self;

    fn mul(self, rhs: Factor) -> Self::Output {
        LayerIndex(self.0 * rhs)
    }
}
impl ops::MulAssign<Factor> for LayerIndex {
    fn mul_assign(&mut self, rhs: Factor) {
        self.0 *= rhs;
    }
}
impl ops::Div<Factor> for LayerIndex {
    type Output = Self;

    fn div(self, rhs: Factor) -> Self::Output {
        LayerIndex(self.0 / rhs)
    }
}
impl ops::DivAssign<Factor> for LayerIndex {
    fn div_assign(&mut self, rhs: Factor) {
        self.0 /= rhs;
    }
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum LayerIndexSerde<'s> {
    Named(&'s str),
    Unamed(u32),
}
impl serde::Serialize for LayerIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            if let Some(name) = self.name() {
                return LayerIndexSerde::Named(name).serialize(serializer);
            }
        }
        LayerIndexSerde::Unamed(self.0).serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for LayerIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        match LayerIndexSerde::deserialize(deserializer)? {
            LayerIndexSerde::Named(name) => match name {
                "DEFAULT" => Ok(Self::DEFAULT),
                "TOP_MOST" => Ok(Self::TOP_MOST),
                "ADORNER" => Ok(Self::ADORNER),
                unknown => Err(D::Error::unknown_variant(unknown, &["DEFAULT", "TOP_MOST", "ADORNER"])),
            },
            LayerIndexSerde::Unamed(i) => Ok(Self(i)),
        }
    }
}

/// Represents two points that position a layer widget with its anchor widget.
///
/// The `place` point is layout in the anchor widget bounds, the `origin` point is layout in the layer widget bounds,
/// the layer widget is offset so that the `origin` point aligns with the `place` point.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnchorOffset {
    /// Point in the anchor widget.
    pub place: Point,
    /// Point in the layer widget.
    pub origin: Point,
}
impl AnchorOffset {
    /// New place and origin points from same `point`.
    pub fn new(point: Point) -> Self {
        Self {
            place: point.clone(),
            origin: point,
        }
    }

    /// Layer widget is horizontally centered on the anchor widget and the top edge aligns.
    pub fn in_top() -> Self {
        Self::new(Point::top())
    }

    /// Layer widget is horizontally centered on the anchor widget and the bottom edge aligns.
    pub fn in_bottom() -> Self {
        Self::new(Point::bottom())
    }

    /// Layer widget is vertically centered on the anchor widget and the left edge aligns.
    pub fn in_left() -> Self {
        Self::new(Point::left())
    }

    /// Layer widget is vertically centered on the anchor widget and the right edge aligns.
    pub fn in_right() -> Self {
        Self::new(Point::right())
    }

    /// Layer widget top-left corner aligns with the anchor widget top-left corner.
    pub fn in_top_left() -> Self {
        Self::new(Point::top_left())
    }

    /// Layer widget top-right corner aligns with the anchor widget top-right corner.
    pub fn in_top_right() -> Self {
        Self::new(Point::top_right())
    }

    /// Layer widget bottom-left corner aligns with the anchor widget bottom-left corner.
    pub fn in_bottom_left() -> Self {
        Self::new(Point::bottom_left())
    }

    /// Layer widget bottom-right corner aligns with the anchor widget bottom-right corner.
    pub fn in_bottom_right() -> Self {
        Self::new(Point::bottom_right())
    }

    /// Layer widget is centered on the anchor widget.
    pub fn center() -> Self {
        Self::new(Point::center())
    }

    /// Layer widget is horizontally centered on the anchor widget and its bottom edge aligns with the anchors top edge.
    pub fn out_top() -> Self {
        Self {
            place: Point::top(),
            origin: Point::bottom(),
        }
    }

    /// Layer widget is horizontally centered on the anchor widget and its top edge aligns with the anchors bottom edge.
    pub fn out_bottom() -> Self {
        Self {
            place: Point::bottom(),
            origin: Point::top(),
        }
    }

    /// Layer widget is vertically centered on the anchor widget and its right edge aligns with the anchors left edge.
    pub fn out_left() -> Self {
        Self {
            place: Point::left(),
            origin: Point::right(),
        }
    }

    /// Layer widget is vertically centered on the anchor widget and its left edge aligns with the anchors right edge.
    pub fn out_right() -> Self {
        Self {
            place: Point::right(),
            origin: Point::left(),
        }
    }

    /// Layer widget bottom-right corner aligns with anchor widget top-left corner.
    pub fn out_top_left() -> Self {
        Self {
            place: Point::top_left(),
            origin: Point::bottom_right(),
        }
    }

    /// Layer widget bottom-left corner aligns with anchor widget top-right corner.
    pub fn out_top_right() -> Self {
        Self {
            place: Point::top_right(),
            origin: Point::bottom_left(),
        }
    }

    /// Layer widget top-right corner aligns with anchor widget bottom-left corner.
    pub fn out_bottom_left() -> Self {
        Self {
            place: Point::bottom_left(),
            origin: Point::top_right(),
        }
    }

    /// Layer widget bottom-right corner aligns with anchor widget top-left corner.
    pub fn out_bottom_right() -> Self {
        Self {
            place: Point::bottom_right(),
            origin: Point::top_left(),
        }
    }

    /// Layer widget bottom-left corner aligns with anchor widget top-left corner.
    pub fn out_top_in_left() -> Self {
        Self {
            place: Point::top_left(),
            origin: Point::bottom_left(),
        }
    }

    /// Layer widget bottom-right corner aligns with anchor widget top-right corner.
    pub fn out_top_in_right() -> Self {
        Self {
            place: Point::top_right(),
            origin: Point::bottom_right(),
        }
    }

    /// Layer widget top-left corner aligns with anchor widget bottom-left corner.
    pub fn out_bottom_in_left() -> Self {
        Self {
            place: Point::bottom_left(),
            origin: Point::top_left(),
        }
    }

    /// Layer widget top-right corner aligns with anchor widget bottom-right corner.
    pub fn out_bottom_in_right() -> Self {
        Self {
            place: Point::bottom_right(),
            origin: Point::top_right(),
        }
    }

    /// Layer widget top-right corner aligns with anchor widget top-left corner.
    pub fn out_left_in_top() -> Self {
        Self {
            place: Point::top_left(),
            origin: Point::top_right(),
        }
    }

    /// Layer widget bottom-right corner aligns with anchor widget bottom-left corner.
    pub fn out_left_in_bottom() -> Self {
        Self {
            place: Point::bottom_left(),
            origin: Point::bottom_right(),
        }
    }

    /// Layer widget top-left corner aligns with anchor widget top-right corner.
    pub fn out_right_in_top() -> Self {
        Self {
            place: Point::top_right(),
            origin: Point::top_left(),
        }
    }

    /// Layer widget bottom-left corner aligns with anchor widget bottom-right corner.
    pub fn out_right_in_bottom() -> Self {
        Self {
            place: Point::bottom_right(),
            origin: Point::bottom_left(),
        }
    }
}
impl_from_and_into_var! {
     /// `(place, origin)`.
     fn from<P: Into<Point>, O: Into<Point>>(place_origin: (P, O)) -> AnchorOffset {
        AnchorOffset {
            place: place_origin.0.into(),
            origin: place_origin.1.into(),
        }
    }
}
impl animation::Transitionable for AnchorOffset {
    fn lerp(self, to: &Self, step: super::EasingStep) -> Self {
        Self {
            place: self.place.lerp(&to.place, step),
            origin: self.origin.lerp(&to.place, step),
        }
    }
}

/// Options for [`AnchorMode::transform`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AnchorTransform {
    /// Widget does not copy any position from the anchor widget.
    None,
    /// The layer widget is translated so that a point in the layer widget outer-bounds aligns with a point
    /// in the anchor widget inner-bounds.
    InnerOffset(AnchorOffset),
    /// The layer widget is translated so that a point in the layer widget outer-bounds aligns with a point
    /// in the anchor widget fill area (inside the border offset).
    InnerBorderOffset(AnchorOffset),

    /// The layer widget is translated so that a point in the layer widget outer-bounds aligns with a point
    /// in the anchor widget outer-bounds.
    OuterOffset(AnchorOffset),

    /// The full inner transform of the anchor object is applied to the widget.
    InnerTransform,

    /// The full inner transform of the anchor object is applied to the widget plus the border widths offset.
    InnerBorderTransform,

    /// The full outer transform of the anchor object is applied to the widget.
    OuterTransform,

    /// The layer widget is translated on the first layout to be at the cursor position.
    ///
    /// The anchor offset place point is resolved in the cursor icon size (approximate).
    CursorOnce(AnchorOffset),
    /// The layer widget is translated to follow the cursor position.
    ///
    /// The anchor offset place point is resolved in the cursor icon size (approximate).
    Cursor(AnchorOffset),
}
impl_from_and_into_var! {
    /// `InnerOffset`.
    fn from(inner_offset: AnchorOffset) -> AnchorTransform {
        AnchorTransform::InnerOffset(inner_offset)
    }
    /// `InnerOffset`.
    fn from<P: Into<Point>, O: Into<Point>>(inner_offset: (P, O)) -> AnchorTransform {
        AnchorOffset::from(inner_offset).into()
    }
}

/// Options for [`AnchorMode::size`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnchorSize {
    /// Widget does not copy any size from the anchor widget, the available size is infinite, the
    /// final size is the desired size.
    ///
    /// Note that layered widgets do not affect the window size and a widget that overflows the content
    /// boundaries is clipped.
    Unbounded,
    /// Widget does not copy any size from the anchor widget, the available size and final size
    /// are the window's root size.
    Window,
    /// The available size and final size is the anchor widget's outer size.
    OuterSize,
    /// The available size and final size is the anchor widget's inner size.
    InnerSize,
    /// The available size and final size is the anchor widget's inner size offset by the border widths.
    InnerBorder,
}

/// Defines what properties the layered widget takes from the anchor widget.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnchorMode {
    /// What transforms are copied from the anchor widget and applied as a *parent* transform of the widget.
    pub transform: AnchorTransform,
    /// What size is copied from the anchor widget and used as the available size and final size of the widget.
    pub size: AnchorSize,
    /// After the `transform` and `size` are resolved the transform is adjusted so that the layered widget is
    /// fully visible in the window.
    pub viewport_bound: bool,

    /// If the widget is only layout if the anchor widget is not [`Collapsed`] and is only rendered
    /// if the anchor widget is rendered.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub visibility: bool,
    /// The widget [`interactivity`] is set to the the same as the anchor widget.
    ///
    /// [`interactivity`]: crate::core::widget_info::WidgetInfo::interactivity
    pub interactivity: bool,

    /// The widget's corner radius is set for the layer.
    ///
    /// If `size` is [`InnerBorder`] the corner radius are deflated to fit the *inner* curve of the borders.
    ///
    /// [`InnerBorder`]: AnchorSize::InnerBorder
    pub corner_radius: bool,
}
impl AnchorMode {
    /// Mode where widget behaves like an unanchored widget, except that it is still only
    /// layout an rendered if the anchor widget exists in the same window.
    pub fn none() -> Self {
        AnchorMode {
            transform: AnchorTransform::None,
            size: AnchorSize::Window,
            viewport_bound: false,
            visibility: false,
            interactivity: false,
            corner_radius: false,
        }
    }

    /// Mode where the widget behaves like a [`foreground`] to the target widget.
    ///
    /// [`foreground`]: fn@crate::properties::foreground
    pub fn foreground() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerTransform,
            size: AnchorSize::InnerSize,
            visibility: true,
            viewport_bound: false,
            interactivity: false,
            corner_radius: true,
        }
    }

    /// Mode where widget behaves like a flyout menu for the anchor.
    pub fn popup(place: AnchorOffset) -> Self {
        AnchorMode {
            transform: place.into(),
            size: AnchorSize::InnerSize,
            visibility: true,
            viewport_bound: true,
            interactivity: true,
            corner_radius: true,
        }
    }

    /// Returns the mode with `transform` set.
    pub fn with_transform(mut self, transform: impl Into<AnchorTransform>) -> Self {
        self.transform = transform.into();
        self
    }

    /// Returns the mode with `size` set.
    pub fn with_size(mut self, size: impl Into<AnchorSize>) -> Self {
        self.size = size.into();
        self
    }

    /// Returns the mode with `visibility` set.
    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    /// Returns the mode with `interactivity` set.
    pub fn with_interactivity(mut self, interactivity: bool) -> Self {
        self.interactivity = interactivity;
        self
    }

    /// Returns the mode with `corner_radius` set.
    pub fn with_corner_radius(mut self, corner_radius: bool) -> Self {
        self.corner_radius = corner_radius;
        self
    }

    /// Returns the mode with `viewport_bound` set.
    pub fn with_viewport_bound(mut self, viewport_bound: bool) -> Self {
        self.viewport_bound = viewport_bound;
        self
    }
}
impl Default for AnchorMode {
    /// Transform `InnerOffset` top-left, size infinite, copy visibility and corner-radius.
    fn default() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerOffset(AnchorOffset::in_top_left()),
            size: AnchorSize::Unbounded,
            viewport_bound: false,
            visibility: true,
            interactivity: false,
            corner_radius: true,
        }
    }
}
impl_from_and_into_var! {
    /// Custom transform, all else default.
    fn from(transform: AnchorTransform) -> AnchorMode {
        AnchorMode {
            transform,
            ..AnchorMode::default()
        }
    }
    /// Transform `InnerOffset`, all else default.
    fn from(inner_offset: AnchorOffset) -> AnchorMode {
        AnchorTransform::from(inner_offset).into()
    }

    /// Custom transform and size, all else default.
    fn from<T: Into<AnchorTransform>, S: Into<AnchorSize>>((transform, size): (T, S)) -> AnchorMode {
        AnchorMode {
            transform: transform.into(),
            size: size.into(),
            ..AnchorMode::default()
        }
    }
}

pub(super) fn node(child: impl UiNode) -> impl UiNode {
    type SortFn = fn(&mut BoxedUiNode, &mut BoxedUiNode) -> std::cmp::Ordering;

    let layers = EditableUiNodeList::new();
    let layered = layers.reference();

    let sorting_layers = SortingList::<_, SortFn>::new(layers, |a, b| {
        let a = a
            .with_context(WidgetUpdateMode::Ignore, || WIDGET.req_state(&LAYER_INDEX_ID))
            .unwrap_or(LayerIndex::DEFAULT);
        let b = b
            .with_context(WidgetUpdateMode::Ignore, || WIDGET.req_state(&LAYER_INDEX_ID))
            .unwrap_or(LayerIndex::DEFAULT);

        a.cmp(&b)
    });
    let children = ui_vec![child].chain(sorting_layers);

    match_node_list(children, move |c, op| match op {
        UiNodeOp::Init => {
            WINDOW.with_state_mut(|mut s| {
                s.set(&WINDOW_LAYERS_ID, LayersCtx { items: layered.clone() });

                if let Some(widgets) = s.get_mut(&WINDOW_PRE_INIT_LAYERS_ID) {
                    for wgt in widgets.drain(..) {
                        layered.push(wgt.into_inner());
                    }
                }
            });
        }
        UiNodeOp::Update { updates } => {
            let mut changed = false;
            {
                let editable_list = c.children().1.list();

                let mut retains = editable_list.take_retain_requests();

                if !retains.is_empty() {
                    editable_list.retain_mut(|n| {
                        enum Action {
                            Remove,
                            Event,
                            Retain,
                        }
                        let remove_requested = retains.iter_mut().any(|r| !r(n));
                        let action = n
                            .with_context(WidgetUpdateMode::Bubble, || {
                                if remove_requested {
                                    if WIDGET.get_state(&HAS_LAYER_REMOVE_HANDLERS_ID).is_some() {
                                        Action::Event
                                    } else {
                                        Action::Remove
                                    }
                                } else {
                                    Action::Retain
                                }
                            })
                            .unwrap_or(if remove_requested { Action::Remove } else { Action::Retain });

                        match action {
                            Action::Remove => {
                                n.deinit();
                                WIDGET.info();
                                changed = true;
                                false
                            }
                            Action::Event => {
                                let args = LayerRemoveRequestedArgs::now(layered.clone());
                                let propagation = args.propagation().clone();
                                let mut delivery_list = UpdateDeliveryList::new_any();
                                n.with_context(WidgetUpdateMode::Bubble, || {
                                    delivery_list.insert_wgt(&WIDGET.info());
                                });
                                n.event(&LAYER_REMOVE_REQUESTED_EVENT.new_update_custom(args, delivery_list));
                                if propagation.is_stopped() {
                                    true
                                } else {
                                    n.deinit();
                                    WIDGET.info();
                                    changed = true;
                                    false
                                }
                            }
                            Action::Retain => true,
                        }
                    });
                }
            }

            c.update_all(updates, &mut changed);

            if changed {
                WIDGET.layout().render();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = c.with_node(0, |n| n.measure(wm));
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.with_node(0, |n| n.layout(wl));
            let _ = c.children().1.layout_each(wl, |_, l, wl| l.layout(wl), |_, _| PxSize::zero());
        }
        UiNodeOp::Render { frame } => {
            c.with_node(0, |n| n.render(frame));
            c.children().1.render_all(frame);
        }
        UiNodeOp::RenderUpdate { update } => {
            c.with_node(0, |n| n.render_update(update));
            c.children().1.render_update_all(update);
        }
        _ => {}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn layer_index_ops() {
        let idx = LayerIndex::DEFAULT;

        let p1 = idx + 1;
        let m1 = idx - 1;

        let mut idx = idx;

        idx += 1;
        assert_eq!(idx, p1);

        idx -= 2;
        assert_eq!(idx, m1);
    }
}
