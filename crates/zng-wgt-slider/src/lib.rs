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

use std::{any::Any, fmt, ops::Range, sync::Arc};

use colors::ACCENT_COLOR_VAR;
use parking_lot::Mutex;
use zng_var::{AnyVar, AnyVarValue, BoxedAnyVar};
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
            capture_pointer = true;
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
    fn selection(&self) -> BoxedAnyVar;
    fn set(&mut self, nearest: Factor, to: Factor);
    fn thumbs(&self) -> Vec<ThumbValue>;
    fn to_offset(&self, t: &dyn AnyVarValue) -> Option<Factor>;
    #[allow(clippy::wrong_self_convention)]
    fn from_offset(&self, offset: Factor) -> Box<dyn Any>;
}

trait OffsetConvert<T>: Send {
    fn to(&self, t: &T) -> Factor;
    fn from(&self, f: Factor) -> T;
}
impl<T, Tf: Fn(&T) -> Factor + Send, Ff: Fn(Factor) -> T + Send> OffsetConvert<T> for (Tf, Ff) {
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
    fn to_selector(value: BoxedVar<Self>, min: Self, max: Self) -> Selector;
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
        T::to_selector(selection.into_var().boxed(), min, max)
    }

    /// New with a single value thumb of type `T`.
    ///
    /// The value must convert to a normalized factor `[0.fct()..=1.fct()]` where `0.fct()` is the minimum possible value and `1.fct()` is the maximum
    /// possible value. If a value outside of this range is returned it is clamped to the range and the `selection` variable is updated back.
    pub fn value_with<T>(
        selection: impl IntoVar<T>,
        to_offset: impl Fn(&T) -> Factor + Send + 'static,
        from_offset: impl Fn(Factor) -> T + Send + 'static,
    ) -> Self
    where
        T: VarValue,
    {
        struct SingleImpl<T> {
            selection: BoxedVar<T>,
            selection_fct: Factor,
            to_from: Box<dyn OffsetConvert<T>>,
        }
        impl<T: VarValue> SelectorImpl for SingleImpl<T> {
            fn selection(&self) -> BoxedAnyVar {
                self.selection.clone_any()
            }

            fn set(&mut self, _: Factor, to: Factor) {
                self.selection_fct = to;
                let _ = self.selection.set(self.to_from.from(to));
            }

            fn thumbs(&self) -> Vec<ThumbValue> {
                vec![ThumbValue {
                    offset: self.selection_fct,
                    n_of: (0, 0),
                }]
            }

            fn to_offset(&self, t: &dyn AnyVarValue) -> Option<Factor> {
                let f = self.to_from.to(t.as_any().downcast_ref::<T>()?);
                Some(f)
            }

            fn from_offset(&self, offset: Factor) -> Box<dyn Any> {
                Box::new(self.to_from.from(offset))
            }
        }
        let selection = selection.into_var();
        Self(Arc::new(Mutex::new(SingleImpl {
            selection_fct: selection.with(&to_offset),
            selection: selection.boxed(),
            to_from: Box::new((to_offset, from_offset)),
        })))
    }

    /// New with two value thumbs of type `T` that can be set any value in the `min..=max` range.
    pub fn range<T: SelectorValue>(range: impl IntoVar<std::ops::Range<T>>, min: T, max: T) -> Self { 
        // TODO(breaking) don't use Range here, it should be an inclusive type

        // create a selector just to get the conversion closures
        let convert = T::to_selector(zng_var::LocalVar(min.clone()).boxed(), min, max);
        Self::range_with(range.into_var(), clmv!(convert, |t| convert.to_offset(t).unwrap()), move |f| {
            convert.from_offset(f).unwrap()
        })
    }

    /// New with two values thumbs that define a range of type `T`.
    ///
    /// The conversion closure have the same constraints as [`value_with`].
    ///
    /// [`value_with`]: Self::value_with
    pub fn range_with<T>(
        range: impl IntoVar<std::ops::Range<T>>,
        to_offset: impl Fn(&T) -> Factor + Send + 'static,
        from_offset: impl Fn(Factor) -> T + Send + 'static,
    ) -> Self
    where
        T: VarValue,
    {
        struct RangeImpl<T> {
            selection: BoxedVar<Range<T>>,
            selection_fct: [Factor; 2],
            to_from: Box<dyn OffsetConvert<T>>,
        }
        impl<T: VarValue> SelectorImpl for RangeImpl<T> {
            fn selection(&self) -> BoxedAnyVar {
                self.selection.clone_any()
            }

            fn set(&mut self, nearest: Factor, to: Factor) {
                if (self.selection_fct[0] - nearest).abs() < (self.selection_fct[1] - nearest).abs() {
                    self.selection_fct[0] = to;
                } else {
                    self.selection_fct[1] = to;
                }
                if self.selection_fct[0] > self.selection_fct[1] {
                    self.selection_fct.swap(0, 1);
                }
                let start = self.to_from.from(self.selection_fct[0]);
                let end = self.to_from.from(self.selection_fct[1]);
                let _ = self.selection.set(start..end);
            }

            fn thumbs(&self) -> Vec<ThumbValue> {
                vec![
                    ThumbValue {
                        offset: self.selection_fct[0],
                        n_of: (0, 2),
                    },
                    ThumbValue {
                        offset: self.selection_fct[1],
                        n_of: (1, 2),
                    },
                ]
            }

            fn to_offset(&self, t: &dyn AnyVarValue) -> Option<Factor> {
                let f = self.to_from.to(t.as_any().downcast_ref::<T>()?);
                Some(f)
            }

            fn from_offset(&self, offset: Factor) -> Box<dyn Any> {
                Box::new(self.to_from.from(offset))
            }
        }
        let selection = range.into_var();

        Self(Arc::new(Mutex::new(RangeImpl {
            selection_fct: selection.with(|r| [to_offset(&r.start), to_offset(&r.end)]),
            selection: selection.boxed(),
            to_from: Box::new((to_offset, from_offset)),
        })))
    }

    /// New with many value thumbs of type `T` that can be set any value in the `min..=max` range.
    pub fn many<T: SelectorValue>(many: impl IntoVar<Vec<T>>, min: T, max: T) -> Self {
        // create a selector just to get the conversion closures
        let convert = T::to_selector(zng_var::LocalVar(min.clone()).boxed(), min, max);
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
        to_offset: impl Fn(&T) -> Factor + Send + 'static,
        from_offset: impl Fn(Factor) -> T + Send + 'static,
    ) -> Self
    where
        T: VarValue,
    {
        struct ManyImpl<T> {
            selection: BoxedVar<Vec<T>>,
            selection_fct: Vec<Factor>,
            to_from: Box<dyn OffsetConvert<T>>,
        }
        impl<T: VarValue> SelectorImpl for ManyImpl<T> {
            fn selection(&self) -> BoxedAnyVar {
                self.selection.clone_any()
            }

            fn set(&mut self, nearest: Factor, to: Factor) {
                if let Some((i, _)) = self
                    .selection_fct
                    .iter()
                    .enumerate()
                    .map(|(i, &f)| (i, (f - nearest).abs()))
                    .reduce(|a, b| if a.1 < b.1 { a } else { b })
                {
                    self.selection_fct[i] = to;
                    self.selection_fct.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let s: Vec<_> = self.selection_fct.iter().map(|&f| self.to_from.from(f)).collect();
                    let _ = self.selection.set(s);
                }
            }

            fn thumbs(&self) -> Vec<ThumbValue> {
                let len = self.selection_fct.len().min(u16::MAX as usize) as u16;
                self.selection_fct
                    .iter()
                    .enumerate()
                    .map(|(i, &f)| ThumbValue {
                        offset: f,
                        n_of: (i.min(u16::MAX as usize) as u16, len),
                    })
                    .collect()
            }

            fn to_offset(&self, t: &dyn AnyVarValue) -> Option<Factor> {
                let f = self.to_from.to(t.as_any().downcast_ref::<T>()?);
                Some(f)
            }

            fn from_offset(&self, offset: Factor) -> Box<dyn Any> {
                Box::new(self.to_from.from(offset))
            }
        }
        let selection = many.into_var();
        Self(Arc::new(Mutex::new(ManyImpl {
            selection_fct: selection.with(|m| m.iter().map(&to_offset).collect()),
            selection: selection.boxed(),
            to_from: Box::new((to_offset, from_offset)),
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

    /// Gets the value thumbs.
    pub fn thumbs(&self) -> Vec<ThumbValue> {
        self.0.lock().thumbs()
    }

    /// Move the `nearest_thumb` to a new offset.
    ///
    /// Note that ranges don't invert, this operation may swap the thumb roles.
    pub fn set(&self, nearest_thumb: impl IntoValue<Factor>, to: impl IntoValue<Factor>) {
        self.0.lock().set(nearest_thumb.into(), to.into())
    }

    /// The selection var.
    ///
    /// Downcast to `T`, `Range<T>` or `Vec<T>` to get and set the value.
    pub fn selection(&self) -> BoxedAnyVar {
        self.0.lock().selection()
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
    thumb: ArcVar<ThumbValue>,
}
impl ThumbArgs {
    /// Variable with the thumb value that must be represented by the widget.
    pub fn thumb(&self) -> ReadOnlyArcVar<ThumbValue> {
        self.thumb.read_only()
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
        })
    }
}

fn slider_track_node() -> impl UiNode {
    let mut thumbs = ui_vec![];
    let mut thumb_vars = vec![];
    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&THUMB_FN_VAR);

            thumb_vars = SELECTOR.get().thumbs().into_iter().map(zng_var::var).collect();
            thumbs.reserve(thumb_vars.len());

            let thumb_fn = THUMB_FN_VAR.get();
            for v in &thumb_vars {
                thumbs.push(thumb_fn(ThumbArgs { thumb: v.clone() }))
            }

            thumbs.init_all();
        }
        UiNodeOp::Deinit => {
            thumbs.deinit_all();
            thumbs = ui_vec![];
            thumb_vars = vec![];
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
            let _ = thumbs.layout_each(wl, |_, n, wl| n.layout(wl), |_, _| PxSize::zero());
        }
        UiNodeOp::Update { updates } => {
            if let Some(thumb_fn) = THUMB_FN_VAR.get_new() {
                thumbs.deinit_all();
                thumb_vars.clear();
                thumbs.clear();

                for value in SELECTOR.get().thumbs() {
                    let var = zng_var::var(value);
                    let thumb = thumb_fn(ThumbArgs { thumb: var.clone() });
                    thumb_vars.push(var);
                    thumbs.push(thumb);
                }

                thumbs.init_all();

                WIDGET.update_info().layout().render();
            } else {
                thumbs.update_all(updates, &mut ());

                // sync views and vars with updated SELECTOR thumbs

                let mut thumb_values = SELECTOR.get().thumbs();
                match thumb_values.len().cmp(&thumb_vars.len()) {
                    std::cmp::Ordering::Less => {
                        // now has less thumbs
                        for mut drop in thumbs.drain(thumb_values.len()..) {
                            drop.deinit();
                        }
                        thumb_vars.truncate(thumbs.len());
                    }
                    std::cmp::Ordering::Greater => {
                        // now has more thumbs
                        let thumb_fn = THUMB_FN_VAR.get();
                        for value in thumb_values.drain(thumbs.len()..) {
                            let var = zng_var::var(value);
                            let mut thumb = thumb_fn(ThumbArgs { thumb: var.clone() });
                            thumb.init();
                            thumb_vars.push(var);
                            thumbs.push(thumb);
                        }
                    }
                    std::cmp::Ordering::Equal => {}
                }

                // reuse thumbs
                for (var, value) in thumb_vars.iter().zip(thumb_values) {
                    var.set(value);
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
                fn to_selector(value: BoxedVar<Self>, min: Self, max: Self) -> Selector {
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
                fn to_selector(value: BoxedVar<Self>, min: Self, max: Self) -> Selector {
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
    fn to_selector(value: BoxedVar<Self>, min: Self, max: Self) -> Selector {
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

    #[test]
    fn selector_value_set() {
        let s = Selector::value(10u8, 0, 100);
        s.set(10.pct(), 20.pct());
        assert_eq!(s.thumbs()[0].offset, 0.2.fct());
    }

    #[test]
    fn selector_range_set() {
        let s = Selector::range(10u8..20u8, 0, 100);
        // less then first
        s.set(0.pct(), 5.pct());
        assert_eq!(s.thumbs()[0].offset, 0.05.fct());
        assert_eq!(s.thumbs()[1].offset, 0.2.fct());

        // more then last
        s.set(25.pct(), 30.pct());
        assert_eq!(s.thumbs()[0].offset, 0.05.fct());
        assert_eq!(s.thumbs()[1].offset, 0.3.fct());

        // nearest first
        s.set(6.pct(), 7.pct());
        assert_eq!(s.thumbs()[0].offset, 0.07.fct());
        assert_eq!(s.thumbs()[1].offset, 0.3.fct());

        // invert
        s.set(7.pct(), 40.pct());
        assert_eq!(s.thumbs()[0].offset, 0.3.fct());
        assert_eq!(s.thumbs()[1].offset, 0.4.fct());
    }

    #[test]
    fn selector_range_set_eq() {
        let s = Selector::range(10u8..10, 0, 100);
        assert_eq!(s.thumbs()[0].offset, 0.1.fct());
        assert_eq!(s.thumbs()[1].offset, 0.1.fct());

        // only the last must move
        s.set(10.pct(), 20.pct());
        assert_eq!(s.thumbs()[0].offset, 0.1.fct());
        assert_eq!(s.thumbs()[1].offset, 0.2.fct());

        let s = Selector::range(10u8..10, 0, 100);
        s.set(5.pct(), 5.pct());
        assert_eq!(s.thumbs()[0].offset, 0.05.fct());
        assert_eq!(s.thumbs()[1].offset, 0.1.fct());
    }
}
