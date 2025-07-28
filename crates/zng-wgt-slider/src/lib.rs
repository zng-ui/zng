#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Widget for selecting a value or range by dragging a selector thumb.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

pub mod thumb;

use std::{any::Any, fmt, sync::Arc};

use colors::ACCENT_COLOR_VAR;
use parking_lot::Mutex;
use zng_ext_input::{
    mouse::{ButtonState, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT},
    pointer_capture::CaptureMode,
    touch::{TOUCH_INPUT_EVENT, TouchPhase},
};
use zng_var::{VarAny, VarValueAny};
use zng_wgt::prelude::*;
use zng_wgt_input::{focus::FocusableMix, pointer_capture::capture_pointer};
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};

/// Value selector from a range of values.
#[widget($crate::Slider)]
pub struct Slider(FocusableMix<StyleMix<WidgetBase>>);
impl Slider {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(Slider);

/// Default slider style.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            zng_wgt_container::child = SliderTrack! {
                zng_wgt::corner_radius = 5;
                zng_wgt_fill::background_color = ACCENT_COLOR_VAR.rgba();
                zng_wgt::margin = 8; // thumb overflow

                when #{SLIDER_DIRECTION_VAR}.is_horizontal() {
                    zng_wgt_size_offset::height = 5;
                }
                when #{SLIDER_DIRECTION_VAR}.is_vertical() {
                    zng_wgt_size_offset::width = 5;
                }
            };
            zng_wgt_container::child_align = Align::FILL_X;
        }
    }
}

trait SelectorImpl: Send {
    fn selection(&self) -> VarAny;
    fn thumbs(&self) -> Var<Vec<ThumbValue>>;
    fn set(&self, nearest: Factor, to: Factor);

    fn to_offset(&self, t: &dyn VarValueAny) -> Option<Factor>;
    #[allow(clippy::wrong_self_convention)]
    fn from_offset(&self, offset: Factor) -> Box<dyn Any>;
}

trait OffsetConvert<T>: Send + Sync {
    fn to(&self, t: &T) -> Factor;
    fn from(&self, f: Factor) -> T;
}
impl<T, Tf: Fn(&T) -> Factor + Send + Sync, Ff: Fn(Factor) -> T + Send + Sync> OffsetConvert<T> for (Tf, Ff) {
    fn to(&self, t: &T) -> Factor {
        (self.0)(t)
    }

    fn from(&self, f: Factor) -> T {
        (self.1)(f)
    }
}

/// Represents a type that can auto implement a [`Selector`].
///
/// # Implementing
///
/// This trait is implemented for all primitive type and Zng layout types, if a type does not you
/// can declare custom conversions using [`Selector::value`].
pub trait SelectorValue: VarValue {
    /// Make the selector.
    fn to_selector(value: Var<Self>, min: Self, max: Self) -> Selector;
}

/// Defines the values and ranges selected by a slider.
///
/// Selectors are set on the [`selector`](fn@selector) property.
#[derive(Clone)]
pub struct Selector(Arc<Mutex<dyn SelectorImpl>>);
impl fmt::Debug for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Selector(_)")
    }
}
impl PartialEq for Selector {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Selector {
    /// New with single value thumb of type `T` that can be set any value in the `min..=max` range.
    pub fn value<T: SelectorValue>(selection: impl IntoVar<T>, min: T, max: T) -> Self {
        T::to_selector(selection.into_var(), min, max)
    }

    /// New with a single value thumb of type `T`.
    ///
    /// The value must convert to a normalized factor `[0.fct()..=1.fct()]` where `0.fct()` is the minimum possible value and `1.fct()` is the maximum
    /// possible value. If a value outside of this range is returned it is clamped to the range and the `selection` variable is updated back.
    pub fn value_with<T>(
        selection: impl IntoVar<T>,
        to_offset: impl Fn(&T) -> Factor + Send + Sync + 'static,
        from_offset: impl Fn(Factor) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: VarValue,
    {
        struct SingleImpl<T: VarValue> {
            selection: Var<T>,
            thumbs: Var<Vec<ThumbValue>>,
            to_from: Arc<dyn OffsetConvert<T>>,
        }
        impl<T: VarValue> SelectorImpl for SingleImpl<T> {
            fn selection(&self) -> VarAny {
                self.selection.as_any().clone()
            }

            fn set(&self, _: Factor, to: Factor) {
                self.selection.set(self.to_from.from(to));
            }

            fn thumbs(&self) -> Var<Vec<ThumbValue>> {
                self.thumbs.clone()
            }

            fn to_offset(&self, t: &dyn VarValueAny) -> Option<Factor> {
                let f = self.to_from.to(t.downcast_ref::<T>()?);
                Some(f)
            }

            fn from_offset(&self, offset: Factor) -> Box<dyn Any> {
                Box::new(self.to_from.from(offset))
            }
        }
        let to_from = Arc::new((to_offset, from_offset));
        let selection = selection.into_var();
        let thumbs = selection.map(clmv!(to_from, |s| vec![ThumbValue {
            offset: to_from.to(s),
            n_of: (0, 1)
        }]));
        Self(Arc::new(Mutex::new(SingleImpl {
            thumbs,
            selection,
            to_from,
        })))
    }

    /// New with many value thumbs of type `T` that can be set any value in the `min..=max` range.
    pub fn many<T: SelectorValue>(many: impl IntoVar<Vec<T>>, min: T, max: T) -> Self {
        // create a selector just to get the conversion closures
        let convert = T::to_selector(zng_var::var_local(min.clone()), min, max);
        Self::many_with(many.into_var(), clmv!(convert, |t| convert.to_offset(t).unwrap()), move |f| {
            convert.from_offset(f).unwrap()
        })
    }

    /// New with many value thumbs of type `T`.
    ///
    /// The conversion closure have the same constraints as [`value_with`].
    ///
    /// [`value_with`]: Self::value_with
    pub fn many_with<T>(
        many: impl IntoVar<Vec<T>>,
        to_offset: impl Fn(&T) -> Factor + Send + Sync + 'static,
        from_offset: impl Fn(Factor) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: VarValue,
    {
        struct ManyImpl<T: VarValue> {
            selection: Var<Vec<T>>,
            thumbs: Var<Vec<ThumbValue>>,
            to_from: Arc<dyn OffsetConvert<T>>,
        }
        impl<T: VarValue> SelectorImpl for ManyImpl<T> {
            fn selection(&self) -> VarAny {
                self.selection.as_any().clone()
            }

            fn set(&self, from: Factor, to: Factor) {
                // modify selection to remove nearest and insert to in new sorted position
                // or just replace it if it is the same position

                let mut selection = self.selection.get();
                if selection.is_empty() {
                    return;
                }

                let to_value = self.to_from.from(to);

                let (remove_i, mut insert_i) = self.thumbs.with(|t| {
                    let (remove_i, _) = t
                        .iter()
                        .enumerate()
                        .map(|(i, f)| (i, (f.offset - from).abs()))
                        .reduce(|a, b| if a.1 < b.1 { a } else { b })
                        .unwrap_or((t.len(), 0.fct()));
                    let insert_i = t.iter().position(|t| t.offset >= to).unwrap_or(t.len());
                    (remove_i, insert_i)
                });

                if remove_i == insert_i {
                    selection[remove_i] = to_value;
                } else {
                    if insert_i > remove_i {
                        insert_i -= 1;
                    }
                    selection.remove(remove_i);
                    selection.insert(insert_i, to_value)
                }

                self.selection.set(selection);
            }

            fn thumbs(&self) -> Var<Vec<ThumbValue>> {
                self.thumbs.clone()
            }

            fn to_offset(&self, t: &dyn VarValueAny) -> Option<Factor> {
                let f = self.to_from.to(t.downcast_ref::<T>()?);
                Some(f)
            }

            fn from_offset(&self, offset: Factor) -> Box<dyn Any> {
                Box::new(self.to_from.from(offset))
            }
        }

        let to_from = Arc::new((to_offset, from_offset));
        let selection = many.into_var();
        let thumbs = selection.map(clmv!(to_from, |s| {
            let len = s.len().min(u16::MAX as _) as u16;
            s.iter()
                .enumerate()
                .take(u16::MAX as _)
                .map(|(i, s)| ThumbValue {
                    offset: to_from.to(s),
                    n_of: (i as u16, len),
                })
                .collect()
        }));

        Self(Arc::new(Mutex::new(ManyImpl {
            selection,
            thumbs,
            to_from,
        })))
    }

    /// New with no value thumb.
    pub fn nil() -> Self {
        Self::many_with(vec![], |_: &bool| 0.fct(), |_| false)
    }

    /// Convert the value to a normalized factor.
    ///
    /// If `T` is not the same type returns `None`.
    pub fn to_offset<T: VarValue>(&self, t: &T) -> Option<Factor> {
        self.0.lock().to_offset(t)
    }

    /// Convert the normalized factor to a value `T`.
    ///
    /// If `T` is not the same type returns `None`.
    pub fn from_offset<T: VarValue>(&self, offset: impl IntoValue<Factor>) -> Option<T> {
        let b = self.0.lock().from_offset(offset.into()).downcast().ok()?;
        Some(*b)
    }

    /// Move the thumb nearest to `from` to a new offset `to`.
    ///
    /// Note that ranges don't invert, this operation may swap the thumb roles.
    pub fn set(&self, from: impl IntoValue<Factor>, to: impl IntoValue<Factor>) {
        self.0.lock().set(from.into(), to.into())
    }

    /// The selection var.
    ///
    /// Downcast to `T`' or `Vec<T>` to get and set the value.
    pub fn selection(&self) -> VarAny {
        self.0.lock().selection()
    }

    /// Read-only variable mapped from the [`selection`].
    ///
    /// [`selection`]: Self::selection
    pub fn thumbs(&self) -> Var<Vec<ThumbValue>> {
        self.0.lock().thumbs()
    }
}

/// Represents a selector thumb in a slider.
#[derive(Clone, Debug, PartialEq, Copy)]
pub struct ThumbValue {
    offset: Factor,
    n_of: (u16, u16),
}
impl ThumbValue {
    /// Thumb offset.
    pub fn offset(&self) -> Factor {
        self.offset
    }

    /// Thumb position among others.
    ///
    /// In a single value this is `(0, 1)`, in a range this is `(0, 2)` for the start thumb and `(1, 2)` for the end thumb.
    pub fn n_of(&self) -> (u16, u16) {
        self.n_of
    }

    /// Is first thumb (smallest offset).
    pub fn is_first(&self) -> bool {
        self.n_of.0 == 0
    }

    /// Is last thumb (largest offset).
    pub fn is_last(&self) -> bool {
        self.n_of.0 == self.n_of.1
    }
}

context_local! {
    /// Contextual [`Selector`].
    pub static SELECTOR: Selector = Selector::nil();
}
context_var! {
    /// Contextual thumb function.
    pub static THUMB_FN_VAR: WidgetFn<ThumbArgs> = wgt_fn!(|a: ThumbArgs| thumb::Thumb!(a.thumb()));
}

/// Sets the slider selector that defines the values, ranges that are selected.
#[property(CONTEXT, default(Selector::nil()), widget_impl(Slider))]
pub fn selector(child: impl UiNode, selector: impl IntoValue<Selector>) -> impl UiNode {
    with_context_local(child, &SELECTOR, selector)
}

/// Widget function that converts [`ThumbArgs`] to widgets.
///
/// This property sets the [`THUMB_FN_VAR`].
#[property(CONTEXT, default(THUMB_FN_VAR))]
pub fn thumb_fn(child: impl UiNode, thumb: impl IntoVar<WidgetFn<ThumbArgs>>) -> impl UiNode {
    with_context_var(child, THUMB_FN_VAR, thumb)
}

/// Arguments for a slider thumb widget generator.
pub struct ThumbArgs {
    thumb: Var<ThumbValue>,
}
impl ThumbArgs {
    /// Variable with the thumb value that must be represented by the widget.
    pub fn thumb(&self) -> Var<ThumbValue> {
        self.thumb.clone()
    }
}

/// Slider extension methods for widget info.
pub trait WidgetInfoExt {
    /// Widget inner bounds define the slider range length.
    fn is_slider_track(&self) -> bool;

    /// Find the nearest ancestor that is a slider track.
    fn slider_track(&self) -> Option<WidgetInfo>;
}
impl WidgetInfoExt for WidgetInfo {
    fn is_slider_track(&self) -> bool {
        self.meta().flagged(*IS_SLIDER_ID)
    }

    fn slider_track(&self) -> Option<WidgetInfo> {
        self.self_and_ancestors().find(|w| w.is_slider_track())
    }
}

static_id! {
    static ref IS_SLIDER_ID: StateId<()>;
}

/// Slider orientation and direction.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SliderDirection {
    /// Horizontal. Minimum at start, maximum at end.
    ///
    /// Start is left in LTR contexts and right in RTL contexts.
    StartToEnd,
    /// Horizontal. Minimum at end, maximum at start.
    ///
    /// Start is left in LTR contexts and right in RTL contexts.
    EndToStart,
    /// Horizontal. Minimum at left, maximum at right.
    LeftToRight,
    /// Horizontal. Minimum at right, maximum at left.
    RightToLeft,
    /// Vertical. Minimum at bottom, maximum at top.
    BottomToTop,
    /// Vertical. Minimum at top, maximum at bottom.
    TopToBottom,
}
impl SliderDirection {
    /// Slider track is vertical.
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::BottomToTop | Self::TopToBottom)
    }

    /// Slider track is horizontal.
    pub fn is_horizontal(&self) -> bool {
        !self.is_vertical()
    }

    /// Convert start/end to left/right in the given `direction` context.
    pub fn layout(&self, direction: LayoutDirection) -> Self {
        match *self {
            SliderDirection::StartToEnd => {
                if direction.is_ltr() {
                    Self::LeftToRight
                } else {
                    Self::RightToLeft
                }
            }
            SliderDirection::EndToStart => {
                if direction.is_ltr() {
                    Self::RightToLeft
                } else {
                    Self::LeftToRight
                }
            }
            s => s,
        }
    }
}
impl fmt::Debug for SliderDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "SliderDirection::")?;
        }
        match self {
            Self::StartToEnd => write!(f, "StartToEnd"),
            Self::EndToStart => write!(f, "EndToStart"),
            Self::LeftToRight => write!(f, "LeftToRight"),
            Self::RightToLeft => write!(f, "RightToLeft"),
            Self::BottomToTop => write!(f, "BottomToTop"),
            Self::TopToBottom => write!(f, "TopToBottom"),
        }
    }
}

context_var! {
    /// Orientation and direction of the parent slider.
    pub static SLIDER_DIRECTION_VAR: SliderDirection = SliderDirection::StartToEnd;
}

/// Defines the orientation and direction of the slider track.
///
/// This property sets the [`SLIDER_DIRECTION_VAR`].
#[property(CONTEXT, default(SLIDER_DIRECTION_VAR), widget_impl(Slider))]
fn direction(child: impl UiNode, direction: impl IntoVar<SliderDirection>) -> impl UiNode {
    with_context_var(child, SLIDER_DIRECTION_VAR, direction)
}

/// Slider track container widget.
///
/// The slider track widget is an special container that generates thumb widgets for the slider. The widget
/// inner bounds define the track area/range.
#[widget($crate::SliderTrack)]
pub struct SliderTrack(WidgetBase);
impl SliderTrack {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            wgt.set_child(slider_track_node());
        });
        widget_set! {
            self;
            capture_pointer = CaptureMode::Subtree;
        }
    }
}

fn slider_track_node() -> impl UiNode {
    let mut thumbs = ui_vec![];
    let mut layout_direction = LayoutDirection::LTR;
    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&THUMB_FN_VAR)
                .sub_event(&MOUSE_INPUT_EVENT)
                .sub_event(&TOUCH_INPUT_EVENT)
                .sub_event(&MOUSE_MOVE_EVENT);

            let thumb_fn = THUMB_FN_VAR.get();

            let thumbs_var = SELECTOR.get().thumbs();
            let thumbs_len = thumbs_var.with(|t| t.len());
            thumbs.reserve(thumbs_len);
            for i in 0..thumbs_len {
                let thumb_var = thumbs_var.map(move |t| {
                    t.get(i).copied().unwrap_or(ThumbValue {
                        offset: 0.fct(),
                        n_of: (0, 0),
                    })
                });
                thumbs.push(thumb_fn(ThumbArgs { thumb: thumb_var }))
            }

            thumbs.init_all();
        }
        UiNodeOp::Deinit => {
            thumbs.deinit_all();
            thumbs = ui_vec![];
        }
        UiNodeOp::Info { info } => {
            info.flag_meta(*IS_SLIDER_ID);
            thumbs.info_all(info);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, wl } => {
            *final_size = LAYOUT.constraints().fill_size();
            layout_direction = LAYOUT.direction();
            let _ = thumbs.layout_each(wl, |_, n, wl| n.layout(wl), |_, _| PxSize::zero());
        }
        UiNodeOp::Event { update } => {
            thumbs.event_all(update);

            let mut pos = None;

            if let Some(args) = MOUSE_MOVE_EVENT.on_unhandled(update) {
                if let Some(cap) = &args.capture {
                    if cap.target.contains(WIDGET.id()) {
                        pos = Some(args.position);
                        args.propagation().stop();
                    }
                }
            } else if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update) {
                if args.state == ButtonState::Pressed {
                    pos = Some(args.position);
                    args.propagation().stop();
                }
            } else if let Some(args) = TOUCH_INPUT_EVENT.on_unhandled(update) {
                if args.phase == TouchPhase::Start {
                    pos = Some(args.position);
                    args.propagation().stop();
                }
            }

            if let Some(pos) = pos {
                let track_info = WIDGET.info();
                let track_bounds = track_info.inner_bounds();
                let track_orientation = SLIDER_DIRECTION_VAR.get();

                let (track_min, track_max) = match track_orientation.layout(layout_direction) {
                    SliderDirection::LeftToRight => (track_bounds.min_x(), track_bounds.max_x()),
                    SliderDirection::RightToLeft => (track_bounds.max_x(), track_bounds.min_x()),
                    SliderDirection::BottomToTop => (track_bounds.max_y(), track_bounds.min_y()),
                    SliderDirection::TopToBottom => (track_bounds.min_y(), track_bounds.max_y()),
                    _ => unreachable!(),
                };
                let cursor = if track_orientation.is_horizontal() {
                    pos.x.to_px(track_info.tree().scale_factor())
                } else {
                    pos.y.to_px(track_info.tree().scale_factor())
                };
                let new_offset = (cursor - track_min).0 as f32 / (track_max - track_min).abs().0 as f32;
                let new_offset = new_offset.fct().clamp_range();

                let selector = crate::SELECTOR.get();
                selector.set(new_offset, new_offset);
            }
        }
        UiNodeOp::Update { updates } => {
            if let Some(thumb_fn) = THUMB_FN_VAR.get_new() {
                thumbs.deinit_all();
                thumbs.clear();

                let thumbs_var = SELECTOR.get().thumbs();
                let thumbs_len = thumbs_var.with(|t| t.len());
                thumbs.reserve(thumbs_len);
                for i in 0..thumbs_len {
                    let thumb_var = thumbs_var.map(move |t| {
                        t.get(i).copied().unwrap_or(ThumbValue {
                            offset: 0.fct(),
                            n_of: (0, 0),
                        })
                    });
                    thumbs.push(thumb_fn(ThumbArgs { thumb: thumb_var }))
                }

                thumbs.init_all();

                WIDGET.update_info().layout().render();
            } else {
                thumbs.update_all(updates, &mut ());

                // sync views and vars with updated SELECTOR thumbs

                let thumbs_var = SELECTOR.get().thumbs();
                let thumbs_len = thumbs_var.with(|t| t.len());

                match thumbs_len.cmp(&thumbs.len()) {
                    std::cmp::Ordering::Less => {
                        // now has less thumbs
                        for mut drop in thumbs.drain(thumbs_len..) {
                            drop.deinit();
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        // now has more thumbs
                        let thumb_fn = THUMB_FN_VAR.get();
                        let from_len = thumbs.len();
                        thumbs.reserve(thumbs_len - from_len);
                        for i in from_len..thumbs_len {
                            let thumb_var = thumbs_var.map(move |t| {
                                t.get(i).copied().unwrap_or(ThumbValue {
                                    offset: 0.fct(),
                                    n_of: (0, 0),
                                })
                            });
                            thumbs.push(thumb_fn(ThumbArgs { thumb: thumb_var }))
                        }
                    }
                    std::cmp::Ordering::Equal => {}
                }
            }
        }
        op => thumbs.op(op),
    })
}

macro_rules! impl_32 {
    (($to_f32:expr, $from_f32:expr) => $($T:ident),+ $(,)?) => {
        $(
            impl SelectorValue for $T {
                #[allow(clippy::unnecessary_cast)]
                fn to_selector(value: Var<Self>, min: Self, max: Self) -> Selector {
                    let to_f32 = $to_f32;
                    let from_f32 = $from_f32;

                    let min = to_f32(min);
                    let max = to_f32(max);
                    if min >= max {
                        Selector::nil()
                    } else {
                        let d = max - min;
                        Selector::value_with(value, move |i| {
                            let i = to_f32(i.clone());
                            ((i - min)  / d).fct()
                        }, move |f| {
                            from_f32((f.0 * d + min).round())
                        })
                    }
                }
            }
        )+
    };
}
impl_32!((|i| i as f32, |f| f as Self) => u32, i32, u16, i16, u8, i8, f32);
impl_32!((|p: Self| p.0 as f32, |f| Self(f as _)) => Px, Factor, FactorPercent);
impl_32!((|p: Dip| p.to_f32(), |f| Dip::new_f32(f)) => Dip);

macro_rules! impl_64 {
    ($($T:ident),+ $(,)?) => {
        $(
            impl SelectorValue for $T {
                fn to_selector(value: Var<Self>, min: Self, max: Self) -> Selector {
                    let min = min as f64;
                    let max = max as f64;
                    if min >= max {
                        Selector::nil()
                    } else {
                        let d = max - min;
                        Selector::value_with(value, move |&i| {
                            let i = i as f64;
                            Factor(((i - min)  / d) as f32)
                        }, move |f| {
                            ((f.0 as f64) * d + min).round() as Self
                        })
                    }
                }
            }
        )+
    };
}
impl_64!(u64, i64, u128, i128, f64);

impl SelectorValue for Length {
    fn to_selector(value: Var<Self>, min: Self, max: Self) -> Selector {
        let (min_f32, max_f32) = LAYOUT.with_context(LayoutMetrics::new(1.fct(), PxSize::splat(Px(1000)), Px(16)), || {
            (min.layout_f32(LayoutAxis::X), max.layout_f32(LayoutAxis::X))
        });
        if min_f32 >= max_f32 {
            Selector::nil()
        } else {
            let d_f32 = max_f32 - min_f32;
            let d = max - min.clone();
            Selector::value_with(
                value,
                move |l| {
                    let l = LAYOUT.with_context(LayoutMetrics::new(1.fct(), PxSize::splat(Px(1000)), Px(16)), || {
                        l.layout_f32(LayoutAxis::X)
                    });
                    Factor((l - min_f32) / d_f32)
                },
                move |f| d.clone() * f + min.clone(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selector_value_t<T: SelectorValue>(min: T, max: T) {
        let s = Selector::value(min.clone(), min.clone(), max.clone());
        assert_eq!(s.to_offset(&min), Some(0.fct()));
        assert_eq!(s.to_offset(&max), Some(1.fct()));
        assert_eq!(s.from_offset(0.fct()), Some(min));
        assert_eq!(s.from_offset(1.fct()), Some(max));
    }

    #[test]
    fn selector_value_u8() {
        selector_value_t(u8::MIN, u8::MAX);
        selector_value_t(20u8, 120u8);
    }

    #[test]
    fn selector_value_i32() {
        selector_value_t(i32::MIN, i32::MAX);
        selector_value_t(20i32, 120i32);
    }

    #[test]
    fn selector_value_i64() {
        selector_value_t(i64::MIN, i64::MAX);
    }

    #[test]
    fn selector_value_f64() {
        selector_value_t(-200f64, 200f64);
        selector_value_t(20f64, 120f64);
    }

    #[test]
    fn selector_pct() {
        selector_value_t(0.pct(), 100.pct());
    }

    #[test]
    fn selector_value_px() {
        selector_value_t(Px(20), Px(200));
    }

    #[test]
    fn selector_value_dip() {
        selector_value_t(Dip::new(20), Dip::new(200));
    }

    #[test]
    fn selector_value_length() {
        selector_value_t(20.px(), 200.px());
    }
}
