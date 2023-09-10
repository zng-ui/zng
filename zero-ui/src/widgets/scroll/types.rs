use std::{fmt, mem, sync::Arc, time::Duration};

use crate::core::{
    context::{context_local, with_context_local_init, StaticStateId, WIDGET},
    task::parking_lot::Mutex,
    units::*,
    var::{
        animation::{ChaseAnimation, *},
        *,
    },
    widget_info::WidgetInfo,
    widget_instance::{match_node, UiNode, UiNodeOp, WidgetId},
};
use atomic::{Atomic, Ordering};
use bitflags::bitflags;

use super::{commands, SMOOTH_SCROLLING_VAR};

bitflags! {
    /// What dimensions are scrollable in a widget.
    ///
    /// If a dimension is scrollable the content can be any size in that dimension, if the size
    /// is more then available scrolling is enabled for that dimension.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct ScrollMode: u8 {
        /// Content size is constrained by the viewport and is not scrollable.
        const NONE = 0;
        /// Content can be any height and scrolls vertically if overflow height.
        const VERTICAL = 0b01;
        /// Content can be any width and scrolls horizontally if overflow width.
        const HORIZONTAL = 0b10;
        /// Content can be any size and scrolls if overflow.
        const PAN = 0b11;
        /// Content can be any size and scrolls if overflow (`PAN`) and also can be scaled
        /// up and down by zoom commands and gestures.
        const ZOOM = 0b111;
    }
}
impl_from_and_into_var! {
    /// Returns [`ZOOM`] for `true` and [`NONE`] for `false`.
    ///
    /// [`ZOOM`]: ScrollMode::ZOOM
    /// [`NONE`]: ScrollMode::NONE
    fn from(zoom: bool) -> ScrollMode {
        if zoom {
            ScrollMode::ZOOM
        } else {
            ScrollMode::NONE
        }
    }
}

context_var! {
    /// Vertical offset of the parent scroll.
    ///
    /// The value is a percentage of `content.height - viewport.height`.
    pub(super) static SCROLL_VERTICAL_OFFSET_VAR: Factor = 0.fct();
    /// Horizontal offset of the parent scroll.
    ///
    /// The value is a percentage of `content.width - viewport.width`.
    pub(super) static SCROLL_HORIZONTAL_OFFSET_VAR: Factor = 0.fct();

    /// Extra vertical offset requested that could not be fulfilled because [`SCROLL_VERTICAL_OFFSET_VAR`]
    /// is already at `0.fct()` or `1.fct()`.
    pub(super) static OVERSCROLL_VERTICAL_OFFSET_VAR: Factor = 0.fct();

    /// Extra horizontal offset requested that could not be fulfilled because [`SCROLL_HORIZONTAL_OFFSET_VAR`]
    /// is already at `0.fct()` or `1.fct()`.
    pub(super) static OVERSCROLL_HORIZONTAL_OFFSET_VAR: Factor = 0.fct();

    /// Ratio of the scroll parent viewport height to its content.
    ///
    /// The value is `viewport.height / content.height`.
    pub(super) static SCROLL_VERTICAL_RATIO_VAR: Factor = 0.fct();

    /// Ratio of the scroll parent viewport width to its content.
    ///
    /// The value is `viewport.width / content.width`.
    pub(super) static SCROLL_HORIZONTAL_RATIO_VAR: Factor = 0.fct();

    /// If the vertical scrollbar should be visible.
    pub(super) static SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR: bool = false;

    /// If the horizontal scrollbar should be visible.
    pub(super) static SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR: bool = false;

    /// Latest computed viewport size of the parent scroll.
    pub(super) static SCROLL_VIEWPORT_SIZE_VAR: PxSize = PxSize::zero();

    /// Latest computed content size of the parent scroll.
    ///
    /// The size is scaled if zoom is set.
    pub(super) static SCROLL_CONTENT_SIZE_VAR: PxSize = PxSize::zero();

    /// Zoom scaling of the parent scroll.
    pub(super) static SCROLL_SCALE_VAR: Factor = 1.fct();

    /// Scroll mode.
    pub(super) static SCROLL_MODE_VAR: ScrollMode = ScrollMode::empty();

}

context_local! {
    static SCROLL_CONFIG: ScrollConfig = ScrollConfig::default();
}

#[derive(Debug, Clone, Copy, bytemuck::NoUninit)]
#[repr(C)]
struct RenderedOffsets {
    h: Factor,
    v: Factor,
    z: Factor,
}

#[derive(Debug)]
struct ScrollConfig {
    id: Option<WidgetId>,
    horizontal: Mutex<Option<ChaseAnimation<Factor>>>,
    vertical: Mutex<Option<ChaseAnimation<Factor>>>,
    zoom: Mutex<Option<ChaseAnimation<Factor>>>,

    // last rendered horizontal, vertical offsets.
    rendered: Atomic<RenderedOffsets>,

    overscroll_horizontal: Mutex<AnimationHandle>,
    overscroll_vertical: Mutex<AnimationHandle>,
}
impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            id: Default::default(),
            horizontal: Default::default(),
            vertical: Default::default(),
            zoom: Default::default(),
            rendered: Atomic::new(RenderedOffsets {
                h: 0.fct(),
                v: 0.fct(),
                z: 0.fct(),
            }),
            overscroll_horizontal: Default::default(),
            overscroll_vertical: Default::default(),
        }
    }
}

/// Defines a scroll delta and to what value source it is applied.
///
/// Scrolling can get out of sync depending on what moment and source the current scroll is read,
/// the offset vars can be multiple frames ahead as update cycles have higher priority than render,
/// some scrolling operations also target the value the smooth scrolling animation is animating too,
/// this enum lets you specify from what scroll offset a delta must be computed.
#[derive(Debug, Clone, Copy, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ScrollFrom {
    /// Scroll amount added to the offset var current value, if smooth scrolling is enabled this
    /// can be a partial value different from `VarTarget`.
    ///
    /// Operations that compute a scroll delta from the offset var must use this variant otherwise they
    /// will overshoot.
    Var(Px),
    /// Scroll amount added to the value the offset var is animating too.
    ///
    /// Operations that accumulate a delta (line-up/down) must use this variant otherwise they will
    /// undershoot.
    ///
    /// This is the same as `Var` if smooth scrolling is disabled.
    VarTarget(Px),

    /// Scroll amount added to the offset already rendered, this can be different from the offset var as multiple
    /// events and updates can happen before a pending render is applied.
    ///
    /// Operations that compute a scroll offset from widget bounds info must use this variant otherwise they
    /// will overshoot.
    Rendered(Px),
}

/// Controls the parent scroll.
pub struct SCROLL;
impl SCROLL {
    /// Gets the ID of the scroll ancestor represented by the [`SCROLL`].
    pub fn try_id(&self) -> Option<WidgetId> {
        SCROLL_CONFIG.get().id
    }
    /// Gets the ID of the scroll ancestor represented by the [`SCROLL`].
    ///
    /// # Panics
    ///
    /// Panics if not inside a scroll.
    pub fn id(&self) -> WidgetId {
        self.try_id().expect("not inside scroll")
    }

    /// New node that holds data for the [`SCROLL`] context.
    ///
    /// Scroll implementers must add this node to their context.
    pub fn config_node(&self, child: impl UiNode) -> impl UiNode {
        let child = match_node(child, move |_, op| {
            if let UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } = op {
                let h = SCROLL_HORIZONTAL_OFFSET_VAR.get();
                let v = SCROLL_VERTICAL_OFFSET_VAR.get();
                let z = SCROLL_SCALE_VAR.get();
                SCROLL_CONFIG.get().rendered.store(RenderedOffsets { h, v, z }, Ordering::Relaxed);
            }
        });
        with_context_local_init(child, &SCROLL_CONFIG, || ScrollConfig {
            id: WIDGET.try_id(),
            ..Default::default()
        })
    }

    /// Scroll mode of the parent scroll.
    pub fn mode(&self) -> ReadOnlyContextVar<ScrollMode> {
        SCROLL_MODE_VAR.read_only()
    }

    /// Vertical offset of the parent scroll.
    ///
    /// The value is a percentage of `content.height - viewport.height`. This variable is usually read-write,
    /// scrollable content can modify it to scroll the parent.
    pub fn vertical_offset(&self) -> ReadOnlyContextVar<Factor> {
        SCROLL_VERTICAL_OFFSET_VAR.read_only()
    }

    /// Horizontal offset of the parent scroll.
    ///
    /// The value is a percentage of `content.width - viewport.width`. This variable is usually read-write,
    /// scrollable content can modify it to scroll the parent.
    pub fn horizontal_offset(&self) -> ReadOnlyContextVar<Factor> {
        SCROLL_HORIZONTAL_OFFSET_VAR.read_only()
    }

    /// Zoom scale factor.
    pub fn zoom_scale(&self) -> ReadOnlyContextVar<Factor> {
        SCROLL_SCALE_VAR.read_only()
    }

    /// Latest rendered offset.
    pub fn rendered_offset(&self) -> Factor2d {
        let cfg = SCROLL_CONFIG.get().rendered.load(Ordering::Relaxed);
        Factor2d::new(cfg.h, cfg.v)
    }

    /// Latest rendered zoom scale factor.
    pub fn rendered_zoom_scale(&self) -> Factor {
        SCROLL_CONFIG.get().rendered.load(Ordering::Relaxed).z
    }

    /// Latest rendered offset in pixels.
    pub fn rendered_offset_px(&self) -> PxVector {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
        let content = SCROLL_CONTENT_SIZE_VAR.get();
        let max_scroll = content - viewport;
        max_scroll.to_vector() * self.rendered_offset()
    }

    /// Extra vertical offset, requested by touch gesture, that could not be fulfilled because [`vertical_offset`]
    /// is already at `0.fct()` or `1.fct()`.
    ///
    /// [`vertical_offset`]: Self::vertical_offset
    pub fn vertical_overscroll(&self) -> ReadOnlyContextVar<Factor> {
        OVERSCROLL_VERTICAL_OFFSET_VAR.read_only()
    }

    /// Extra horizontal offset requested that could not be fulfilled because [`horizontal_offset`]
    /// is already at `0.fct()` or `1.fct()`.
    ///
    /// [`horizontal_offset`]: Self::horizontal_offset
    pub fn horizontal_overscroll(&self) -> ReadOnlyContextVar<Factor> {
        OVERSCROLL_HORIZONTAL_OFFSET_VAR.read_only()
    }

    /// Ratio of the scroll parent viewport height to its content.
    ///
    /// The value is `viewport.height / content.height`.
    pub fn vertical_ratio(&self) -> ReadOnlyContextVar<Factor> {
        SCROLL_VERTICAL_RATIO_VAR.read_only()
    }
    /// Ratio of the scroll parent viewport width to its content.
    ///
    /// The value is `viewport.width / content.width`.
    pub fn horizontal_ratio(&self) -> ReadOnlyContextVar<Factor> {
        SCROLL_HORIZONTAL_RATIO_VAR.read_only()
    }

    /// If the vertical scrollbar should be visible.
    pub fn vertical_content_overflows(&self) -> ReadOnlyContextVar<bool> {
        SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.read_only()
    }

    /// If the horizontal scrollbar should be visible.
    pub fn horizontal_content_overflows(&self) -> ReadOnlyContextVar<bool> {
        SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.read_only()
    }

    /// Latest computed viewport size of the parent scroll.
    pub fn viewport_size(&self) -> ReadOnlyContextVar<PxSize> {
        SCROLL_VIEWPORT_SIZE_VAR.read_only()
    }

    /// Latest computed content size of the parent scroll.
    pub fn content_size(&self) -> ReadOnlyContextVar<PxSize> {
        SCROLL_CONTENT_SIZE_VAR.read_only()
    }

    /// Applies the `delta` to the vertical offset.
    ///
    /// If smooth scrolling is enabled it is used to update the offset.
    pub fn scroll_vertical(&self, delta: ScrollFrom) {
        self.scroll_vertical_clamp(delta, f32::MIN, f32::MAX);
    }

    /// Applies the `delta` to the vertical offset, but clamps the final offset by the inclusive `min` and `max`.
    ///
    /// If smooth scrolling is enabled it is used to update the offset.
    pub fn scroll_vertical_clamp(&self, delta: ScrollFrom, min: f32, max: f32) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        let max_scroll = content - viewport;

        if max_scroll <= Px(0) {
            return;
        }

        match delta {
            ScrollFrom::Var(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                let f = SCROLL_VERTICAL_OFFSET_VAR.get();
                SCROLL.chase_vertical(|_| (f.0 + amount).clamp(min, max).fct());
            }
            ScrollFrom::VarTarget(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                SCROLL.chase_vertical(|f| (f.0 + amount).clamp(min, max).fct());
            }
            ScrollFrom::Rendered(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                let f = SCROLL_CONFIG.get().rendered.load(Ordering::Relaxed).v;
                SCROLL.chase_vertical(|_| (f.0 + amount).clamp(min, max).fct());
            }
        }
    }

    /// Applies the `delta` to the vertical offset without smooth scrolling and
    /// updates the vertical overscroll if it changes.
    ///
    /// This method is used to implement touch gesture scrolling, the delta is always [`ScrollFrom::Var`].
    pub fn scroll_vertical_touch(&self, delta: Px) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        let max_scroll = content - viewport;
        if max_scroll <= Px(0) {
            return;
        }

        let delta = delta.0 as f32 / max_scroll.0 as f32;

        let current = SCROLL_VERTICAL_OFFSET_VAR.get();
        let mut next = current + delta.fct();
        let mut overscroll = 0.fct();
        if next > 1.fct() {
            overscroll = next - 1.fct();
            next = 1.fct();
        } else if next < 0.fct() {
            overscroll = next;
            next = 0.fct();
        }

        let _ = SCROLL_VERTICAL_OFFSET_VAR.set(next);
        if overscroll != 0.fct() {
            let new_handle = Self::increment_overscroll(OVERSCROLL_VERTICAL_OFFSET_VAR, overscroll);

            let config = SCROLL_CONFIG.get();
            let mut handle = config.overscroll_vertical.lock();
            mem::replace(&mut *handle, new_handle).stop();
        } else {
            self.clear_vertical_overscroll();
        }
    }

    fn increment_overscroll(overscroll: ContextVar<Factor>, delta: Factor) -> AnimationHandle {
        enum State {
            Increment,
            ClearDelay,
            Clear(Transition<Factor>),
        }
        let mut state = State::Increment;
        overscroll.animate(move |a, o| match &mut state {
            State::Increment => {
                // set the increment and start delay to animation.
                *o.to_mut() += delta;

                a.sleep(300.ms());
                state = State::ClearDelay;
            }
            State::ClearDelay => {
                a.restart();
                let t = Transition::new(**o, 0.fct());
                state = State::Clear(t);
            }
            State::Clear(t) => {
                let step = easing::linear(a.elapsed_stop(300.ms()));
                o.set(t.sample(step));
            }
        })
    }

    /// Quick ease vertical overscroll to zero.
    pub fn clear_vertical_overscroll(&self) {
        if OVERSCROLL_VERTICAL_OFFSET_VAR.get() != 0.fct() {
            let new_handle = OVERSCROLL_VERTICAL_OFFSET_VAR.ease(0.fct(), 100.ms(), easing::linear);

            let config = SCROLL_CONFIG.get();
            let mut handle = config.overscroll_vertical.lock();
            mem::replace(&mut *handle, new_handle).stop();
        }
    }

    /// Quick ease horizontal overscroll to zero.
    pub fn clear_horizontal_overscroll(&self) {
        if OVERSCROLL_HORIZONTAL_OFFSET_VAR.get() != 0.fct() {
            let new_handle = OVERSCROLL_HORIZONTAL_OFFSET_VAR.ease(0.fct(), 100.ms(), easing::linear);

            let config = SCROLL_CONFIG.get();
            let mut handle = config.overscroll_horizontal.lock();
            mem::replace(&mut *handle, new_handle).stop();
        }
    }

    /// Applies the `delta` to the horizontal offset.
    ///
    /// If smooth scrolling is enabled the chase animation is created or updated by this call.
    pub fn scroll_horizontal(&self, delta: ScrollFrom) {
        self.scroll_horizontal_clamp(delta, f32::MIN, f32::MAX)
    }

    /// Applies the `delta` to the horizontal offset, but clamps the final offset by the inclusive `min` and `max`.
    ///
    /// If smooth scrolling is enabled it is used to update the offset.
    pub fn scroll_horizontal_clamp(&self, delta: ScrollFrom, min: f32, max: f32) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        let max_scroll = content - viewport;

        if max_scroll <= Px(0) {
            return;
        }

        match delta {
            ScrollFrom::Var(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                let f = SCROLL_HORIZONTAL_OFFSET_VAR.get();
                SCROLL.chase_horizontal(|_| (f.0 + amount).clamp(min, max).fct());
            }
            ScrollFrom::VarTarget(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                SCROLL.chase_horizontal(|f| (f.0 + amount).clamp(min, max).fct());
            }
            ScrollFrom::Rendered(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                let f = SCROLL_CONFIG.get().rendered.load(Ordering::Relaxed).h;
                SCROLL.chase_horizontal(|_| (f.0 + amount).clamp(min, max).fct());
            }
        }
    }

    /// Applies the `delta` to the horizontal offset without smooth scrolling and
    /// updates the horizontal overscroll if it changes.
    ///
    /// This method is used to implement touch gesture scrolling, the delta is always [`ScrollFrom::Var`].
    pub fn scroll_horizontal_touch(&self, amount: Px) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        let max_scroll = content - viewport;
        if max_scroll <= Px(0) {
            return;
        }

        let amount = amount.0 as f32 / max_scroll.0 as f32;

        let current = SCROLL_HORIZONTAL_OFFSET_VAR.get();
        let mut next = current + amount.fct();
        let mut overscroll = 0.fct();
        if next > 1.fct() {
            overscroll = next - 1.fct();
            next = 1.fct();
        } else if next < 0.fct() {
            overscroll = next;
            next = 0.fct();
        }

        let _ = SCROLL_HORIZONTAL_OFFSET_VAR.set(next);
        if overscroll != 0.fct() {
            let new_handle = Self::increment_overscroll(OVERSCROLL_HORIZONTAL_OFFSET_VAR, overscroll);

            let config = SCROLL_CONFIG.get();
            let mut handle = config.overscroll_horizontal.lock();
            mem::replace(&mut *handle, new_handle).stop();
        } else {
            self.clear_horizontal_overscroll();
        }
    }

    /// Set the vertical offset to a new offset derived from the last, blending into the active smooth
    /// scrolling chase animation, or starting a new one, or just setting the var if smooth scrolling is disabled.
    pub fn chase_vertical(&self, modify_offset: impl FnOnce(Factor) -> Factor) {
        #[cfg(dyn_closure)]
        let modify_offset: Box<dyn FnOnce(Factor) -> Factor> = Box::new(modify_offset);
        self.chase_vertical_impl(modify_offset);
    }
    fn chase_vertical_impl(&self, modify_offset: impl FnOnce(Factor) -> Factor) {
        let smooth = SMOOTH_SCROLLING_VAR.get();
        let config = SCROLL_CONFIG.get();
        let mut vertical = config.vertical.lock();
        match &mut *vertical {
            Some(t) => {
                if smooth.is_disabled() {
                    let t = modify_offset(*t.target()).clamp_range();
                    let _ = SCROLL_VERTICAL_OFFSET_VAR.set(t);
                    *vertical = None;
                } else {
                    let easing = smooth.easing.clone();
                    t.modify(|f| *f = modify_offset(*f).clamp_range(), smooth.duration, move |t| easing(t));
                }
            }
            None => {
                let t = modify_offset(SCROLL_VERTICAL_OFFSET_VAR.get()).clamp_range();
                if smooth.is_disabled() {
                    let _ = SCROLL_VERTICAL_OFFSET_VAR.set(t);
                } else {
                    let easing = smooth.easing.clone();
                    let anim = SCROLL_VERTICAL_OFFSET_VAR.chase(t, smooth.duration, move |t| easing(t));
                    *vertical = Some(anim);
                }
            }
        }
    }

    /// Set the horizontal offset to a new offset derived from the last set offset, blending into the active smooth
    /// scrolling chase animation, or starting a new one, or just setting the var if smooth scrolling is disabled.
    pub fn chase_horizontal(&self, modify_offset: impl FnOnce(Factor) -> Factor) {
        #[cfg(dyn_closure)]
        let modify_offset: Box<dyn FnOnce(Factor) -> Factor> = Box::new(modify_offset);
        self.chase_horizontal_impl(modify_offset);
    }
    fn chase_horizontal_impl(&self, modify_offset: impl FnOnce(Factor) -> Factor) {
        let smooth = SMOOTH_SCROLLING_VAR.get();
        let config = SCROLL_CONFIG.get();
        let mut horizontal = config.horizontal.lock();
        match &mut *horizontal {
            Some(t) => {
                if smooth.is_disabled() {
                    let t = modify_offset(*t.target()).clamp_range();
                    let _ = SCROLL_HORIZONTAL_OFFSET_VAR.set(t);
                    *horizontal = None;
                } else {
                    let easing = smooth.easing.clone();
                    t.modify(|f| *f = modify_offset(*f).clamp_range(), smooth.duration, move |t| easing(t));
                }
            }
            None => {
                let t = modify_offset(SCROLL_HORIZONTAL_OFFSET_VAR.get()).clamp_range();
                if smooth.is_disabled() {
                    let _ = SCROLL_HORIZONTAL_OFFSET_VAR.set(t);
                } else {
                    let easing = smooth.easing.clone();
                    let anim = SCROLL_HORIZONTAL_OFFSET_VAR.chase(t, smooth.duration, move |t| easing(t));
                    *horizontal = Some(anim);
                }
            }
        }
    }

    /// Set the zoom scale to a new scale derived from the last set scale, blending into the active
    /// smooth scaling chase animation, or starting a new or, or just setting the var if smooth scrolling is disabled.
    pub fn chase_zoom(&self, modify_scale: impl FnOnce(Factor) -> Factor) {
        #[cfg(dyn_closure)]
        let modify_scale: Box<dyn FnOnce(Factor) -> Factor> = Box::new(modify_scale);
        self.chase_zoom_impl(modify_scale);
    }
    fn chase_zoom_impl(&self, modify_scale: impl FnOnce(Factor) -> Factor) {
        if !SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) {
            return;
        }

        let smooth = SMOOTH_SCROLLING_VAR.get();
        let config = SCROLL_CONFIG.get();
        let mut zoom = config.zoom.lock();

        let min = super::MIN_ZOOM_VAR.get();
        let max = super::MAX_ZOOM_VAR.get();

        match &mut *zoom {
            Some(t) => {
                if smooth.is_disabled() {
                    let next = modify_scale(*t.target()).max(min).min(max);
                    let _ = SCROLL_SCALE_VAR.set(next);
                    *zoom = None;
                } else {
                    let easing = smooth.easing.clone();
                    t.modify(|f| *f = modify_scale(*f).max(min).min(max), smooth.duration, move |t| easing(t));
                }
            }
            None => {
                let t = modify_scale(SCROLL_SCALE_VAR.get()).max(min).min(max);
                if smooth.is_disabled() {
                    let _ = SCROLL_SCALE_VAR.set(t);
                } else {
                    let easing = smooth.easing.clone();
                    let anim = SCROLL_SCALE_VAR.chase(t, smooth.duration, move |t| easing(t));
                    *zoom = Some(anim);
                }
            }
        }
    }

    /// Returns `true` if the content height is greater then the viewport height.
    pub fn can_scroll_vertical(&self) -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        content > viewport
    }

    /// Returns `true` if the content width is greater then the viewport with.
    pub fn can_scroll_horizontal(&self) -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        content > viewport
    }

    /// Returns `true` if the content height is greater then the viewport height and the vertical offset
    /// is not at the maximum.
    pub fn can_scroll_down(&self) -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        content > viewport && 1.fct() > SCROLL_VERTICAL_OFFSET_VAR.get()
    }

    /// Returns `true` if the content height is greater then the viewport height and the vertical offset
    /// is not at the minimum.
    pub fn can_scroll_up(&self) -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        content > viewport && 0.fct() < SCROLL_VERTICAL_OFFSET_VAR.get()
    }

    /// Returns `true` if the content width is greater then the viewport width and the horizontal offset
    /// is not at the minimum.
    pub fn can_scroll_left(&self) -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        content > viewport && 0.fct() < SCROLL_HORIZONTAL_OFFSET_VAR.get()
    }

    /// Returns `true` if the content width is greater then the viewport width and the horizontal offset
    /// is not at the maximum.
    pub fn can_scroll_right(&self) -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        content > viewport && 1.fct() > SCROLL_HORIZONTAL_OFFSET_VAR.get()
    }

    /// Scroll the [`WIDGET`] into view.
    ///
    /// This requests [`commands::scroll_to_info`] for the contextual widget.
    pub fn scroll_to(&self, mode: impl Into<super::commands::ScrollToMode>) {
        commands::scroll_to_info(&WIDGET.info(), mode.into())
    }

    /// Scroll the [`WIDGET`] into view and adjusts the zoom scale.
    ///
    /// This rquests [`commands::scroll_to_info_zoom`] for the contextual widget.
    pub fn scroll_to_zoom(&self, mode: impl Into<super::commands::ScrollToMode>, zoom: impl Into<Factor>) {
        commands::scroll_to_info_zoom(&WIDGET.info(), mode.into(), zoom.into())
    }

    /// Returns `true` if the content can be scaled and the current scale is less than the max.
    pub fn can_zoom_in(&self) -> bool {
        SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) && SCROLL_SCALE_VAR.get() < super::MAX_ZOOM_VAR.get()
    }

    /// Returns `true` if the content can be scaled and the current scale is more than the min.
    pub fn can_zoom_out(&self) -> bool {
        SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) && SCROLL_SCALE_VAR.get() > super::MIN_ZOOM_VAR.get()
    }
}

/// Scroll extensions for [`WidgetInfo`].
pub trait WidgetInfoExt {
    /// Returns `true` if the widget is a [`Scroll!`](struct@super::Scroll).
    fn is_scroll(&self) -> bool;

    /// Returns a reference to the viewport bounds if the widget is a [`Scroll!`](struct@super::Scroll).
    fn scroll_info(&self) -> Option<ScrollInfo>;

    /// Gets the viewport bounds relative to the scroll widget inner bounds.
    ///
    /// The value is updated every layout and render, without requiring an info rebuild.
    fn viewport(&self) -> Option<PxRect>;
}
impl WidgetInfoExt for WidgetInfo {
    fn is_scroll(&self) -> bool {
        self.meta().get(&SCROLL_INFO_ID).is_some()
    }

    fn scroll_info(&self) -> Option<ScrollInfo> {
        self.meta().get(&SCROLL_INFO_ID).cloned()
    }

    fn viewport(&self) -> Option<PxRect> {
        self.meta().get(&SCROLL_INFO_ID).map(|r| r.viewport())
    }
}

#[derive(Debug, Default)]
struct ScrollData {
    viewport_transform: PxTransform,
    viewport_size: PxSize,
}

/// Shared reference to the viewport bounds of a scroll.
#[derive(Clone, Default, Debug)]
pub struct ScrollInfo(Arc<Mutex<ScrollData>>);
impl ScrollInfo {
    /// Gets the viewport bounds in the window space.
    pub fn viewport(&self) -> PxRect {
        self.viewport_transform()
            .outer_transformed(PxBox::from_size(self.viewport_size()))
            .unwrap_or_default()
            .to_rect()
    }

    /// Gets the layout size of the viewport.
    pub fn viewport_size(&self) -> PxSize {
        self.0.lock().viewport_size
    }

    /// Gets the render transform of the viewport.
    pub fn viewport_transform(&self) -> PxTransform {
        self.0.lock().viewport_transform
    }

    pub(super) fn set_viewport_size(&self, size: PxSize) {
        self.0.lock().viewport_size = size;
    }

    pub(super) fn set_viewport_transform(&self, transform: PxTransform) {
        self.0.lock().viewport_transform = transform;
    }
}

pub(super) static SCROLL_INFO_ID: StaticStateId<ScrollInfo> = StaticStateId::new_unique();

/// Smooth scrolling config.
///
/// This config can be set by the [`smooth_scrolling`] property.
///
/// [`smooth_scrolling`]: fn@smooth_scrolling.
#[derive(Clone)]
pub struct SmoothScrolling {
    /// Chase transition duration.
    ///
    /// Default is `150.ms()`.
    pub duration: Duration,
    /// Chase transition easing function.
    ///
    /// Default is linear.
    pub easing: Arc<dyn Fn(EasingTime) -> EasingStep + Send + Sync>,
}
impl fmt::Debug for SmoothScrolling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmoothScrolling")
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}
impl PartialEq for SmoothScrolling {
    // can only fail by returning `false` in some cases where the value pointer is actually equal.
    // see: https://github.com/rust-lang/rust/issues/103763
    //
    // we are fine with this, worst case is just an extra var update
    #[allow(clippy::vtable_address_comparisons)]
    fn eq(&self, other: &Self) -> bool {
        self.duration == other.duration && Arc::ptr_eq(&self.easing, &other.easing)
    }
}
impl Default for SmoothScrolling {
    fn default() -> Self {
        Self::new(150.ms(), easing::linear)
    }
}
impl SmoothScrolling {
    /// New custom smooth scrolling config.
    pub fn new(duration: Duration, easing: impl Fn(EasingTime) -> EasingStep + Send + Sync + 'static) -> Self {
        Self {
            duration,
            easing: Arc::new(easing),
        }
    }

    /// No smooth scrolling, scroll position updates immediately.
    pub fn disabled() -> Self {
        Self::new(Duration::ZERO, easing::none)
    }

    /// If this config represents [`disabled`].
    ///
    /// [`disabled`]: Self::disabled
    pub fn is_disabled(&self) -> bool {
        self.duration == Duration::ZERO
    }
}
impl_from_and_into_var! {
    /// Linear duration of smooth transition.
    fn from(duration: Duration) -> SmoothScrolling {
        SmoothScrolling {
            duration,
            ..Default::default()
        }
    }

    /// Returns default config for `true`, [`disabled`] for `false`.
    ///
    /// [`disabled`]: SmoothScrolling::disabled
    fn from(enabled: bool) -> SmoothScrolling {
        if enabled {
            SmoothScrolling::default()
        } else {
            SmoothScrolling::disabled()
        }
    }

    fn from<F: Fn(EasingTime) -> EasingStep + Send + Sync + 'static>((duration, easing): (Duration, F)) -> SmoothScrolling {
        SmoothScrolling::new(duration, easing)
    }

    fn from((duration, easing): (Duration, easing::EasingFn)) -> SmoothScrolling {
        SmoothScrolling::new(duration, easing.ease_fn())
    }
}
