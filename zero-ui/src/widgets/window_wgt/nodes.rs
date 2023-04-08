//! UI nodes used for building a window widget.

use crate::core::{
    mouse::MOUSE,
    units::DipToPx,
    window::{WIDGET_INFO_CHANGED_EVENT, WINDOW_CTRL},
};
use crate::prelude::new_property::*;

use std::{mem, ops};
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
/// size is discarded, only the root widget desired size can affect the window size. Layered widgets are all layout
/// and rendered after the window content and from the bottom layer up to the top-most, this means that the [`WidgetBoundsInfo`]
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
    pub fn insert(&self, layer: impl IntoVar<LayerIndex>, widget: impl UiNode) {
        struct LayeredWidget<L, W> {
            layer: L,
            widget: W,
        }
        #[ui_node(
                delegate = &self.widget,
                delegate_mut = &mut self.widget,
            )]
        impl<L: Var<LayerIndex>, W: UiNode> UiNode for LayeredWidget<L, W> {
            fn init(&mut self) {
                self.widget.with_context(|| {
                    WIDGET.set_state(&LAYER_INDEX_ID, self.layer.get());
                });
                self.widget.init();
            }

            fn update(&mut self, updates: &WidgetUpdates) {
                if let Some(index) = self.layer.get_new() {
                    self.widget.with_context(|| {
                        WIDGET.set_state(&LAYER_INDEX_ID, index);
                    });
                    SortingListParent::invalidate_sort();
                }
                self.widget.update(updates);
            }

            fn with_context<R, F>(&self, f: F) -> Option<R>
            where
                F: FnOnce() -> R,
            {
                self.widget.with_context(f)
            }
        }

        WINDOW.with_state(|s| {
            s.req(&WINDOW_LAYERS_ID).items.push(LayeredWidget {
                layer: layer.into_var(),
                widget: widget.cfg_boxed(),
            });
        });
    }

    /// Insert the `widget` in the layer and *anchor* it to the offset/transform of another widget.
    ///
    /// The `anchor` is the ID of another widget, the inserted `widget` will be offset/transform so that it aligns
    /// with the `anchor` widget top-left. The `mode` is a value of [`AnchorMode`] that defines if the `widget` will
    /// receive the full transform or just the offset.
    ///
    /// If the `anchor` widget is not found the `widget` is not rendered (visibility `Collapsed`).
    pub fn insert_anchored(
        &self,
        layer: impl IntoVar<LayerIndex>,
        anchor: impl IntoVar<WidgetId>,
        mode: impl IntoVar<AnchorMode>,

        widget: impl UiNode,
    ) {
        struct AnchoredWidget<A, M, W> {
            anchor: A,
            mode: M,
            widget: W,
            info_changed_handle: Option<EventHandle>,
            mouse_pos_handle: Option<VarHandle>,

            anchor_info: Option<(WidgetBoundsInfo, WidgetBorderInfo)>,
            offset: (PxPoint, PxPoint), // place, origin (place is relative)
            interactivity: bool,
            cursor_once_pending: bool,

            transform_key: FrameValueKey<PxTransform>,
            corner_radius_ctx_handle: Option<ContextInitHandle>,
        }
        #[ui_node(
                delegate = &self.widget,
                delegate_mut = &mut self.widget,
            )]
        impl<A, M, W> UiNode for AnchoredWidget<A, M, W>
        where
            A: Var<WidgetId>,
            M: Var<AnchorMode>,
            W: UiNode,
        {
            fn info(&self, info: &mut WidgetInfoBuilder) {
                if self.interactivity {
                    if let Some(widget) = self.widget.with_context(|| WIDGET.id()) {
                        let anchor = self.anchor.get();
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
                self.widget.info(info)
            }

            fn init(&mut self) {
                WIDGET.sub_var(&self.anchor).sub_var(&self.mode);

                let tree = WINDOW.widget_tree();
                if let Some(w) = tree.get(self.anchor.get()) {
                    self.anchor_info = Some((w.bounds_info(), w.border_info()));
                }

                self.interactivity = self.mode.with(|m| m.interactivity);
                self.info_changed_handle = Some(WIDGET_INFO_CHANGED_EVENT.subscribe(WIDGET.id()));

                if self.mode.with(|m| matches!(&m.transform, AnchorTransform::Cursor(_))) {
                    self.mouse_pos_handle = Some(MOUSE.position().subscribe(WIDGET.id()));
                } else if self.mode.with(|m| matches!(&m.transform, AnchorTransform::CursorOnce(_))) {
                    self.cursor_once_pending = true;
                }

                self.widget.init();
            }

            fn deinit(&mut self) {
                self.anchor_info = None;
                self.info_changed_handle = None;
                self.mouse_pos_handle = None;
                self.widget.deinit();
                self.corner_radius_ctx_handle = None;
                self.cursor_once_pending = false;
            }

            fn event(&mut self, update: &EventUpdate) {
                if let Some(args) = WIDGET_INFO_CHANGED_EVENT.on(update) {
                    if args.window_id == WINDOW.id() {
                        self.anchor_info = WINDOW
                            .widget_tree()
                            .get(self.anchor.get())
                            .map(|w| (w.bounds_info(), w.border_info()));
                    }
                }
                self.widget.event(update);
            }

            fn update(&mut self, updates: &WidgetUpdates) {
                if let Some(anchor) = self.anchor.get_new() {
                    self.anchor_info = WINDOW.widget_tree().get(anchor).map(|w| (w.bounds_info(), w.border_info()));
                    if self.mode.with(|m| m.interactivity) {
                        WIDGET.update_info();
                    }
                    WIDGET.layout().render();
                }
                if let Some(mode) = self.mode.get_new() {
                    if mode.interactivity != self.interactivity {
                        self.interactivity = mode.interactivity;
                        WIDGET.update_info();
                    }
                    if matches!(&mode.transform, AnchorTransform::Cursor(_)) {
                        if self.mouse_pos_handle.is_none() {
                            self.mouse_pos_handle = Some(MOUSE.position().subscribe(WIDGET.id()));
                        }
                        self.cursor_once_pending = false;
                    } else {
                        self.cursor_once_pending = matches!(&mode.transform, AnchorTransform::CursorOnce(_));
                        self.mouse_pos_handle = None;
                    }
                    WIDGET.layout().render();
                } else if self.mouse_pos_handle.is_some() && MOUSE.position().is_new() {
                    WIDGET.layout();
                }
                self.widget.update(updates);
            }

            fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
                if let Some((bounds, border)) = &self.anchor_info {
                    let mode = self.mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        return LAYOUT.with_constraints(
                            match mode.size {
                                AnchorSize::Unbounded => PxConstraints2d::new_unbounded(),
                                AnchorSize::Window => LAYOUT.constraints(),
                                AnchorSize::InnerSize => PxConstraints2d::new_exact_size(bounds.inner_size()),
                                AnchorSize::InnerBorder => PxConstraints2d::new_exact_size(border.inner_size(bounds)),
                                AnchorSize::OuterSize => PxConstraints2d::new_exact_size(bounds.outer_size()),
                            },
                            || self.widget.measure(wm),
                        );
                    }
                }

                PxSize::zero()
            }
            fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
                if let Some((bounds, border)) = &self.anchor_info {
                    let mode = self.mode.get();

                    if !mode.visibility || bounds.inner_size() != PxSize::zero() {
                        // if we don't link visibility or anchor is not collapsed.

                        let layer_size = LAYOUT.with_constraints(
                            match mode.size {
                                AnchorSize::Unbounded => PxConstraints2d::new_unbounded(),
                                AnchorSize::Window => LAYOUT.constraints(),
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
                                        self.corner_radius_ctx_handle.get_or_insert_with(ContextInitHandle::new).clone(),
                                        cr,
                                        || BORDER.with_corner_radius(|| self.widget.layout(wl)),
                                    )
                                } else {
                                    self.widget.layout(wl)
                                }
                            },
                        );

                        if let Some((p, update)) = match &mode.transform {
                            AnchorTransform::Cursor(p) => Some((p, true)),
                            AnchorTransform::CursorOnce(p) => Some((p, mem::take(&mut self.cursor_once_pending))),
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
                                    let cursor_size = PxSize::splat(Dip::new(22).to_px(fct));
                                    let place = pos.to_px(fct)
                                        + LAYOUT
                                            .with_constraints(PxConstraints2d::new_exact_size(cursor_size), || p.place.layout())
                                            .to_vector();
                                    let origin = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(layer_size), || p.origin.layout());

                                    let offset = (place, origin);
                                    if self.offset != offset {
                                        self.offset = offset;
                                        WIDGET.render_update();
                                    }

                                    return layer_size;
                                } else {
                                    // collapsed signal (permanent if `CursorOnce`)
                                    self.offset.0.x = NO_POS_X;
                                }
                            } else {
                                // offset already set
                                if self.offset.0.x != NO_POS_X {
                                    // and it is not collapsed `CursorOnce`
                                    return layer_size;
                                }
                            }
                        } else {
                            // other transform modes, will be visible
                            let offset = match &mode.transform {
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
                            if self.offset != offset {
                                self.offset = offset;
                                WIDGET.render_update();
                            }

                            return layer_size;
                        }
                    }
                }

                // no visible mode match
                wl.collapse();
                PxSize::zero()
            }

            fn render(&self, frame: &mut FrameBuilder) {
                if let Some((bounds_info, border_info)) = &self.anchor_info {
                    let mode = self.mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        let mut push_reference_frame = |mut transform: PxTransform, is_translate_only: bool| {
                            if mode.viewport_bound {
                                transform = adjust_viewport_bound(transform, &self.widget);
                            }
                            frame.push_reference_frame(
                                self.transform_key.into(),
                                self.transform_key.bind(transform, true),
                                is_translate_only,
                                false,
                                |frame| self.widget.render(frame),
                            );
                        };

                        match mode.transform {
                            AnchorTransform::InnerOffset(_) => {
                                let place_in_window = bounds_info.inner_transform().transform_point(self.offset.0).unwrap_or_default();
                                let offset = place_in_window - self.offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::InnerBorderOffset(_) => {
                                let place_in_window = border_info
                                    .inner_transform(bounds_info)
                                    .transform_point(self.offset.0)
                                    .unwrap_or_default();
                                let offset = place_in_window - self.offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::OuterOffset(_) => {
                                let place_in_window = bounds_info.outer_transform().transform_point(self.offset.0).unwrap_or_default();
                                let offset = place_in_window - self.offset.1;

                                push_reference_frame(PxTransform::from(offset), true);
                            }
                            AnchorTransform::Cursor(_) | AnchorTransform::CursorOnce(_) => {
                                let offset = self.offset.0 - self.offset.1;

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
                            _ => self.widget.render(frame),
                        }
                    }
                }
            }

            fn render_update(&self, update: &mut FrameUpdate) {
                if let Some((bounds_info, border_info)) = &self.anchor_info {
                    let mode = self.mode.get();
                    if !mode.visibility || bounds_info.rendered().is_some() {
                        let mut with_transform = |mut transform: PxTransform| {
                            if mode.viewport_bound {
                                transform = adjust_viewport_bound(transform, &self.widget);
                            }
                            update.with_transform(self.transform_key.update(transform, true), false, |update| {
                                self.widget.render_update(update)
                            });
                        };

                        match mode.transform {
                            AnchorTransform::InnerOffset(_) => {
                                let place_in_window = bounds_info.inner_transform().transform_point(self.offset.0).unwrap_or_default();
                                let offset = place_in_window - self.offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::InnerBorderOffset(_) => {
                                let place_in_window = border_info
                                    .inner_transform(bounds_info)
                                    .transform_point(self.offset.0)
                                    .unwrap_or_default();
                                let offset = place_in_window - self.offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::OuterOffset(_) => {
                                let place_in_window = bounds_info.outer_transform().transform_point(self.offset.0).unwrap_or_default();
                                let offset = place_in_window - self.offset.1;
                                with_transform(PxTransform::from(offset));
                            }
                            AnchorTransform::Cursor(_) | AnchorTransform::CursorOnce(_) => {
                                let offset = self.offset.0 - self.offset.1;
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
                            _ => self.widget.render_update(update),
                        }
                    }
                }
            }

            fn with_context<R, F>(&self, f: F) -> Option<R>
            where
                F: FnOnce() -> R,
            {
                self.widget.with_context(f)
            }
        }

        self.insert(
            layer,
            AnchoredWidget {
                anchor: anchor.into_var(),
                mode: mode.into_var(),
                widget: widget.cfg_boxed(),
                info_changed_handle: None,
                mouse_pos_handle: None,
                cursor_once_pending: false,

                anchor_info: None,
                offset: (PxPoint::zero(), PxPoint::zero()),
                interactivity: false,

                transform_key: FrameValueKey::new_unique(),
                corner_radius_ctx_handle: None,
            },
        );
    }

    /// Remove the widget from the layers overlay in the next update.
    ///
    /// The `id` must the widget id of a previous inserted widget, nothing happens if the widget is not found.
    pub fn remove(&self, id: impl Into<WidgetId>) {
        WINDOW.with_state(|s| {
            s.req(&WINDOW_LAYERS_ID).items.remove(id);
        });
    }
}

fn adjust_viewport_bound(transform: PxTransform, widget: &impl UiNode) -> PxTransform {
    let window_bounds = WINDOW_CTRL.vars().actual_size_px().get();
    let wgt_bounds = PxBox::from(widget.with_context(|| WIDGET.bounds().outer_size()).unwrap_or_else(PxSize::zero));
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

static WINDOW_LAYERS_ID: StaticStateId<LayersCtx> = StaticStateId::new_unique();
static LAYER_INDEX_ID: StaticStateId<LayerIndex> = StaticStateId::new_unique();

/// Wrap around the window outer-most event node to create the layers.
///
/// This node is included in the [`NestGroup::EVENT`] group.
///
/// [`NestGroup::EVENT`]: crate::core::widget_builder::NestGroup::EVENT
pub fn layers(child: impl UiNode) -> impl UiNode {
    #[ui_node(struct LayersNode {
        children: impl UiNodeList,
        layered: EditableUiNodeListRef,
    })]
    impl UiNode for LayersNode {
        fn init(&mut self) {
            WINDOW.set_state(
                &WINDOW_LAYERS_ID,
                LayersCtx {
                    items: self.layered.clone(),
                },
            );

            self.children.init_all();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            let mut changed = false;

            self.children.update_all(updates, &mut changed);

            if changed {
                WIDGET.layout().render();
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.children.with_node(0, |n| n.measure(wm))
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let mut size = PxSize::zero();
            self.children.for_each_mut(|i, n| {
                let s = n.layout(wl);
                if i == 0 {
                    size = s;
                }
                true
            });
            size
        }
    }

    let layers = EditableUiNodeList::new();
    let layered = layers.reference();

    let sorting_layers = SortingList::new(layers, |a, b| {
        let a = a.with_context(|| WIDGET.req_state(&LAYER_INDEX_ID)).unwrap_or(LayerIndex::DEFAULT);
        let b = b.with_context(|| WIDGET.req_state(&LAYER_INDEX_ID)).unwrap_or(LayerIndex::DEFAULT);

        a.cmp(&b)
    });

    LayersNode {
        children: ui_vec![child].chain(sorting_layers),
        layered,
    }
    .cfg_boxed()
}

/// Represents a layer in a window.
///
/// See [`LAYERS`] for more information.
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
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

/// Represents two points that position a layer widget with its anchor widget.
///
/// The `place` point is layout in the anchor widget bounds, the `origin` point is layout in the layer widget bounds,
/// the layer widget is offset so that the `origin` point aligns with the `place` point.
#[derive(Debug, Clone, PartialEq)]
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

    fn chase(&mut self, increment: Self) {
        self.place.chase(increment.place);
        self.origin.chase(increment.origin);
    }
}

/// Options for [`AnchorMode::transform`].
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
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

/// Defines if a widget load affects the parent window load.
///
/// Widgets that support this behavior have a `block_window_load` property.
#[derive(Clone, Copy, Debug)]
pub enum BlockWindowLoad {
    /// Widget requests a [`WindowLoadingHandle`] and retains it until the widget is loaded.
    ///
    /// [`WindowLoadingHandle`]: crate::core::window::WindowLoadingHandle
    Enabled {
        /// Handle expiration deadline, if the widget takes longer than this deadline the window loads anyway.
        deadline: Deadline,
    },
    /// Widget does not hold back window load.
    Disabled,
}
impl BlockWindowLoad {
    /// Enabled value.
    pub fn enabled(deadline: impl Into<Deadline>) -> BlockWindowLoad {
        BlockWindowLoad::Enabled { deadline: deadline.into() }
    }

    /// Returns `true` if is enabled.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// Returns `true` if is disabled.
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Returns the block deadline if is enabled and the deadline has not expired.
    pub fn deadline(self) -> Option<Deadline> {
        match self {
            BlockWindowLoad::Enabled { deadline } => {
                if deadline.has_elapsed() {
                    None
                } else {
                    Some(deadline)
                }
            }
            BlockWindowLoad::Disabled => None,
        }
    }
}
impl_from_and_into_var! {
    /// Converts `true` to `BlockWindowLoad::enabled(1.secs())` and `false` to `BlockWindowLoad::Disabled`.
    fn from(enabled: bool) -> BlockWindowLoad {
        if enabled {
            BlockWindowLoad::enabled(1.secs())
        } else {
            BlockWindowLoad::Disabled
        }
    }

    /// Converts to enabled with the duration timeout.
    fn from(enabled_timeout: Duration) -> BlockWindowLoad {
        BlockWindowLoad::enabled(enabled_timeout)
    }
}

/// Node that binds the [`COLOR_SCHEME_VAR`] to the [`actual_color_scheme`].
///
/// [`actual_color_scheme`]: crate::core::window::WindowVars::actual_color_scheme
pub fn color_scheme(child: impl UiNode) -> impl UiNode {
    with_context_var_init(child, COLOR_SCHEME_VAR, || WINDOW_CTRL.vars().actual_color_scheme().boxed())
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
