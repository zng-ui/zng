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

use parking_lot::Mutex;
use zng_var::{AnyVar, AnyVarValue, BoxedAnyVar};
use zng_wgt::prelude::*;
use zng_wgt_input::{focus::FocusableMix, pointer_capture::capture_pointer};
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};

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

/// Defines the values and ranges selected by a slider.
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
    /// New with a single value thumb of type `T`.
    ///
    /// The value must convert to a normalized factor `[0.fct()..=1.fct()]` where `0.fct()` is the minimum possible value and `1.fct()` is the maximum
    /// possible value. If a value outside of this range is returned it is clamped to the range and the `selection` variable is updated back.
    pub fn single<T>(
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

    /// New with two values thumbs that define a range of type `T`.
    ///
    /// The conversion closure have the same constraints as [`single`].
    ///
    /// [`single`]: Self::single
    pub fn range<T>(
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

    /// New with many value thumbs of type `T`.
    ///
    /// The conversion closure have the same constraints as [`single`].
    ///
    /// [`single`]: Self::single
    pub fn many<T>(
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
        Self::many(vec![], |_: &bool| 0.fct(), |_| false)
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
    /// Downcast to `T` or `Range<T>` to get and set the value.
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
