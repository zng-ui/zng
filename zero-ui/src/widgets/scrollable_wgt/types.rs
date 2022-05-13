use std::{cell::Cell, fmt, rc::Rc, time::Duration};

use crate::core::{
    context::state_key,
    units::*,
    var::{animation::EasingFn, *},
    widget_info::WidgetInfo,
};
use bitflags::bitflags;
use zero_ui_core::var::animation::ChaseAnimation;

use super::scrollable::properties::SmoothScrollingVar;

bitflags! {
    /// What dimensions are scrollable in an widget.
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
    pub struct ScrollVerticalOffsetVar: Factor = 0.fct();
    /// Horizontal offset of the parent scroll.
    ///
    /// The value is a percentage of `content.width - viewport.width`. This variable is usually read-write,
    /// scrollable content can modify it to scroll the parent.
    pub struct ScrollHorizontalOffsetVar: Factor = 0.fct();

    /// Ratio of the scroll parent viewport height to its content.
    ///
    /// The value is `viewport.height / content.height`.
    pub(super) struct ScrollVerticalRatioVar: Factor = 0.fct();

    /// Ratio of the scroll parent viewport width to its content.
    ///
    /// The value is `viewport.width / content.width`.
    pub(super) struct ScrollHorizontalRatioVar: Factor = 0.fct();

    /// If the vertical scrollbar should be visible.
    pub(super) struct ScrollVerticalContentOverflowsVar: bool = false;

    /// If the horizontal scrollbar should be visible.
    pub(super) struct ScrollHorizontalContentOverflowsVar: bool = false;

    /// Latest computed viewport size of the parent scrollable.
    pub(super) struct ScrollViewportSizeVar: PxSize = PxSize::zero();

    /// Latest computed content size of the parent scrollable.
    pub(super) struct ScrollContentSizeVar: PxSize = PxSize::zero();

    // TODO
    pub(super) struct ScrollVerticalChasingVar: Option<ChaseAnimation<Factor>> = None;
    //pub(super) struct ScrollHorizontalChasingVar: Option<ChaseAnimation<Factor>> = None;
}

/// Controls the parent scrollable.
///
/// Also see [`ScrollVerticalOffsetVar`] and [`ScrollHorizontalOffsetVar`] for controlling the scroll offset.
pub struct ScrollContext {}
impl ScrollContext {
    /// Ratio of the scroll parent viewport height to its content.
    ///
    /// The value is `viewport.height / content.height`.
    pub fn vertical_ratio() -> impl Var<Factor> {
        ScrollVerticalRatioVar::new().into_read_only()
    }
    /// Ratio of the scroll parent viewport width to its content.
    ///
    /// The value is `viewport.width / content.width`.
    pub fn horizontal_ratio() -> impl Var<Factor> {
        ScrollHorizontalRatioVar::new().into_read_only()
    }

    /// If the vertical scrollbar should be visible.
    pub fn vertical_content_overflows() -> impl Var<bool> {
        ScrollVerticalContentOverflowsVar::new().into_read_only()
    }

    /// If the horizontal scrollbar should be visible.
    pub fn horizontal_content_overflows() -> impl Var<bool> {
        ScrollHorizontalContentOverflowsVar::new().into_read_only()
    }

    /// Latest computed viewport size of the parent scrollable.
    pub fn viewport_size() -> impl Var<PxSize> {
        ScrollViewportSizeVar::new().into_read_only()
    }

    /// Latest computed content size of the parent scrollable.
    pub fn content_size() -> impl Var<PxSize> {
        ScrollContentSizeVar::new().into_read_only()
    }

    /// Offset the vertical position by the given pixel `amount`.
    pub fn scroll_vertical<Vw: WithVars>(vars: &Vw, amount: Px) {
        vars.with_vars(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).height;
            let content = ScrollContentSizeVar::get(vars).height;

            let max_scroll = content - viewport;

            if max_scroll <= Px(0) {
                return;
            }

            let curr_scroll_fct = *ScrollVerticalOffsetVar::get(vars);
            let curr_scroll = max_scroll * curr_scroll_fct;
            let new_scroll = (curr_scroll + amount).min(max_scroll).max(Px(0));

            if new_scroll != curr_scroll {
                let new_offset = new_scroll.0 as f32 / max_scroll.0 as f32;

                //smooth scrolling
                let smooth = SmoothScrollingVar::get(vars);
                if smooth.is_disabled() {
                    let _ = ScrollVerticalOffsetVar::set(vars, new_offset);
                } else {
                    match ScrollVerticalChasingVar::get(vars) {
                        Some(anim) if !anim.handle.is_stopped() => {
                            let amount = amount.0 as f32 / max_scroll.0 as f32;
                            anim.add(amount.fct());
                        }
                        _ => {
                            let ease = smooth.easing.clone();
                            let anim = ScrollVerticalOffsetVar::new().chase_bounded(
                                vars,
                                new_offset.fct(),
                                smooth.duration,
                                move |t| ease(t),
                                0.fct()..=1.fct(),
                            );
                            let _ = ScrollVerticalChasingVar::set(vars, Some(anim));
                        }
                    }
                }
            }
        })
    }

    /// Offset the horizontal position by the given pixel `amount`.
    pub fn scroll_horizontal<Vw: WithVars>(vars: &Vw, amount: Px) {
        vars.with_vars(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).width;
            let content = ScrollContentSizeVar::get(vars).width;

            let max_scroll = content - viewport;

            if max_scroll <= Px(0) {
                return;
            }

            let curr_scroll = max_scroll * *ScrollHorizontalOffsetVar::get(vars);
            let new_scroll = (curr_scroll + amount).min(max_scroll).max(Px(0));

            if new_scroll != curr_scroll {
                let new_offset = new_scroll.0 as f32 / max_scroll.0 as f32;

                //smooth scrolling
                let smooth = SmoothScrollingVar::get(vars);
                if smooth.is_disabled() {
                    let _ = ScrollHorizontalOffsetVar::set(vars, new_offset);
                } else {
                    let ease = smooth.easing.clone();
                    ScrollHorizontalOffsetVar::new()
                        .ease(vars, new_offset.fct(), smooth.duration, move |t| ease(t))
                        .perm();
                }
            }
        })
    }

    /// Returns `true` if the content height is greater then the viewport height.
    pub fn can_scroll_vertical<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).height;
            let content = ScrollContentSizeVar::get(vars).height;

            content > viewport
        })
    }

    /// Returns `true` if the content width is greater then the viewport with.
    pub fn can_scroll_horizontal<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).width;
            let content = ScrollContentSizeVar::get(vars).width;

            content > viewport
        })
    }

    /// Returns `true` if the content height is greater then the viewport height and the vertical offset
    /// is not at the maximum.
    pub fn can_scroll_down<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).height;
            let content = ScrollContentSizeVar::get(vars).height;

            content > viewport && 1.fct() > *ScrollVerticalOffsetVar::get(vars)
        })
    }

    /// Returns `true` if the content height is greater then the viewport height and the vertical offset
    /// is not at the minimum.
    pub fn can_scroll_up<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).height;
            let content = ScrollContentSizeVar::get(vars).height;

            content > viewport && 0.fct() < *ScrollVerticalOffsetVar::get(vars)
        })
    }

    /// Returns `true` if the content width is greater then the viewport width and the horizontal offset
    /// is not at the minimum.
    pub fn can_scroll_left<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).width;
            let content = ScrollContentSizeVar::get(vars).width;

            content > viewport && 0.fct() < *ScrollHorizontalOffsetVar::get(vars)
        })
    }

    /// Returns `true` if the content width is greater then the viewport width and the horizontal offset
    /// is not at the maximum.
    pub fn can_scroll_right<Vr: WithVarsRead>(vars: &Vr) -> bool {
        vars.with_vars_read(|vars| {
            let viewport = ScrollViewportSizeVar::get(vars).width;
            let content = ScrollContentSizeVar::get(vars).width;

            content > viewport && 1.fct() > *ScrollHorizontalOffsetVar::get(vars)
        })
    }
}

/// Scrollable extensions for [`WidgetInfo`].
pub trait WidgetInfoExt {
    /// Returns `true` if the widget is a [`scrollable!`](mod@super::scrollable).
    #[allow(clippy::wrong_self_convention)] // WidgetInfo is a reference.
    fn is_scrollable(self) -> bool;

    /// Returns a reference to the viewport bounds if the widget is a [`scrollable!`](mod@super::scrollable).
    fn scrollable_info(self) -> Option<ScrollableInfo>;

    /// Gets the viewport bounds relative to the scrollable widget inner bounds.
    ///
    /// The value is updated every layout, without requiring an info rebuild.
    fn viewport(self) -> Option<PxRect>;
}
impl<'a> WidgetInfoExt for WidgetInfo<'a> {
    fn is_scrollable(self) -> bool {
        self.meta().get(ScrollableInfoKey).is_some()
    }

    fn scrollable_info(self) -> Option<ScrollableInfo> {
        self.meta().get(ScrollableInfoKey).cloned()
    }

    fn viewport(self) -> Option<PxRect> {
        self.meta().get(ScrollableInfoKey).map(|r| r.viewport())
    }
}

#[derive(Debug, Default)]
struct ScrollableData {
    viewport: Cell<PxRect>,
}

/// Shared reference to the viewport bounds of a scrollable.
#[derive(Clone, Default, Debug)]
pub struct ScrollableInfo(Rc<ScrollableData>);
impl ScrollableInfo {
    /// Gets the viewport bounds in the window space.
    pub fn viewport(&self) -> PxRect {
        self.0.viewport.get()
    }

    pub(super) fn set_viewport(&self, bounds: PxRect) {
        self.0.viewport.set(bounds)
    }
}

state_key! {
    pub(super) struct ScrollableInfoKey: ScrollableInfo;
}

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

    fn from((duration, easing): (Duration, EasingFn)) -> SmoothScrolling {
        SmoothScrolling::new(duration, easing.ease_fn())
    }
}
