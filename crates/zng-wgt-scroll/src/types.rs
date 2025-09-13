use std::{fmt, mem, sync::Arc, time::Duration};

use atomic::{Atomic, Ordering};
use bitflags::bitflags;
use parking_lot::Mutex;
use zng_ext_input::touch::TouchPhase;
use zng_var::{
    VARS,
    animation::{
        AnimationHandle, ChaseAnimation, Transition,
        easing::{self, EasingStep, EasingTime},
    },
};
use zng_wgt::prelude::*;

use super::{SMOOTH_SCROLLING_VAR, cmd};

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
        if zoom { ScrollMode::ZOOM } else { ScrollMode::NONE }
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

#[derive(Default, Debug)]
enum ZoomState {
    #[default]
    None,
    Chasing(ChaseAnimation<Factor>),
    TouchStart {
        start_factor: Factor,
        start_center: euclid::Point2D<f32, Px>,
        applied_offset: euclid::Vector2D<f32, Px>,
    },
}

#[derive(Debug)]
struct ScrollConfig {
    id: Option<WidgetId>,
    chase: [Mutex<Option<ChaseAnimation<Factor>>>; 2], // [horizontal, vertical]
    zoom: Mutex<ZoomState>,

    // last rendered horizontal, vertical offsets.
    rendered: Atomic<RenderedOffsets>,

    overscroll: [Mutex<AnimationHandle>; 2],
    inertia: [Mutex<AnimationHandle>; 2],
    auto: [Mutex<AnimationHandle>; 2],
}
impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            id: Default::default(),
            chase: Default::default(),
            zoom: Default::default(),
            rendered: Atomic::new(RenderedOffsets {
                h: 0.fct(),
                v: 0.fct(),
                z: 0.fct(),
            }),
            overscroll: Default::default(),
            inertia: Default::default(),
            auto: Default::default(),
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
    pub fn config_node(&self, child: impl IntoUiNode) -> UiNode {
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
    pub fn mode(&self) -> Var<ScrollMode> {
        SCROLL_MODE_VAR.read_only()
    }

    /// Vertical offset of the parent scroll.
    ///
    /// The value is a percentage of `content.height - viewport.height`.
    ///
    /// This variable is usually read-write, but you should avoid modifying it directly as
    /// direct assign as the value is not validated and does not participate in smooths scrolling.
    /// Prefer the scroll methods of this service to scroll.
    pub fn vertical_offset(&self) -> ContextVar<Factor> {
        SCROLL_VERTICAL_OFFSET_VAR
    }

    /// Horizontal offset of the parent scroll.
    ///
    /// The value is a percentage of `content.width - viewport.width`.
    ///
    /// This variable is usually read-write, but you should avoid modifying it directly as
    /// direct assign as the value is not validated and does not participate in smooths scrolling.
    /// Prefer the scroll methods of this service to scroll.
    pub fn horizontal_offset(&self) -> ContextVar<Factor> {
        SCROLL_HORIZONTAL_OFFSET_VAR
    }

    /// Zoom scale factor of the parent scroll.
    ///
    /// This variable is usually read-write, but you should avoid modifying it directly as
    /// direct assign as the value is not validated and does not participate in smooths scrolling.
    /// Prefer the zoom methods of this service to change scale.
    pub fn zoom_scale(&self) -> ContextVar<Factor> {
        SCROLL_SCALE_VAR
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

    /// Extra vertical offset, requested by touch gesture, that could not be fulfilled because [`vertical_offset`]
    /// is already at `0.fct()` or `1.fct()`.
    ///
    /// The factor is between in the `-1.0..=1.0` range and represents the overscroll offset in pixels divided
    /// by the viewport width.
    ///
    /// [`vertical_offset`]: Self::vertical_offset
    pub fn vertical_overscroll(&self) -> Var<Factor> {
        OVERSCROLL_VERTICAL_OFFSET_VAR.read_only()
    }

    /// Extra horizontal offset requested that could not be fulfilled because [`horizontal_offset`]
    /// is already at `0.fct()` or `1.fct()`.
    ///
    /// The factor is between in the `-1.0..=1.0` range and represents the overscroll offset in pixels divided
    /// by the viewport width.
    ///
    /// [`horizontal_offset`]: Self::horizontal_offset
    pub fn horizontal_overscroll(&self) -> Var<Factor> {
        OVERSCROLL_HORIZONTAL_OFFSET_VAR.read_only()
    }

    /// Ratio of the scroll parent viewport height to its content.
    ///
    /// The value is `viewport.height / content.height`.
    pub fn vertical_ratio(&self) -> Var<Factor> {
        SCROLL_VERTICAL_RATIO_VAR.read_only()
    }
    /// Ratio of the scroll parent viewport width to its content.
    ///
    /// The value is `viewport.width / content.width`.
    pub fn horizontal_ratio(&self) -> Var<Factor> {
        SCROLL_HORIZONTAL_RATIO_VAR.read_only()
    }

    /// If the vertical scrollbar should be visible.
    pub fn vertical_content_overflows(&self) -> Var<bool> {
        SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.read_only()
    }

    /// If the horizontal scrollbar should be visible.
    pub fn horizontal_content_overflows(&self) -> Var<bool> {
        SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.read_only()
    }

    /// Latest computed viewport size of the parent scroll.
    pub fn viewport_size(&self) -> Var<PxSize> {
        SCROLL_VIEWPORT_SIZE_VAR.read_only()
    }

    /// Latest computed content size of the parent scroll.
    pub fn content_size(&self) -> Var<PxSize> {
        SCROLL_CONTENT_SIZE_VAR.read_only()
    }

    /// Applies the `delta` to the vertical offset.
    ///
    /// If smooth scrolling is enabled it is used to update the offset.
    pub fn scroll_vertical(&self, delta: ScrollFrom) {
        self.scroll_vertical_clamp(delta, f32::MIN, f32::MAX);
    }

    /// Applies the `delta` to the horizontal offset.
    ///
    /// If smooth scrolling is enabled the chase animation is created or updated by this call.
    pub fn scroll_horizontal(&self, delta: ScrollFrom) {
        self.scroll_horizontal_clamp(delta, f32::MIN, f32::MAX)
    }

    /// Applies the `delta` to the vertical offset, but clamps the final offset by the inclusive `min` and `max`.
    ///
    /// If smooth scrolling is enabled it is used to update the offset.
    pub fn scroll_vertical_clamp(&self, delta: ScrollFrom, min: f32, max: f32) {
        self.scroll_clamp(true, SCROLL_VERTICAL_OFFSET_VAR, delta, min, max)
    }

    /// Applies the `delta` to the horizontal offset, but clamps the final offset by the inclusive `min` and `max`.
    ///
    /// If smooth scrolling is enabled it is used to update the offset.
    pub fn scroll_horizontal_clamp(&self, delta: ScrollFrom, min: f32, max: f32) {
        self.scroll_clamp(false, SCROLL_HORIZONTAL_OFFSET_VAR, delta, min, max)
    }
    fn scroll_clamp(&self, vertical: bool, scroll_offset_var: ContextVar<Factor>, delta: ScrollFrom, min: f32, max: f32) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().to_array()[vertical as usize];
        let content = SCROLL_CONTENT_SIZE_VAR.get().to_array()[vertical as usize];

        let max_scroll = content - viewport;

        if max_scroll <= 0 {
            return;
        }

        match delta {
            ScrollFrom::Var(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                let f = scroll_offset_var.get();
                SCROLL.chase(vertical, scroll_offset_var, |_| (f.0 + amount).clamp(min, max).fct());
            }
            ScrollFrom::VarTarget(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                SCROLL.chase(vertical, scroll_offset_var, |f| (f.0 + amount).clamp(min, max).fct());
            }
            ScrollFrom::Rendered(a) => {
                let amount = a.0 as f32 / max_scroll.0 as f32;
                let f = SCROLL_CONFIG.get().rendered.load(Ordering::Relaxed).h;
                SCROLL.chase(vertical, scroll_offset_var, |_| (f.0 + amount).clamp(min, max).fct());
            }
        }
    }

    /// Animate scroll at the direction and velocity (in DIPs per second).
    pub fn auto_scroll(&self, velocity: DipVector) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get();
        let content = SCROLL_CONTENT_SIZE_VAR.get();
        let max_scroll = content - viewport;

        let velocity = velocity.to_px(WINDOW.info().scale_factor());

        fn scroll(dimension: usize, velocity: Px, max_scroll: Px, offset_var: &ContextVar<Factor>) {
            if velocity == 0 {
                SCROLL_CONFIG.get().auto[dimension].lock().clone().stop();
            } else {
                let mut travel = max_scroll * offset_var.get();
                let mut target = 0.0;
                if velocity > Px(0) {
                    travel = max_scroll - travel;
                    target = 1.0;
                }
                let time = (travel.0 as f32 / velocity.0.abs() as f32).secs();

                VARS.with_animation_controller(zng_var::animation::ForceAnimationController, || {
                    let handle = offset_var.ease(target, time, easing::linear);
                    mem::replace(&mut *SCROLL_CONFIG.get().auto[dimension].lock(), handle).stop();
                });
            }
        }
        scroll(0, velocity.x, max_scroll.width, &SCROLL_HORIZONTAL_OFFSET_VAR);
        scroll(1, velocity.y, max_scroll.height, &SCROLL_VERTICAL_OFFSET_VAR);
    }

    /// Applies the `delta` to the vertical offset without smooth scrolling and
    /// updates the vertical overscroll if it changes.
    ///
    /// This method is used to implement touch gesture scrolling, the delta is always [`ScrollFrom::Var`].
    pub fn scroll_vertical_touch(&self, delta: Px) {
        self.scroll_touch(true, SCROLL_VERTICAL_OFFSET_VAR, OVERSCROLL_VERTICAL_OFFSET_VAR, delta)
    }

    /// Applies the `delta` to the horizontal offset without smooth scrolling and
    /// updates the horizontal overscroll if it changes.
    ///
    /// This method is used to implement touch gesture scrolling, the delta is always [`ScrollFrom::Var`].
    pub fn scroll_horizontal_touch(&self, delta: Px) {
        self.scroll_touch(false, SCROLL_HORIZONTAL_OFFSET_VAR, OVERSCROLL_HORIZONTAL_OFFSET_VAR, delta)
    }

    fn scroll_touch(&self, vertical: bool, scroll_offset_var: ContextVar<Factor>, overscroll_offset_var: ContextVar<Factor>, delta: Px) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().to_array()[vertical as usize];
        let content = SCROLL_CONTENT_SIZE_VAR.get().to_array()[vertical as usize];

        let max_scroll = content - viewport;
        if max_scroll <= 0 {
            return;
        }

        let delta = delta.0 as f32 / max_scroll.0 as f32;

        let current = scroll_offset_var.get();
        let mut next = current + delta.fct();
        let mut overscroll = 0.fct();
        if next > 1.fct() {
            overscroll = next - 1.fct();
            next = 1.fct();

            let overscroll_px = overscroll * content.0.fct();
            let overscroll_max = viewport.0.fct();
            overscroll = overscroll_px.min(overscroll_max) / overscroll_max;
        } else if next < 0.fct() {
            overscroll = next;
            next = 0.fct();

            let overscroll_px = -overscroll * content.0.fct();
            let overscroll_max = viewport.0.fct();
            overscroll = -(overscroll_px.min(overscroll_max) / overscroll_max);
        }

        scroll_offset_var.set(next);
        if overscroll != 0.fct() {
            let new_handle = self.increment_overscroll(overscroll_offset_var, overscroll);

            let config = SCROLL_CONFIG.get();
            let mut handle = config.overscroll[vertical as usize].lock();
            mem::replace(&mut *handle, new_handle).stop();
        } else {
            self.clear_horizontal_overscroll();
        }
    }

    fn increment_overscroll(&self, overscroll: ContextVar<Factor>, delta: Factor) -> AnimationHandle {
        enum State {
            Increment,
            ClearDelay,
            Clear(Transition<Factor>),
        }
        let mut state = State::Increment;
        overscroll.animate(move |a, o| match &mut state {
            State::Increment => {
                // set the increment and start delay to animation.
                **o += delta;
                **o = (*o).clamp((-1).fct(), 1.fct());

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
        self.clear_overscroll(true, OVERSCROLL_VERTICAL_OFFSET_VAR)
    }

    /// Quick ease horizontal overscroll to zero.
    pub fn clear_horizontal_overscroll(&self) {
        self.clear_overscroll(false, OVERSCROLL_HORIZONTAL_OFFSET_VAR)
    }

    fn clear_overscroll(&self, vertical: bool, overscroll_offset_var: ContextVar<Factor>) {
        if overscroll_offset_var.get() != 0.fct() {
            let new_handle = overscroll_offset_var.ease(0.fct(), 100.ms(), easing::linear);

            let config = SCROLL_CONFIG.get();
            let mut handle = config.overscroll[vertical as usize].lock();
            mem::replace(&mut *handle, new_handle).stop();
        }
    }

    /// Animates to `delta` over `duration`.
    pub fn scroll_vertical_touch_inertia(&self, delta: Px, duration: Duration) {
        self.scroll_touch_inertia(true, SCROLL_VERTICAL_OFFSET_VAR, OVERSCROLL_VERTICAL_OFFSET_VAR, delta, duration)
    }

    /// Animates to `delta` over `duration`.
    pub fn scroll_horizontal_touch_inertia(&self, delta: Px, duration: Duration) {
        self.scroll_touch_inertia(
            false,
            SCROLL_HORIZONTAL_OFFSET_VAR,
            OVERSCROLL_HORIZONTAL_OFFSET_VAR,
            delta,
            duration,
        )
    }

    fn scroll_touch_inertia(
        &self,
        vertical: bool,
        scroll_offset_var: ContextVar<Factor>,
        overscroll_offset_var: ContextVar<Factor>,
        delta: Px,
        duration: Duration,
    ) {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().to_array()[vertical as usize];
        let content = SCROLL_CONTENT_SIZE_VAR.get().to_array()[vertical as usize];

        let max_scroll = content - viewport;
        if max_scroll <= 0 {
            return;
        }

        let delta = delta.0 as f32 / max_scroll.0 as f32;

        let current = scroll_offset_var.get();
        let mut next = current + delta.fct();
        let mut overscroll = 0.fct();
        if next > 1.fct() {
            overscroll = next - 1.fct();
            next = 1.fct();

            let overscroll_px = overscroll * content.0.fct();
            let overscroll_max = viewport.0.fct();
            overscroll = overscroll_px.min(overscroll_max) / overscroll_max;
        } else if next < 0.fct() {
            overscroll = next;
            next = 0.fct();

            let overscroll_px = -overscroll * content.0.fct();
            let overscroll_max = viewport.0.fct();
            overscroll = -(overscroll_px.min(overscroll_max) / overscroll_max);
        }

        let cfg = SCROLL_CONFIG.get();
        let easing = |t| easing::ease_out(easing::quad, t);
        *cfg.inertia[vertical as usize].lock() = if overscroll != 0.fct() {
            let transition = Transition::new(current, next + overscroll);

            let overscroll_var = overscroll_offset_var.current_context();
            let overscroll_tr = Transition::new(overscroll, 0.fct());
            let mut is_inertia_anim = true;

            scroll_offset_var.animate(move |animation, value| {
                if is_inertia_anim {
                    // inertia ease animation
                    let step = easing(animation.elapsed(duration));
                    let v = transition.sample(step);

                    if v < 0.fct() || v > 1.fct() {
                        // follows the easing curve until cap, cuts out to overscroll indicator.
                        value.set(v.clamp_range());
                        animation.restart();
                        is_inertia_anim = false;
                        overscroll_var.set(overscroll_tr.from);
                    } else {
                        value.set(v);
                    }
                } else {
                    // overscroll clear ease animation
                    let step = easing::linear(animation.elapsed_stop(300.ms()));
                    let v = overscroll_tr.sample(step);
                    overscroll_var.set(v);
                }
            })
        } else {
            scroll_offset_var.ease(next, duration, easing)
        };
    }

    /// Set the vertical offset to a new offset derived from the last, blending into the active smooth
    /// scrolling chase animation, or starting a new one, or just setting the var if smooth scrolling is disabled.
    pub fn chase_vertical(&self, modify_offset: impl FnOnce(Factor) -> Factor) {
        self.chase(true, SCROLL_VERTICAL_OFFSET_VAR, modify_offset);
    }

    /// Set the horizontal offset to a new offset derived from the last set offset, blending into the active smooth
    /// scrolling chase animation, or starting a new one, or just setting the var if smooth scrolling is disabled.
    pub fn chase_horizontal(&self, modify_offset: impl FnOnce(Factor) -> Factor) {
        self.chase(false, SCROLL_HORIZONTAL_OFFSET_VAR, modify_offset);
    }

    fn chase(&self, vertical: bool, scroll_offset_var: ContextVar<Factor>, modify_offset: impl FnOnce(Factor) -> Factor) {
        let smooth = SMOOTH_SCROLLING_VAR.get();
        let config = SCROLL_CONFIG.get();
        let mut chase = config.chase[vertical as usize].lock();
        match &mut *chase {
            Some(t) => {
                if smooth.is_disabled() {
                    let t = modify_offset(*t.target()).clamp_range();
                    scroll_offset_var.set(t);
                    *chase = None;
                } else {
                    let easing = smooth.easing.clone();
                    t.modify(|f| *f = modify_offset(*f).clamp_range(), smooth.duration, move |t| easing(t));
                }
            }
            None => {
                let t = modify_offset(scroll_offset_var.get()).clamp_range();
                if smooth.is_disabled() {
                    scroll_offset_var.set(t);
                } else {
                    let easing = smooth.easing.clone();
                    let anim = scroll_offset_var.chase(t, smooth.duration, move |t| easing(t));
                    *chase = Some(anim);
                }
            }
        }
    }

    /// Set the zoom scale to a new scale derived from the last set scale, blending into the active
    /// smooth scaling chase animation, or starting a new or, or just setting the var if smooth scrolling is disabled.
    pub fn chase_zoom(&self, modify_scale: impl FnOnce(Factor) -> Factor) {
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
            ZoomState::Chasing(t) => {
                if smooth.is_disabled() {
                    let next = modify_scale(*t.target()).clamp(min, max);
                    SCROLL_SCALE_VAR.set(next);
                    *zoom = ZoomState::None;
                } else {
                    let easing = smooth.easing.clone();
                    t.modify(|f| *f = modify_scale(*f).clamp(min, max), smooth.duration, move |t| easing(t));
                }
            }
            _ => {
                let t = modify_scale(SCROLL_SCALE_VAR.get()).clamp(min, max);
                if smooth.is_disabled() {
                    SCROLL_SCALE_VAR.set(t);
                } else {
                    let easing = smooth.easing.clone();
                    let anim = SCROLL_SCALE_VAR.chase(t, smooth.duration, move |t| easing(t));
                    *zoom = ZoomState::Chasing(anim);
                }
            }
        }
    }

    /// Zoom in or out keeping the `origin` point in the viewport aligned with the same point
    /// in the content.
    pub fn zoom(&self, modify_scale: impl FnOnce(Factor) -> Factor, origin: PxPoint) {
        self.zoom_impl(modify_scale, origin);
    }
    fn zoom_impl(&self, modify_scale: impl FnOnce(Factor) -> Factor, center_in_viewport: PxPoint) {
        if !SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) {
            return;
        }

        let content = WIDGET.info().scroll_info().unwrap().content();
        let mut center_in_content = -content.origin + center_in_viewport.to_vector();
        let mut content_size = content.size;

        let rendered_scale = SCROLL.rendered_zoom_scale();

        SCROLL.chase_zoom(|f| {
            let s = modify_scale(f);
            let f = s / rendered_scale;
            center_in_content *= f;
            content_size *= f;
            s
        });

        let viewport_size = SCROLL_VIEWPORT_SIZE_VAR.get();

        // scroll so that new center_in_content is at the same center_in_viewport
        let max_scroll = content_size - viewport_size;
        let offset = center_in_content - center_in_viewport;

        if offset.y != Px(0) && max_scroll.height > Px(0) {
            let offset_y = offset.y.0 as f32 / max_scroll.height.0 as f32;
            SCROLL.chase_vertical(|_| offset_y.fct());
        }
        if offset.x != Px(0) && max_scroll.width > Px(0) {
            let offset_x = offset.x.0 as f32 / max_scroll.width.0 as f32;
            SCROLL.chase_horizontal(|_| offset_x.fct());
        }
    }

    /// Applies the `scale` to the current zoom scale without smooth scrolling and centered on the touch point.
    pub fn zoom_touch(&self, phase: TouchPhase, scale: Factor, center_in_viewport: euclid::Point2D<f32, Px>) {
        if !SCROLL_MODE_VAR.get().contains(ScrollMode::ZOOM) {
            return;
        }

        let cfg = SCROLL_CONFIG.get();

        let rendered_scale = SCROLL.rendered_zoom_scale();

        let start_scale;
        let start_center;

        let mut cfg = cfg.zoom.lock();

        if let TouchPhase::Start = phase {
            start_scale = rendered_scale;
            start_center = center_in_viewport;

            *cfg = ZoomState::TouchStart {
                start_factor: start_scale,
                start_center: center_in_viewport,
                applied_offset: euclid::vec2(0.0, 0.0),
            };
        } else if let ZoomState::TouchStart {
            start_factor: scale,
            start_center: center_in_viewport,
            ..
        } = &*cfg
        {
            start_scale = *scale;
            start_center = *center_in_viewport;
        } else {
            // touch canceled or not started correctly.
            return;
        }

        // applied translate offset
        let applied_offset = if let ZoomState::TouchStart { applied_offset, .. } = &mut *cfg {
            applied_offset
        } else {
            unreachable!()
        };

        let scale = start_scale + (scale - 1.0.fct());

        let min = super::MIN_ZOOM_VAR.get();
        let max = super::MAX_ZOOM_VAR.get();
        let scale = scale.clamp(min, max);

        let translate_offset = start_center - center_in_viewport;
        let translate_delta = translate_offset - *applied_offset;
        *applied_offset = translate_offset;

        let content = WIDGET.info().scroll_info().unwrap().content();
        let mut center_in_content = -content.origin.cast::<f32>() + center_in_viewport.to_vector();
        let mut content_size = content.size.cast::<f32>();

        let scale_transform = scale / rendered_scale;

        center_in_content *= scale_transform;
        content_size *= scale_transform;

        let viewport_size = SCROLL_VIEWPORT_SIZE_VAR.get().cast::<f32>();

        // scroll so that new center_in_content is at the same center_in_viewport
        let max_scroll = content_size - viewport_size;
        let zoom_offset = center_in_content - center_in_viewport;

        let offset = zoom_offset + translate_delta;

        SCROLL_SCALE_VAR.set(scale);

        if offset.y != 0.0 && max_scroll.height > 0.0 {
            let offset_y = offset.y / max_scroll.height;
            SCROLL_VERTICAL_OFFSET_VAR.set(offset_y.clamp(0.0, 1.0));
        }
        if offset.x != 0.0 && max_scroll.width > 0.0 {
            let offset_x = offset.x / max_scroll.width;
            SCROLL_HORIZONTAL_OFFSET_VAR.set(offset_x.clamp(0.0, 1.0));
        }
    }

    fn can_scroll(&self, predicate: impl Fn(PxSize, PxSize) -> bool + Send + Sync + 'static) -> Var<bool> {
        merge_var!(SCROLL_VIEWPORT_SIZE_VAR, SCROLL_CONTENT_SIZE_VAR, move |&vp, &ct| predicate(vp, ct))
    }

    /// Gets a var that is `true` when the content height is greater then the viewport height.
    pub fn can_scroll_vertical(&self) -> Var<bool> {
        self.can_scroll(|vp, ct| ct.height > vp.height)
    }

    /// Gets a var that is `true` when the content width is greater then the viewport with.
    pub fn can_scroll_horizontal(&self) -> Var<bool> {
        self.can_scroll(|vp, ct| ct.width > vp.width)
    }

    fn can_scroll_v(&self, predicate: impl Fn(PxSize, PxSize, Factor) -> bool + Send + Sync + 'static) -> Var<bool> {
        merge_var!(
            SCROLL_VIEWPORT_SIZE_VAR,
            SCROLL_CONTENT_SIZE_VAR,
            SCROLL_VERTICAL_OFFSET_VAR,
            move |&vp, &ct, &vo| predicate(vp, ct, vo)
        )
    }

    /// Gets a var that is `true` when the content height is greater then the viewport height and the vertical offset
    /// is not at the maximum.
    pub fn can_scroll_down(&self) -> Var<bool> {
        self.can_scroll_v(|vp, ct, vo| ct.height > vp.height && 1.fct() > vo)
    }

    /// Gets a var that is `true` when the content height is greater then the viewport height and the vertical offset
    /// is not at the minimum.
    pub fn can_scroll_up(&self) -> Var<bool> {
        self.can_scroll_v(|vp, ct, vo| ct.height > vp.height && 0.fct() < vo)
    }

    fn can_scroll_h(&self, predicate: impl Fn(PxSize, PxSize, Factor) -> bool + Send + Sync + 'static) -> Var<bool> {
        merge_var!(
            SCROLL_VIEWPORT_SIZE_VAR,
            SCROLL_CONTENT_SIZE_VAR,
            SCROLL_HORIZONTAL_OFFSET_VAR,
            move |&vp, &ct, &ho| predicate(vp, ct, ho)
        )
    }

    /// Gets a var that is `true` when the content width is greater then the viewport width and the horizontal offset
    /// is not at the minimum.
    pub fn can_scroll_left(&self) -> Var<bool> {
        self.can_scroll_h(|vp, ct, ho| ct.width > vp.width && 0.fct() < ho)
    }

    /// Gets a var that is `true` when the content width is greater then the viewport width and the horizontal offset
    /// is not at the maximum.
    pub fn can_scroll_right(&self) -> Var<bool> {
        self.can_scroll_h(|vp, ct, ho| ct.width > vp.width && 1.fct() > ho)
    }

    /// Scroll the [`WIDGET`] into view.
    ///
    /// [`WIDGET`]: zng_wgt::prelude::WIDGET
    pub fn scroll_to(&self, mode: impl Into<super::cmd::ScrollToMode>) {
        cmd::scroll_to(WIDGET.info(), mode.into())
    }

    /// Scroll the [`WIDGET`] into view and adjusts the zoom scale.
    ///
    /// [`WIDGET`]: zng_wgt::prelude::WIDGET
    pub fn scroll_to_zoom(&self, mode: impl Into<super::cmd::ScrollToMode>, zoom: impl Into<Factor>) {
        cmd::scroll_to_zoom(WIDGET.info(), mode.into(), zoom.into())
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

impl SCROLL {
    /// Insert the context values used by `SCROLL` in the `set`.
    ///
    /// Capturing this set plus all context vars enables using all `SCROLL` methods outside the scroll.
    pub fn context_values_set(&self, set: &mut ContextValueSet) {
        set.insert(&SCROLL_CONFIG);
    }
}

/// Scroll extensions for [`WidgetInfo`].
///
/// [`WidgetInfo`]: zng_wgt::prelude::WidgetInfo
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
        self.meta().get(*SCROLL_INFO_ID).is_some()
    }

    fn scroll_info(&self) -> Option<ScrollInfo> {
        self.meta().get(*SCROLL_INFO_ID).cloned()
    }

    fn viewport(&self) -> Option<PxRect> {
        self.meta().get(*SCROLL_INFO_ID).map(|r| r.viewport())
    }
}

#[derive(Debug)]
struct ScrollData {
    viewport_transform: PxTransform,
    viewport_size: PxSize,
    joiner_size: PxSize,
    content: PxRect,
    zoom_scale: Factor,
}
impl Default for ScrollData {
    fn default() -> Self {
        Self {
            viewport_transform: Default::default(),
            viewport_size: Default::default(),
            joiner_size: Default::default(),
            content: Default::default(),
            zoom_scale: 1.fct(),
        }
    }
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

    /// Gets the layout size of both scroll-bars.
    ///
    /// Joiner here is the corner joiner visual, it is sized by the width of the vertical bar and
    /// height of the horizontal bar.
    pub fn joiner_size(&self) -> PxSize {
        self.0.lock().joiner_size
    }

    /// Latest content offset and size.
    ///
    /// This is the content bounds, scaled and in the viewport space.
    pub fn content(&self) -> PxRect {
        self.0.lock().content
    }

    /// Latest zoom scale.
    pub fn zoom_scale(&self) -> Factor {
        self.0.lock().zoom_scale
    }

    pub(super) fn set_viewport_size(&self, size: PxSize) {
        self.0.lock().viewport_size = size;
    }

    pub(super) fn set_viewport_transform(&self, transform: PxTransform) {
        self.0.lock().viewport_transform = transform;
    }

    pub(super) fn set_joiner_size(&self, size: PxSize) {
        self.0.lock().joiner_size = size;
    }

    pub(super) fn set_content(&self, content: PxRect, scale: Factor) {
        let mut m = self.0.lock();
        m.content = content;
        m.zoom_scale = scale;
    }
}

static_id! {
    pub(super) static ref SCROLL_INFO_ID: StateId<ScrollInfo>;
}

/// Smooth scrolling config.
///
/// This config can be set by the [`smooth_scrolling`] property.
///
/// [`smooth_scrolling`]: fn@crate::smooth_scrolling
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

/// Arguments for the [`auto_scroll_indicator`] closure.
///
/// Empty struct, there are no args in the current release, this struct is declared so that if
/// args may be introduced in the future with minimal breaking changes.
///
/// Note that the [`SCROLL`] context is available during the icon closure call.
///
/// [`auto_scroll_indicator`]: fn@crate::auto_scroll_indicator
#[derive(Debug, Default, Clone, PartialEq)]
#[non_exhaustive]
pub struct AutoScrollArgs {}

/// Defines how the scale is changed by the [`ZOOM_TO_FIT_CMD`].
///
/// See the [`zoom_to_fit_mode`] property for more details.
///
/// [`ZOOM_TO_FIT_CMD`]: crate::cmd::ZOOM_TO_FIT_CMD
/// [`zoom_to_fit_mode`]: fn@crate::zoom_to_fit_mode
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ZoomToFitMode {
    /// The content is scaled down or up to fit the viewport.
    #[default]
    Contain,
    /// The content is only scaled down to fit the viewport. If the content is smaller them the viewport the scale is set to 100%.
    ScaleDown,
}
