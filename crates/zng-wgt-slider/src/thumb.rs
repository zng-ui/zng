//! Slider thumb widget.

use zng_wgt::prelude::*;
use zng_wgt_input::{focus::FocusableMix, pointer_capture::capture_pointer};
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};

use crate::ThumbValue;

/// Slider thumb widget.
#[widget($crate::thumb::Thumb {
    ($value:expr) => {
        value = $value;
    }
})]
pub struct Thumb(FocusableMix<StyleMix<WidgetBase>>);
impl Thumb {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
            capture_pointer = true;
        }
    }
}
impl_style_fn!(Thumb);

/// Default slider style.
#[widget($crate::thumb::DefaultStyle)]
pub struct DefaultStyle(Style);
/// Value represented by the thumb.
#[property(CONTEXT, capture, widget_impl(Thumb))]
pub fn value(thumb: impl IntoVar<ThumbValue>) {}
