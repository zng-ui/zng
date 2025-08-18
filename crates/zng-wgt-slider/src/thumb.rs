//! Slider thumb widget.

use zng_wgt::prelude::*;
use zng_wgt_input::{focus::FocusableMix, pointer_capture::capture_pointer};
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};

use crate::{SLIDER_DIRECTION_VAR, SliderDirection, ThumbValue};

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

        self.widget_builder()
            .push_build_action(|wgt| match wgt.capture_var::<ThumbValue>(property_id!(Self::value)) {
                Some(v) => {
                    wgt.push_intrinsic(NestGroup::LAYOUT, "event-layout", move |c| thumb_event_layout_node(c, v));
                }
                None => tracing::error!("missing required `slider::Thumb::value` property"),
            });

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
            // this to enable visual feedback on thumb (is_cap_hovered)
            // the SliderTrack also captures the subtree
            capture_pointer = true;
        }
    }
}
impl_style_fn!(Thumb);

/// Default slider style.
#[widget($crate::thumb::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            zng_wgt::border = 3, LightDark::new(colors::BLACK, colors::WHITE).rgba_into();
            zng_wgt_size_offset::force_size = 10 + 3 + 3;
            zng_wgt::corner_radius = 16;
            zng_wgt_fill::background_color = colors::ACCENT_COLOR_VAR.rgba();

            when #{crate::SLIDER_DIRECTION_VAR}.is_horizontal() {
                zng_wgt_size_offset::offset = (-3 - 10 / 2, -3 - 5 / 2); // track is 5 height
            }
            when #{crate::SLIDER_DIRECTION_VAR}.is_vertical() {
                zng_wgt_size_offset::offset = (-3 - 5 / 2, -3 - 10 / 2);
            }

            #[easing(150.ms())]
            zng_wgt_transform::scale = 100.pct();
            when *#zng_wgt_input::is_cap_hovered {
                #[easing(0.ms())]
                zng_wgt_transform::scale = 120.pct();
            }
        }
    }
}

/// Value represented by the thumb.
#[property(CONTEXT, capture, widget_impl(Thumb))]
pub fn value(thumb: impl IntoVar<ThumbValue>) {}

/// Main thumb implementation.
///
/// Handles mouse and touch drag, applies the thumb offset as translation on layout.
fn thumb_event_layout_node(child: impl IntoUiNode, value: impl IntoVar<ThumbValue>) -> UiNode {
    let value = value.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&value);
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            let layout_direction = LAYOUT.direction();

            // max if bounded, otherwise min.
            let c = LAYOUT.constraints();
            let track_size = c.with_fill_vector(c.is_bounded()).fill_size();
            let track_orientation = SLIDER_DIRECTION_VAR.get();
            let offset = value.get().offset;

            let offset = match track_orientation.layout(layout_direction) {
                SliderDirection::LeftToRight => track_size.width * offset,
                SliderDirection::RightToLeft => track_size.width - (track_size.width * offset),
                SliderDirection::BottomToTop => track_size.height - (track_size.height * offset),
                SliderDirection::TopToBottom => track_size.height * offset,
                _ => unreachable!(),
            };
            let offset = if track_orientation.is_horizontal() {
                PxVector::new(offset, Px(0))
            } else {
                PxVector::new(Px(0), offset)
            };
            wl.translate(offset);
        }
        // Actual "drag" is implemented by the parent SliderTrack
        _ => {}
    })
}
