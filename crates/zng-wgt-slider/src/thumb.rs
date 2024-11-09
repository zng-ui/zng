//! Slider thumb widget.

use zng_ext_input::mouse::MOUSE_MOVE_EVENT;
use zng_wgt::prelude::*;
use zng_wgt_input::{focus::FocusableMix, pointer_capture::capture_pointer};
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};

use crate::{SliderDirection, ThumbValue, WidgetInfoExt as _, SLIDER_DIRECTION_VAR};

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
                zng_wgt_size_offset::offset = (-3 -10/2, -3 -5/2); // track is 5 height
            }
            when #{crate::SLIDER_DIRECTION_VAR}.is_vertical() {
                zng_wgt_size_offset::offset = (-3 -5/2, -3 -10/2);
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
fn thumb_event_layout_node(child: impl UiNode, value: impl IntoVar<ThumbValue>) -> impl UiNode {
    let value = value.into_var();
    let mut layout_direction = LayoutDirection::LTR;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&value).sub_event(&MOUSE_MOVE_EVENT);
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if let Some(args) = MOUSE_MOVE_EVENT.on_unhandled(update) {
                if let Some(c) = &args.capture {
                    if c.target.widget_id() == WIDGET.id() {
                        let thumb_info = WIDGET.info();
                        let track_info = match thumb_info.slider_track() {
                            Some(i) => i,
                            None => {
                                tracing::error!("slider::Thumb is not inside a slider_track");
                                return;
                            }
                        };
                        args.propagation().stop();

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
                            args.position.x.to_px(track_info.tree().scale_factor())
                        } else {
                            args.position.y.to_px(track_info.tree().scale_factor())
                        };
                        let new_offset = (cursor - track_min).0 as f32 / (track_max - track_min).abs().0 as f32;

                        let selector = crate::SELECTOR.get();
                        selector.set(value.get().offset(), new_offset.fct().clamp_range());
                        WIDGET.update();
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            layout_direction = LAYOUT.direction();

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
        _ => {}
    })
}
