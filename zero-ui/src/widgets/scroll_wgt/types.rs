use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
    time::Duration,
};

use crate::core::{
    context::StaticStateId,
    units::*,
    var::{animation::*, *},
    widget_info::WidgetInfo,
    UiNode,
};
use bitflags::bitflags;
use zero_ui_core::var::animation::ChaseAnimation;

use super::scroll::properties::SMOOTH_SCROLLING_VAR;

bitflags! {
    /// What dimensions are scrollable in a widget.
    ///
    /// If a dimension is scrollable the content can be any size in that dimension, if the size
    /// is more then available scrolling is enabled for that dimension.
    pub struct ScrollMode: u8 {
        /// Content is not scrollable.
        const NONE = 0;
        /// Content can be any height.
        const VERTICAL = 0b01;
        /// Content can be any width.
        const HORIZONTAL = 0b10;
        /// Content can be any size.
        const ALL = 0b11;
    }
}
impl_from_and_into_var! {
    /// Returns [`ALL`] for `true` and [`NONE`] for `false`.
    ///
    /// [`ALL`]: ScrollMode::ALL
    /// [`NONE`]: ScrollMode::NONE
    fn from(all: bool) -> ScrollMode {
        if all {
            ScrollMode::ALL
        } else {
            ScrollMode::NONE
        }
    }
}

context_var! {
    /// Vertical offset of the parent scroll.
    ///
    /// The value is a percentage of `content.height - viewport.height`. This variable is usually read-write,
    /// scrollable content can modify it to scroll the parent.
    pub static SCROLL_VERTICAL_OFFSET_VAR: Factor = 0.fct();
    /// Horizontal offset of the parent scroll.
    ///
    /// The value is a percentage of `content.width - viewport.width`. This variable is usually read-write,
    /// scrollable content can modify it to scroll the parent.
    pub static SCROLL_HORIZONTAL_OFFSET_VAR: Factor = 0.fct();

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
    pub(super) static SCROLL_CONTENT_SIZE_VAR: PxSize = PxSize::zero();

    static SCROLL_CONFIG_VAR: RefCell<ScrollConfig> = RefCell::default();
}

#[derive(Debug, Clone, Default)]
struct ScrollConfig {
    horizontal: Option<ChaseAnimation<Factor>>,
    vertical: Option<ChaseAnimation<Factor>>,
}

/// Controls the parent scroll.
///
/// Also see [`SCROLL_VERTICAL_OFFSET_VAR`] and [`SCROLL_HORIZONTAL_OFFSET_VAR`] for controlling the scroll offset.
pub struct ScrollContext {}
impl ScrollContext {
    /// New node that holds data for the [`ScrollContext`] operation.
    ///
    /// Scroll implementers must add this node to their context.
    pub fn config_node(child: impl UiNode) -> impl UiNode {
        with_context_var(child, SCROLL_CONFIG_VAR, RefCell::default())
    }

    /// Ratio of the scroll parent viewport height to its content.
    ///
    /// The value is `viewport.height / content.height`.
    pub fn vertical_ratio() -> ReadOnlyContextVar<Factor> {
        SCROLL_VERTICAL_RATIO_VAR.into_read_only()
    }
    /// Ratio of the scroll parent viewport width to its content.
    ///
    /// The value is `viewport.width / content.width`.
    pub fn horizontal_ratio() -> ReadOnlyContextVar<Factor> {
        SCROLL_HORIZONTAL_RATIO_VAR.into_read_only()
    }

    /// If the vertical scrollbar should be visible.
    pub fn vertical_content_overflows() -> ReadOnlyContextVar<bool> {
        SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.into_read_only()
    }

    /// If the horizontal scrollbar should be visible.
    pub fn horizontal_content_overflows() -> ReadOnlyContextVar<bool> {
        SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.into_read_only()
    }

    /// Latest computed viewport size of the parent scroll.
    pub fn viewport_size() -> ReadOnlyContextVar<PxSize> {
        SCROLL_VIEWPORT_SIZE_VAR.into_read_only()
    }

    /// Latest computed content size of the parent scroll.
    pub fn content_size() -> ReadOnlyContextVar<PxSize> {
        SCROLL_CONTENT_SIZE_VAR.into_read_only()
    }

    /// Offset the vertical position by the given pixel `amount`.
    pub fn scroll_vertical<Vw: WithVars>(vars: &Vw, amount: Px) {
        vars.with_vars(|vars| {
            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
            let content = SCROLL_CONTENT_SIZE_VAR.get().height;

            let max_scroll = content - viewport;

            if max_scroll <= Px(0) {
                return;
            }

            let curr_scroll_fct = SCROLL_VERTICAL_OFFSET_VAR.get();
            let curr_scroll = max_scroll * curr_scroll_fct;
            let new_scroll = (curr_scroll + amount).min(max_scroll).max(Px(0));

            if new_scroll != curr_scroll {
                let new_offset = new_scroll.0 as f32 / max_scroll.0 as f32;
                ScrollContext::chase_vertical(vars, new_offset.fct());
            }
        })
    }

    /// Offset the horizontal position by the given pixel `amount`.
    pub fn scroll_horizontal<Vw: WithVars>(vars: &Vw, amount: Px) {
        vars.with_vars(|vars| {
            let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
            let content = SCROLL_CONTENT_SIZE_VAR.get().width;

            let max_scroll = content - viewport;

            if max_scroll <= Px(0) {
                return;
            }

            let curr_scroll_fct = SCROLL_HORIZONTAL_OFFSET_VAR.get();
            let curr_scroll = max_scroll * curr_scroll_fct;
            let new_scroll = (curr_scroll + amount).min(max_scroll).max(Px(0));

            if new_scroll != curr_scroll {
                let new_offset = new_scroll.0 as f32 / max_scroll.0 as f32;
                ScrollContext::chase_horizontal(vars, new_offset.fct());
            }
        })
    }

    /// Set the [`SCROLL_VERTICAL_OFFSET_VAR`] to `offset`, blending into the active smooth scrolling chase animation, or starting a new one, or
    /// just setting the var if smooth scrolling is disabled.
    pub fn chase_vertical<Vw: WithVars, F: Into<Factor>>(vars: &Vw, new_offset: F) {
        vars.with_vars(|vars| {
            let new_offset = new_offset.into().clamp_range();

            //smooth scrolling
            let smooth = SMOOTH_SCROLLING_VAR.get();
            if smooth.is_disabled() {
                let _ = SCROLL_VERTICAL_OFFSET_VAR.set(vars, new_offset);
            } else {
                let config = SCROLL_CONFIG_VAR.get();
                let mut config = config.borrow_mut();

                match &config.vertical {
                    Some(anim) if !anim.handle.is_stopped() => {
                        anim.add(new_offset - SCROLL_VERTICAL_OFFSET_VAR.get());
                    }
                    _ => {
                        let ease = smooth.easing.clone();
                        let anim = SCROLL_VERTICAL_OFFSET_VAR.chase_bounded(
                            vars,
                            new_offset,
                            smooth.duration,
                            move |t| ease(t),
                            0.fct()..=1.fct(),
                        );
                        config.vertical = Some(anim);
                    }
                }
            }
        })
    }

    /// Set the [`SCROLL_HORIZONTAL_OFFSET_VAR`] to `offset`, blending into the active smooth scrolling chase animation, or starting a new one, or
    /// just setting the var if smooth scrolling is disabled.
    pub fn chase_horizontal<Vw: WithVars, F: Into<Factor>>(vars: &Vw, new_offset: F) {
        vars.with_vars(|vars| {
            let new_offset = new_offset.into().clamp_range();

            //smooth scrolling
            let smooth = SMOOTH_SCROLLING_VAR.get();
            if smooth.is_disabled() {
                let _ = SCROLL_HORIZONTAL_OFFSET_VAR.set(vars, new_offset);
            } else {
                let config = SCROLL_CONFIG_VAR.get();
                let mut config = config.borrow_mut();

                match &config.horizontal {
                    Some(anim) if !anim.handle.is_stopped() => {
                        anim.add(new_offset - SCROLL_HORIZONTAL_OFFSET_VAR.get());
                    }
                    _ => {
                        let ease = smooth.easing.clone();
                        let anim = SCROLL_HORIZONTAL_OFFSET_VAR.chase_bounded(
                            vars,
                            new_offset,
                            smooth.duration,
                            move |t| ease(t),
                            0.fct()..=1.fct(),
                        );
                        config.horizontal = Some(anim);
                    }
                }
            }
        })
    }

    /// Returns `true` if the content height is greater then the viewport height.
    pub fn can_scroll_vertical() -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        content > viewport
    }

    /// Returns `true` if the content width is greater then the viewport with.
    pub fn can_scroll_horizontal() -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        content > viewport
    }

    /// Returns `true` if the content height is greater then the viewport height and the vertical offset
    /// is not at the maximum.
    pub fn can_scroll_down() -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        content > viewport && 1.fct() > SCROLL_VERTICAL_OFFSET_VAR.get()
    }

    /// Returns `true` if the content height is greater then the viewport height and the vertical offset
    /// is not at the minimum.
    pub fn can_scroll_up() -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().height;
        let content = SCROLL_CONTENT_SIZE_VAR.get().height;

        content > viewport && 0.fct() < SCROLL_VERTICAL_OFFSET_VAR.get()
    }

    /// Returns `true` if the content width is greater then the viewport width and the horizontal offset
    /// is not at the minimum.
    pub fn can_scroll_left() -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        content > viewport && 0.fct() < SCROLL_HORIZONTAL_OFFSET_VAR.get()
    }

    /// Returns `true` if the content width is greater then the viewport width and the horizontal offset
    /// is not at the maximum.
    pub fn can_scroll_right() -> bool {
        let viewport = SCROLL_VIEWPORT_SIZE_VAR.get().width;
        let content = SCROLL_CONTENT_SIZE_VAR.get().width;

        content > viewport && 1.fct() > SCROLL_HORIZONTAL_OFFSET_VAR.get()
    }
}

/// Scroll extensions for [`WidgetInfo`].
pub trait WidgetInfoExt {
    /// Returns `true` if the widget is a [`scroll!`](mod@super::scroll).
    #[allow(clippy::wrong_self_convention)] // WidgetInfo is a reference.
    fn is_scroll(self) -> bool;

    /// Returns a reference to the viewport bounds if the widget is a [`scroll!`](mod@super::scroll).
    fn scroll_info(self) -> Option<ScrollInfo>;

    /// Gets the viewport bounds relative to the scroll widget inner bounds.
    ///
    /// The value is updated every layout and render, without requiring an info rebuild.
    fn viewport(self) -> Option<PxRect>;
}
impl<'a> WidgetInfoExt for WidgetInfo<'a> {
    fn is_scroll(self) -> bool {
        self.meta().get(&SCROLL_INFO_ID).is_some()
    }

    fn scroll_info(self) -> Option<ScrollInfo> {
        self.meta().get(&SCROLL_INFO_ID).cloned()
    }

    fn viewport(self) -> Option<PxRect> {
        self.meta().get(&SCROLL_INFO_ID).map(|r| r.viewport())
    }
}

#[derive(Debug, Default)]
struct ScrollData {
    viewport_transform: Cell<PxTransform>,
    viewport_size: Cell<PxSize>,
}

/// Shared reference to the viewport bounds of a scroll.
#[derive(Clone, Default, Debug)]
pub struct ScrollInfo(Rc<ScrollData>);
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
        self.0.viewport_size.get()
    }

    /// Gets the render transform of the viewport.
    pub fn viewport_transform(&self) -> PxTransform {
        self.0.viewport_transform.get()
    }

    pub(super) fn set_viewport_size(&self, size: PxSize) {
        self.0.viewport_size.set(size)
    }

    pub(super) fn set_viewport_transform(&self, transform: PxTransform) {
        self.0.viewport_transform.set(transform)
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
    pub easing: Rc<dyn Fn(EasingTime) -> EasingStep>,
}
impl fmt::Debug for SmoothScrolling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmoothScrolling")
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}
impl Default for SmoothScrolling {
    fn default() -> Self {
        Self::new(150.ms(), easing::linear)
    }
}
impl SmoothScrolling {
    /// New custom smooth scrolling config.
    pub fn new(duration: Duration, easing: impl Fn(EasingTime) -> EasingStep + 'static) -> Self {
        Self {
            duration,
            easing: Rc::new(easing),
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

    fn from<F: Fn(EasingTime) -> EasingStep + Clone + 'static>((duration, easing): (Duration, F)) -> SmoothScrolling {
        SmoothScrolling::new(duration, easing)
    }

    fn from((duration, easing): (Duration, easing::EasingFn)) -> SmoothScrolling {
        SmoothScrolling::new(duration, easing.ease_fn())
    }
}
