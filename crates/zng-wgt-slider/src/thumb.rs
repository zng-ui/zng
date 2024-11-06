//! Slider thumb widget.

use zng_ext_input::mouse::MOUSE_MOVE_EVENT;
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

        self.widget_builder().push_build_action(|wgt| {
            match wgt.capture_var::<ThumbValue>(property_id!(Self::value)) {
                Some(v) => {
                    wgt.push_intrinsic(NestGroup::LAYOUT, "layout", move |c| thumb_layout_node(c, v));
                },
                None => tracing::error!("missing required `slider::Thumb::value` property"),
            }
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

/// Value represented by the thumb.
#[property(CONTEXT, capture, widget_impl(Thumb))]
pub fn value(thumb: impl IntoVar<ThumbValue>) {}

/// Main thumb implementation.
pub fn thumb_layout_node(child: impl UiNode, value: impl IntoVar<ThumbValue>) -> impl UiNode {
    let value = value.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&value).sub_event(&MOUSE_MOVE_EVENT);
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if let Some(args) = MOUSE_MOVE_EVENT.on_unhandled(update) {
                if let Some(c) = &args.capture {
                    if c.target.widget_id() == WIDGET.id() {
                        args.propagation().stop();

                        let selector = crate::SELECTOR.get();
                        let thumb_info = WIDGET.info();
                        let prev_l = thumb_info.inner_bounds().origin.x;
                        let next_l = args.position.x;
                        // !!: TODO 
                        // * compute new offset
                        //    - need to know the slider parent.
                        //    - what if the slider parent has padding? Need to know the exact track rectangle.
                        //    - need to know the slider orientation.
                        // * what happens when there is more then one thumb in the same exact spot?
                        let value = value.get();
                        selector.set(value.offset(), 0.fct());
                    }
                }
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);

        }
        _ => {}
    })
}
