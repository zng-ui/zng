#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Window layers and popup.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use parking_lot::Mutex;
use zng_app::widget::border::CORNER_RADIUS_VAR;
use zng_app::widget::info::WIDGET_INFO_CHANGED_EVENT;
use zng_ext_input::mouse::MOUSE;
use zng_ext_input::touch::TOUCH;
use zng_ext_window::WINDOW_Ext as _;
use zng_var::{ContextInitHandle, animation};
use zng_view_api::window::FrameId;
use zng_wgt::prelude::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fmt, mem, ops};

pub mod popup;

struct LayersCtx {
    items: EditableUiVecRef,
}

command! {
    /// Insert a layer widget on the scoped window.
    ///
    /// # Params
    ///
    /// The parameter must be a tuple with the `LAYERS` service inputs:
    ///
    /// * `(LayerIndex, WidgetFn<()>)` - Calls the widget function in the window context, then calls [`LAYERS.insert`].
    /// * `(LayerIndex, WidgetId, AnchorMode, WidgetFn<()>)` - Calls the widget function in the window context,
    ///    then calls [`LAYERS.insert_anchored`].
    ///
    /// If the parameter type does not match any of the above a debug trace is logged.
    ///
    /// [`LAYERS.insert`]: LAYERS::insert
    /// [`LAYERS.insert_anchored`]: LAYERS::insert_anchored
    pub static LAYERS_INSERT_CMD;

    /// Remove a layer widget on the scoped window.
    ///
    /// # Params
    ///
    /// * `WidgetId` - Calls [`LAYERS.remove`].
    ///
    /// If the parameter type does not match any of the above a debug trace is logged.
    ///
    /// [`LAYERS.remove`]: LAYERS::remove
    pub static LAYERS_REMOVE_CMD;
}

/// Windows layers.
///
/// The window layers is a z-order stacking panel that fills the window content area, widgets can be inserted
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
/// Note that this single pass behavior only works automatically in the [`AnchorMode`], to implement custom
/// sizing and positioning based on the anchor you must wrap the layered widget with a custom widget node, this
/// is because the default widget implementation skips layout and render when it was not requested for the widget
/// or descendants. See the [`insert_anchored`] source code for an example.
///
///
/// [`insert_anchored`]: Self::insert_anchored
/// [`WidgetBoundsInfo`]: zng_wgt::prelude::WidgetBoundsInfo
pub struct LAYERS;
impl LAYERS {
    /// Insert the `widget` in the layer identified by a [`LayerIndex`].
    ///
    /// If the `layer` variable updates the widget is moved to the new layer, if multiple widgets
    /// are inserted in the same layer the later inserts are on top of the previous.
    ///
    /// If the `widget` node is not a full widget after init it is immediately deinited and removed. Only full
    /// widgets are allowed, use this method when you know the node is a widget and know the widget ID so it can
    /// be removed later. Use [`insert_node`] to insert nodes that may not always be widgets.
    ///
    /// [`insert_node`]: Self::insert_node
    pub fn insert(&self, layer: impl IntoVar<LayerIndex>, widget: impl IntoUiNode) {
        let layer = layer.into_var().current_context();
        self.insert_impl(layer, widget.into_node());
    }
    fn insert_impl(&self, layer: Var<LayerIndex>, widget: UiNode) {
        let widget = match_widget(widget, move |widget, op| match op {
            UiNodeOp::Init => {
                widget.init();

                // widget may only become a full widget after init (ArcNode)

                if let Some(mut wgt) = widget.node().as_widget() {
                    wgt.with_context(WidgetUpdateMode::Bubble, || {
                        WIDGET.set_state(*LAYER_INDEX_ID, layer.get());
                        WIDGET.sub_var(&layer);
                    });
                } else {
                    *widget.node() = UiNode::nil();
                    LAYERS.cleanup();
                }
            }
            UiNodeOp::Update { .. } => {
                if let Some(mut wgt) = widget.node().as_widget()
                    && let Some(index) = layer.get_new()
                {
                    wgt.with_context(WidgetUpdateMode::Bubble, || {
                        WIDGET.set_state(*LAYER_INDEX_ID, index);
                        SORTING_LIST.invalidate_sort();
                    });
                }
            }
            _ => {}
        });

        let r = WINDOW.with_state(|s| match s.get(*WINDOW_LAYERS_ID) {
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
                s.entry(*WINDOW_PRE_INIT_LAYERS_ID).or_default().push(Mutex::new(widget));
            });
        }
    }

    /// Like [`insert`], but does not fail if `maybe_widget` is not a full widget.
    ///
    /// If the `maybe_widget` is not a full widget after the first init, it is upgraded to a full widget. The
    /// widget ID (existing or upgraded) is set on a response var that can be used to remove the node.
    ///
    /// This is the equivalent of calling [`insert`] with the node wrapped in [`UiNode::init_widget`].
    ///
    /// [`insert`]: Self::insert
    /// [`UiNode::init_widget`]: zng_wgt::prelude::UiNode::init_widget
    pub fn insert_node(&self, layer: impl IntoVar<LayerIndex>, maybe_widget: impl IntoUiNode) -> ResponseVar<WidgetId> {
        let (widget, rsp) = maybe_widget.into_node().init_widget();
        self.insert(layer, widget);
        rsp
    }

    /// Insert the `widget` in the layer and *anchor* it to the offset/transform of another widget.
    ///
    /// The `anchor` is the ID of another widget, the inserted `widget` will be offset/transform so that it aligns
    /// with the `anchor` widget top-left. The `mode` is a value of [`AnchorMode`] that defines if the `widget` will
    /// receive the full transform or just the offset.
    ///
    /// If the `anchor` widget is not found the `widget` is anchored to the window root. If the `widget`
    /// is not a full widget after init it is immediately deinited and removed. If you don't
    /// know the widget ID use [`insert_anchored_node`] instead to receive the ID so the layer can be removed.
    ///
    /// [`insert_anchored_node`]: Self::insert_anchored_node
    pub fn insert_anchored(
        &self,
        layer: impl IntoVar<LayerIndex>,
        anchor: impl IntoVar<WidgetId>,
        mode: impl IntoVar<AnchorMode>,

        widget: impl IntoUiNode,
    ) {
        let layer = layer.into_var().current_context();
        let anchor = anchor.into_var().current_context();
        let mode = mode.into_var().current_context();

        self.insert_anchored_impl(layer, anchor, mode, widget.into_node())
    }
    fn insert_anchored_impl(&self, layer: Var<LayerIndex>, anchor: Var<WidgetId>, mode: Var<AnchorMode>, widget: UiNode) {
        let mut _info_changed_handle = None;
        let mut mouse_pos_handle = None;

        let mut cursor_once_pending = false;
        let mut anchor_info = None;
        let mut offset = (PxPoint::zero(), PxPoint::zero());
        let mut cursor_bounds = None;
        let mut interactivity = false;

        let transform_key = FrameValueKey::new_unique();
        let mut corner_radius_ctx_handle = None;

        let widget = with_anchor_id(widget, anchor.clone());

        fn get_anchor_info(anchor: WidgetId) -> (WidgetBoundsInfo, WidgetBorderInfo) {
            let tree = WINDOW.info();
            let w = tree.get(anchor).unwrap_or_else(|| tree.root());
            (w.bounds_info(), w.border_info())
        }

        let widget = match_widget(widget, move |widget, op| match op {
            UiNodeOp::Init => {
                widget.init();

                if let Some(mut wgt) = widget.node().as_widget() {
                    wgt.with_context(WidgetUpdateMode::Bubble, || {
                        WIDGET.sub_var(&anchor).sub_var(&mode);

                        anchor_info = Some(get_anchor_info(anchor.get()));

                        interactivity = mode.with(|m| m.interactivity);
                        _info_changed_handle = Some(WIDGET_INFO_CHANGED_EVENT.subscribe(WIDGET.id()));

                        if mode.with(|m| matches!(&m.transform, AnchorTransform::Cursor { .. })) {
                            mouse_pos_handle = Some(MOUSE.position().subscribe(UpdateOp::Update, WIDGET.id()));
                        } else if mode.with(|m| matches!(&m.transform, AnchorTransform::CursorOnce { .. })) {
                            cursor_once_pending = true;
                        }
                    })
                } else {
                    widget.deinit();
                    *widget.node() = UiNode::nil();
                    // cleanup requested by the `insert` node.
                }
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
                if interactivity && let Some(mut wgt) = widget.node().as_widget() {
                    let wgt = wgt.id();
                    let anchor = anchor.get();
                    let querying = AtomicBool::new(false);
                    info.push_interactivity_filter(move |args| {
                        if args.info.id() == wgt {
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
            UiNodeOp::Event { update } => {
                if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
                    if args.window_id == WINDOW.id() {
                        anchor_info = Some(get_anchor_info(anchor.get()));
                    }
                }
            }
            UiNodeOp::Update { .. } => {
                if let Some(mut wgt) = widget.node().as_widget() {
                    wgt.with_context(WidgetUpdateMode::Bubble, || {
                        if let Some(anchor) = anchor.get_new() {
                            anchor_info = Some(get_anchor_info(anchor));
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
                            if matches!(&mode.transform, AnchorTransform::Cursor { .. }) {
                                if mouse_pos_handle.is_none() {
                                    mouse_pos_handle = Some(MOUSE.position().subscribe(UpdateOp::Update, WIDGET.id()));
                                }
                                cursor_once_pending = false;
                            } else {
                                cursor_once_pending = matches!(&mode.transform, AnchorTransform::CursorOnce { .. });
                                mouse_pos_handle = None;
                            }
                            WIDGET.layout().render();
                        } else if mouse_pos_handle.is_some() && MOUSE.position().is_new() {
                            WIDGET.layout();
                        }
                    })
                }
            }
            UiNodeOp::Measure { wm, desired_size } => {
                widget.delegated();

                if let Some((bounds, border)) = &anchor_info {
                    let mode = mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        let mut constraints = match mode.min_size {
                            AnchorSize::Unbounded => PxConstraints2d::new_unbounded(),
                            AnchorSize::Window => LAYOUT.constraints(),
                            AnchorSize::InnerSize => PxConstraints2d::new_exact_size(bounds.inner_size()).with_fill(false, false),
                            AnchorSize::InnerBorder => PxConstraints2d::new_exact_size(border.inner_size(bounds)).with_fill(false, false),
                            AnchorSize::OuterSize => PxConstraints2d::new_exact_size(bounds.outer_size()).with_fill(false, false),
                        };
                        if mode.max_size != mode.min_size {
                            constraints = match mode.max_size {
                                AnchorSize::Unbounded => constraints.with_unbounded(),
                                AnchorSize::Window => {
                                    let w = LAYOUT.constraints();
                                    constraints.with_new_max(w.x.max().unwrap_or(Px::MAX), w.y.max().unwrap_or(Px::MAX))
                                }
                                AnchorSize::InnerSize => constraints.with_new_max_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => constraints.with_new_max_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => constraints.with_new_max_size(bounds.outer_size()),
                            };
                        }

                        *desired_size = LAYOUT.with_constraints(constraints, || widget.measure(wm));
                    }
                }
            }
            UiNodeOp::Layout { wl, final_size } => {
                widget.delegated();

                if let Some((bounds, border)) = &anchor_info {
                    let mode = mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        // if we don't link visibility or anchor is not collapsed.

                        let mut constraints = match mode.min_size {
                            AnchorSize::Unbounded => PxConstraints2d::new_unbounded(),
                            AnchorSize::Window => LAYOUT.constraints(),
                            AnchorSize::InnerSize => PxConstraints2d::new_exact_size(bounds.inner_size()).with_fill(false, false),
                            AnchorSize::InnerBorder => PxConstraints2d::new_exact_size(border.inner_size(bounds)).with_fill(false, false),
                            AnchorSize::OuterSize => PxConstraints2d::new_exact_size(bounds.outer_size()).with_fill(false, false),
                        };
                        if mode.max_size != mode.min_size {
                            constraints = match mode.max_size {
                                AnchorSize::Unbounded => constraints.with_unbounded(),
                                AnchorSize::Window => {
                                    let w = LAYOUT.constraints();
                                    constraints.with_new_max(w.x.max().unwrap_or(Px::MAX), w.y.max().unwrap_or(Px::MAX))
                                }
                                AnchorSize::InnerSize => constraints.with_new_max_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => constraints.with_new_max_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => constraints.with_new_max_size(bounds.outer_size()),
                            };
                        }

                        let layer_size = LAYOUT.with_constraints(constraints, || {
                            if mode.corner_radius {
                                let mut cr = border.corner_radius();
                                if let AnchorSize::InnerBorder = mode.max_size {
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
                        });

                        if let Some((p, include_touch, bounded, update)) = match &mode.transform {
                            AnchorTransform::Cursor {
                                offset,
                                include_touch,
                                bounds,
                            } => Some((offset, include_touch, bounds, true)),
                            AnchorTransform::CursorOnce {
                                offset,
                                include_touch,
                                bounds,
                            } => Some((offset, include_touch, bounds, mem::take(&mut cursor_once_pending))),
                            _ => None,
                        } {
                            // cursor transform mode, only visible if cursor over window
                            const NO_POS_X: Px = Px::MIN;
                            if update {
                                let pos = if *include_touch {
                                    let oldest_touch = TOUCH.positions().with(|p| p.iter().min_by_key(|p| p.start_time).cloned());
                                    match (oldest_touch, MOUSE.position().get()) {
                                        (Some(t), Some(m)) => {
                                            let window_id = WINDOW.id();
                                            if t.window_id == window_id && m.window_id == window_id {
                                                Some(if t.update_time > m.timestamp { t.position } else { m.position })
                                            } else {
                                                None
                                            }
                                        }
                                        (Some(t), None) => {
                                            if t.window_id == WINDOW.id() {
                                                Some(t.position)
                                            } else {
                                                None
                                            }
                                        }
                                        (None, Some(m)) => {
                                            if m.window_id == WINDOW.id() {
                                                Some(m.position)
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    }
                                } else if let Some(p) = MOUSE.position().get() {
                                    if p.window_id == WINDOW.id() { Some(p.position) } else { None }
                                } else {
                                    None
                                };

                                if let Some(pos) = pos {
                                    let fct = LAYOUT.scale_factor();
                                    let pos = pos.to_px(fct);

                                    let (cursor_size, cursor_spot) = {
                                        let vars = WINDOW.vars();
                                        if let Some((img, spot)) = vars.actual_cursor_img().get() {
                                            (img.size(), spot)
                                        } else {
                                            vars.cursor().with(|s| s.icon()).map(|i| i.size_and_spot(fct)).unwrap_or_default()
                                        }
                                    };
                                    let cursor_rect = PxRect::new((pos - cursor_spot).to_point(), cursor_size);
                                    let place = cursor_rect.origin
                                        + LAYOUT
                                            .with_constraints(PxConstraints2d::new_exact_size(cursor_rect.size), || p.place.layout())
                                            .to_vector();
                                    let origin = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(layer_size), || p.origin.layout());

                                    if let Some(sides) = bounded {
                                        let sides = LAYOUT
                                            .with_constraints(PxConstraints2d::new_exact_size(bounds.inner_size()), || sides.layout());
                                        // render will transform this to anchor and apply to place point
                                        cursor_bounds = Some(PxRect::new(
                                            -PxPoint::new(sides.left, sides.top),
                                            bounds.inner_size() + PxSize::new(sides.horizontal(), sides.vertical()),
                                        ));
                                    } else {
                                        cursor_bounds = None;
                                    }

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
                        }
                        return;
                    }
                }

                if let Some(mut wgt) = widget.node().as_widget() {
                    wgt.with_context(WidgetUpdateMode::Bubble, || {
                        wl.collapse();
                    });
                }
            }
            UiNodeOp::Render { frame } => {
                widget.delegated();

                if let Some((bounds_info, border_info)) = &anchor_info {
                    let mode = mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        let mut push_reference_frame = |mut transform: PxTransform, is_translate_only: bool| {
                            if mode.viewport_bound {
                                transform = adjust_viewport_bound(transform, widget.node());
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
                            AnchorTransform::Cursor { .. } | AnchorTransform::CursorOnce { .. } => {
                                let (mut place, origin) = offset;

                                if let Some(b) = cursor_bounds {
                                    // transform `place` to bounds space, clamp to bounds, transform back to window space.
                                    let transform = bounds_info.inner_transform();
                                    if let Some(inverse) = transform.inverse() {
                                        if let Some(p) = inverse.transform_point(place) {
                                            let bound_p = PxPoint::new(p.x.clamp(b.min_x(), b.max_x()), p.y.clamp(b.min_y(), b.max_y()));
                                            if p != bound_p {
                                                if let Some(p) = transform.transform_point(bound_p) {
                                                    place = p;
                                                }
                                            }
                                        }
                                    }
                                }
                                let offset = place - origin;

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
                    } else {
                        // anchor not visible, call render to properly hide or collapse (if collapsed during layout)
                        frame.hide(|frame| widget.render(frame));

                        if frame.frame_id() == FrameId::first() && anchor.get() == WIDGET.id() {
                            // anchor is the root widget, the only widget that is not done rendering before the layers
                            // if only the first frame is rendered the layer can remain hidden, so ensure a second frame renders.
                            WIDGET.render();
                        }
                    }
                } else {
                    widget.render(frame);
                }
            }
            UiNodeOp::RenderUpdate { update } => {
                if let Some((bounds_info, border_info)) = &anchor_info {
                    let mode = mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        let mut with_transform = |mut transform: PxTransform| {
                            if mode.viewport_bound {
                                transform = adjust_viewport_bound(transform, widget.node());
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
                            AnchorTransform::Cursor { .. } | AnchorTransform::CursorOnce { .. } => {
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
        self.insert_impl(layer, widget);
    }

    /// Like [`insert_anchored`], but does not fail if `maybe_widget` is not a full widget.
    ///
    /// If the `maybe_widget` is not a full widget after the first init, it is upgraded to a full widget. The
    /// widget ID is set on a response var that can be used to remove the node.
    ///
    /// This is the equivalent of calling [`insert_anchored`] with the node wrapped in [`UiNode::init_widget`].
    ///
    /// [`insert`]: Self::insert
    ///
    /// [`insert_anchored`]: Self::insert_anchored
    /// [`UiNode::init_widget`]: zng_wgt::prelude::UiNode::init_widget
    pub fn insert_anchored_node(
        &self,
        layer: impl IntoVar<LayerIndex>,
        anchor: impl IntoVar<WidgetId>,
        mode: impl IntoVar<AnchorMode>,

        maybe_widget: impl IntoUiNode,
    ) -> ResponseVar<WidgetId> {
        let (widget, rsp) = maybe_widget.into_node().init_widget();
        self.insert_anchored(layer, anchor, mode, widget);
        rsp
    }

    /// Remove the widget in the next update.
    ///
    /// The `id` must the widget id of a previous inserted widget, nothing happens if the widget is not found.
    ///
    /// See also [`remove_node`] for removing nodes inserted by `_node` variants.
    ///
    /// [`remove_node`]: Self::remove_node
    pub fn remove(&self, id: impl Into<WidgetId>) {
        WINDOW.with_state(|s| {
            s.req(*WINDOW_LAYERS_ID).items.remove(id);
        });
    }

    /// Remove the widget in the next update.
    ///
    /// If the `id` has not responded yet it will be removed as soon as it initializes. This can happen if
    /// the remove request is made before an update cycle allows time for the inserted widget first init.
    pub fn remove_node(&self, id: ResponseVar<WidgetId>) {
        if let Some(id) = id.rsp() {
            self.remove(id);
        } else {
            let items = WINDOW.with_state(|s| s.req(*WINDOW_LAYERS_ID).items.clone());
            id.hook(move |a| {
                match a.value() {
                    zng_var::Response::Waiting => true,
                    zng_var::Response::Done(id) => {
                        // remove item and hook
                        items.remove(*id);
                        false
                    }
                }
            })
            .perm();
        }
    }

    /// Gets a read-only var that tracks the anchor widget in a layered widget context.
    pub fn anchor_id(&self) -> Var<Option<WidgetId>> {
        ANCHOR_ID_VAR.read_only()
    }

    fn cleanup(&self) {
        WINDOW.with_state(|s| {
            s.req(*WINDOW_LAYERS_ID).items.retain(|n| n.as_widget().is_some());
        });
    }
}

fn adjust_viewport_bound(transform: PxTransform, widget: &mut UiNode) -> PxTransform {
    let window_bounds = WINDOW.vars().actual_size_px().get();
    let wgt_bounds = PxBox::from(
        widget
            .as_widget()
            .map(|mut w| w.with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().outer_size()))
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

fn with_anchor_id(child: impl IntoUiNode, anchor: Var<WidgetId>) -> UiNode {
    let mut ctx = Some(Arc::new(anchor.map(|id| Some(*id)).into()));
    let mut id = None;
    match_widget(child, move |c, op| {
        let mut is_deinit = false;
        match &op {
            UiNodeOp::Init => {
                id = Some(ContextInitHandle::new());
            }
            UiNodeOp::Deinit => {
                is_deinit = true;
            }
            _ => {}
        }
        ANCHOR_ID_VAR.with_context(id.clone().expect("node not inited"), &mut ctx, || c.op(op));

        if is_deinit {
            id = None;
        }
    })
}

context_var! {
    static ANCHOR_ID_VAR: Option<WidgetId> = None;
}

static_id! {
    static ref WINDOW_PRE_INIT_LAYERS_ID: StateId<Vec<Mutex<UiNode>>>;
    static ref WINDOW_LAYERS_ID: StateId<LayersCtx>;
    static ref LAYER_INDEX_ID: StateId<LayerIndex>;
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
    /// drop-downs, pop-ups and tooltips.
    ///
    /// This is the [`u32::MAX`] value.
    pub const TOP_MOST: LayerIndex = LayerIndex(u32::MAX);

    /// The layer for *adorner* display items.
    ///
    /// Adorner widgets are related to another widget but not as a visual part of it, examples of adorners
    /// are resize handles in a widget visual editor, or an interactive help/guide feature.
    ///
    /// This is the `TOP_MOST - u16::MAX` value.
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
            write!(f, "{name}")
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
    Unnamed(u32),
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
        LayerIndexSerde::Unnamed(self.0).serialize(serializer)
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
            LayerIndexSerde::Unnamed(i) => Ok(Self(i)),
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
    fn lerp(self, to: &Self, step: animation::easing::EasingStep) -> Self {
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
    CursorOnce {
        /// The anchor offset place point is resolved in the cursor icon size (approximate).
        offset: AnchorOffset,
        /// If the latest touch position counts as a cursor.
        ///
        /// If `true` the latest position between mouse move and touch start or move is used, if `false`
        /// only the latest mouse position is used. Only active touch points count, that is touch start or
        /// move events only.
        include_touch: bool,

        /// If set defines the offset from the anchor widget inner bounds that is the allowed
        /// area for the layer widget origin.
        ///
        /// Negative offsets are inside the inner bounds, positive outside.
        bounds: Option<SideOffsets>,
    },
    /// The layer widget is translated to follow the cursor position.
    ///
    /// The anchor offset place point is resolved in the cursor icon size (approximate).
    Cursor {
        /// The anchor offset place point is resolved in the cursor icon size (approximate), or in touch point pixel
        /// for touch positions.
        offset: AnchorOffset,

        /// If the latest touch position counts as a cursor.
        ///
        /// If `true` the latest position between mouse move and touch start or move is used, if `false`
        /// only the latest mouse position is used. Only active touch points count, that is touch start or
        /// move events only. In case multiple touches are active only the first one counts.
        include_touch: bool,

        /// If set defines the offset from the anchor widget inner bounds that is the allowed
        /// area for the layer widget origin.
        ///
        /// Negative offsets are inside the inner bounds, positive outside.
        bounds: Option<SideOffsets>,
    },
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

/// Options for [`AnchorMode`] size constraints.
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

    /// What size is copied from the anchor widget and used as the available and final min size of the widget.
    pub min_size: AnchorSize,
    /// What size is copied from the anchor widget and used as the available and final max size of the widget.
    pub max_size: AnchorSize,

    /// After the `transform` and `size` are resolved the transform is adjusted so that the layered widget is
    /// fully visible in the window.
    ///
    /// Has no effect if [`AnchorTransform::None`].
    pub viewport_bound: bool,

    /// If the widget is only layout if the anchor widget is not [`Collapsed`] and is only rendered
    /// if the anchor widget is rendered.
    ///
    /// [`Collapsed`]: Visibility::Collapsed
    pub visibility: bool,
    /// The widget [`interactivity`] is set to the same as the anchor widget.
    ///
    /// [`interactivity`]: zng_app::widget::info::WidgetInfo::interactivity
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
    pub fn window() -> Self {
        AnchorMode {
            transform: AnchorTransform::None,
            min_size: AnchorSize::Window,
            max_size: AnchorSize::Window,
            viewport_bound: false,
            visibility: false,
            interactivity: false,
            corner_radius: false,
        }
    }

    /// Mode where the widget behaves like a `foreground` to the target widget.
    pub fn foreground() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerTransform,
            min_size: AnchorSize::InnerSize,
            max_size: AnchorSize::InnerSize,
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
            min_size: AnchorSize::InnerSize,
            max_size: AnchorSize::Window,
            visibility: true,
            viewport_bound: true,
            interactivity: true,
            corner_radius: false,
        }
    }

    /// Mode where the widget behaves like a tooltip anchored to the cursor.
    pub fn tooltip() -> Self {
        AnchorMode {
            transform: AnchorTransform::CursorOnce {
                offset: AnchorOffset::out_bottom_in_left(),
                include_touch: true,
                bounds: None,
            },
            min_size: AnchorSize::Unbounded,
            max_size: AnchorSize::Window,
            viewport_bound: true,
            corner_radius: false,
            visibility: true,
            interactivity: false,
        }
    }

    /// Mode where the widget behaves like a tooltip anchored to the widget.
    pub fn tooltip_shortcut() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerOffset({
                let mut p = AnchorOffset::out_top();
                p.origin.y += 4;
                p
            }),
            min_size: AnchorSize::Unbounded,
            max_size: AnchorSize::Window,
            viewport_bound: true,
            corner_radius: false,
            visibility: true,
            interactivity: false,
        }
    }

    /// Mode where the widget behaves like a context-menu anchored to the cursor.
    pub fn context_menu() -> Self {
        AnchorMode {
            transform: AnchorTransform::CursorOnce {
                offset: AnchorOffset::in_top_left(),
                include_touch: true,
                bounds: None,
            },
            min_size: AnchorSize::Unbounded,
            max_size: AnchorSize::Window,
            viewport_bound: true,
            corner_radius: false,
            visibility: true,
            interactivity: false,
        }
    }

    /// Mode where the widget behaves like a context-menu anchored to widget.
    pub fn context_menu_shortcut() -> Self {
        AnchorMode {
            transform: AnchorTransform::InnerOffset(AnchorOffset::in_top()),
            min_size: AnchorSize::Unbounded,
            max_size: AnchorSize::Window,
            viewport_bound: true,
            corner_radius: false,
            visibility: true,
            interactivity: false,
        }
    }

    /// Returns the mode with `transform` set.
    pub fn with_transform(mut self, transform: impl Into<AnchorTransform>) -> Self {
        self.transform = transform.into();
        self
    }

    /// Returns the mode with `min_size` set.
    pub fn with_min_size(mut self, size: impl Into<AnchorSize>) -> Self {
        self.min_size = size.into();
        self
    }

    /// Returns the mode with `max_size` set.
    pub fn with_max_size(mut self, size: impl Into<AnchorSize>) -> Self {
        self.max_size = size.into();
        self
    }

    /// Returns the mode with `min_size` and `max_size` set.
    pub fn with_size(mut self, size: impl Into<AnchorSize>) -> Self {
        let size = size.into();
        self.min_size = size;
        self.max_size = size;
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
            min_size: AnchorSize::Unbounded,
            max_size: AnchorSize::Unbounded,
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
        let size = size.into();
        AnchorMode {
            transform: transform.into(),
            min_size: size,
            max_size: size,
            ..AnchorMode::default()
        }
    }
}

/// Node that implements the layers, must be inserted in the [`NestGroup::EVENT`] group by the window implementer.
///
/// [`NestGroup::EVENT`]: zng_app::widget::builder::NestGroup::EVENT
pub fn layers_node(child: impl IntoUiNode) -> UiNode {
    let layers = EditableUiVec::new();
    let layered = layers.reference();

    fn sort(a: &mut UiNode, b: &mut UiNode) -> std::cmp::Ordering {
        let a = a
            .as_widget()
            .map(|mut w| w.with_context(WidgetUpdateMode::Ignore, || WIDGET.req_state(*LAYER_INDEX_ID)))
            .unwrap_or(LayerIndex::DEFAULT);
        let b = b
            .as_widget()
            .map(|mut w| w.with_context(WidgetUpdateMode::Ignore, || WIDGET.req_state(*LAYER_INDEX_ID)))
            .unwrap_or(LayerIndex::DEFAULT);

        a.cmp(&b)
    }
    let sorting_layers = SortingList::new(layers, sort);
    let children = ChainList(ui_vec![child, sorting_layers]);

    let mut _insert_handle = CommandHandle::dummy();
    let mut _remove_handle = CommandHandle::dummy();

    match_node(children, move |c, op| match op {
        UiNodeOp::Init => {
            WINDOW.with_state_mut(|mut s| {
                s.set(*WINDOW_LAYERS_ID, LayersCtx { items: layered.clone() });

                if let Some(widgets) = s.get_mut(*WINDOW_PRE_INIT_LAYERS_ID) {
                    for wgt in widgets.drain(..) {
                        layered.push(wgt.into_inner());
                    }
                }
            });
            _insert_handle = LAYERS_INSERT_CMD.scoped(WINDOW.id()).subscribe(true);
            _remove_handle = LAYERS_REMOVE_CMD.scoped(WINDOW.id()).subscribe(true);
        }
        UiNodeOp::Deinit => {
            _insert_handle = CommandHandle::dummy();
            _remove_handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if let Some(args) = LAYERS_INSERT_CMD.scoped(WINDOW.id()).on_unhandled(update) {
                if let Some((layer, widget)) = args.param::<(LayerIndex, WidgetFn<()>)>() {
                    LAYERS.insert(*layer, widget(()));
                    args.propagation().stop();
                } else if let Some((layer, anchor, mode, widget)) = args.param::<(LayerIndex, WidgetId, AnchorMode, WidgetFn<()>)>() {
                    LAYERS.insert_anchored(*layer, *anchor, mode.clone(), widget(()));
                    args.propagation().stop();
                } else {
                    tracing::debug!("ignoring LAYERS_INSERT_CMD, unknown param type");
                }
            } else if let Some(args) = LAYERS_REMOVE_CMD.scoped(WINDOW.id()).on_unhandled(update) {
                if let Some(id) = args.param::<WidgetId>() {
                    LAYERS.remove(*id);
                } else {
                    tracing::debug!("ignoring LAYERS_REMOVE_CMD, unknown param type");
                }
            }
        }
        UiNodeOp::Update { updates } => {
            let mut changed = false;
            c.update_list(updates, &mut changed);

            if changed {
                WIDGET.layout().render();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            c.delegated();
            *desired_size = c.node_impl::<ChainList>().0[0].measure(wm);
        }
        UiNodeOp::Layout { wl, final_size } => {
            c.delegated();
            let list = c.node_impl::<ChainList>();
            *final_size = list.0[0].layout(wl);
            let _ = list.0[1].layout_list(
                wl,
                |i, l, wl| if i > 0 { l.layout(wl) } else { PxSize::zero() },
                |_, _| PxSize::zero(),
            );
        }
        UiNodeOp::Render { frame } => {
            c.delegated();
            let list = c.node_impl::<ChainList>();
            list.0[0].render(frame); // render main UI first
            list.0[1].render(frame);
        }
        UiNodeOp::RenderUpdate { update } => {
            c.delegated();
            let list = c.node_impl::<ChainList>();
            list.0[0].render_update(update);
            list.0[1].render_update(update);
        }
        _ => {}
    })
}

/// Custom layered foreground generated using a [`WidgetFn<()>`].
///
/// If the `adorner_fn` is not nil, the generated node is [layered] anchored to the widget inner bounds,
/// displaying like a `foreground` that is not clipped by the widget and overlays all other widgets
/// and layers not placed above [`LayerIndex::ADORNER`].
///
/// The full context is captured for adorner widget so you can use context variables inside without issue.
///
/// [layered]: LAYERS
/// [`WidgetFn<()>`]: WidgetFn
#[property(FILL, default(WidgetFn::nil()))]
pub fn adorner_fn(child: impl IntoUiNode, adorner_fn: impl IntoVar<WidgetFn<()>>) -> UiNode {
    let adorner_fn = adorner_fn.into_var();
    let mut adorner_id = None;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&adorner_fn);
            let f = adorner_fn.get();
            if !f.is_nil() {
                let widget = with_context_blend(LocalContext::capture_filtered(CaptureFilter::All), false, f(()));
                let id = LAYERS.insert_anchored_node(LayerIndex::ADORNER, WIDGET.id(), AnchorMode::foreground(), widget);
                adorner_id = Some(id);
            }
        }
        UiNodeOp::Deinit => {
            if let Some(id) = adorner_id.take() {
                LAYERS.remove_node(id);
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(f) = adorner_fn.get_new() {
                if let Some(id) = adorner_id.take() {
                    LAYERS.remove_node(id);
                }

                if !f.is_nil() {
                    let widget = with_context_blend(LocalContext::capture_filtered(CaptureFilter::All), false, f(()));
                    let id = LAYERS.insert_anchored_node(LayerIndex::ADORNER, WIDGET.id(), AnchorMode::foreground(), widget);
                    adorner_id = Some(id);
                }
            }
        }
        _ => {}
    })
}

/// Custom layered foreground.
///
/// This is the equivalent of setting [`adorner_fn`] to a [`WidgetFn::singleton`].
///
/// [`adorner_fn`]: fn@adorner_fn
/// [`WidgetFn::singleton`]: zng_wgt::prelude::WidgetFn::singleton
#[property(FILL, default(UiNode::nil()))]
pub fn adorner(child: impl IntoUiNode, adorner: impl IntoUiNode) -> UiNode {
    adorner_fn(child, WidgetFn::singleton(adorner))
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
