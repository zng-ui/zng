//! Thumb widget, properties and nodes..

use super::*;
use scrollbar::ORIENTATION_VAR;
use zng_ext_input::mouse::{ClickMode, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT};
use zng_wgt_fill::background_color;
use zng_wgt_input::{click_mode, is_cap_pressed, is_hovered, pointer_capture::capture_pointer};

/// Scrollbar thumb widget.
#[widget($crate::Thumb)]
pub struct Thumb(WidgetBase);
impl Thumb {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            background_color = rgba(200, 200, 200, 50.pct());
            capture_pointer = true;
            click_mode = ClickMode::default(); // scrollbar sets to repeat

            when *#is_hovered {
                background_color = rgba(200, 200, 200, 70.pct());
            }

            when *#is_cap_pressed {
                background_color = rgba(200, 200, 200, 90.pct());
            }
        }

        self.widget_builder().push_build_action(on_build);
    }
}

/// Viewport/content ratio.
///
/// This becomes the height for vertical and width for horizontal.
#[property(LAYOUT, capture, widget_impl(Thumb))]
pub fn viewport_ratio(ratio: impl IntoVar<Factor>) {}

/// Content offset.
#[property(LAYOUT, capture, widget_impl(Thumb))]
pub fn offset(offset: impl IntoVar<Factor>) {}

/// Width if orientation is vertical, otherwise height if orientation is horizontal.
#[property(SIZE, capture, default(16), widget_impl(Thumb))]
pub fn cross_length(length: impl IntoVar<Length>) {}

fn on_build(wgt: &mut WidgetBuilding) {
    let cross_length = wgt.capture_var_or_else::<Length, _>(property_id!(cross_length), || 16);
    wgt.push_intrinsic(NestGroup::SIZE, "orientation-size", move |child| {
        zng_wgt_size_offset::size(
            child,
            var_merge!(ORIENTATION_VAR, THUMB_VIEWPORT_RATIO_VAR, cross_length, |o, r, l| {
                match o {
                    scrollbar::Orientation::Vertical => Size::new(l.clone(), *r),
                    scrollbar::Orientation::Horizontal => Size::new(*r, l.clone()),
                }
            }),
        )
    });

    wgt.push_intrinsic(NestGroup::LAYOUT, "thumb_layout", thumb_layout);

    let viewport_ratio = wgt.capture_var_or_else(property_id!(viewport_ratio), || 1.fct());
    let offset = wgt.capture_var_or_else(property_id!(offset), || 0.fct());

    wgt.push_intrinsic(NestGroup::CONTEXT, "thumb-context", move |child| {
        let child = with_context_var(child, THUMB_VIEWPORT_RATIO_VAR, viewport_ratio);
        with_context_var(child, THUMB_OFFSET_VAR, offset)
    });
}

fn thumb_layout(child: impl UiNode) -> impl UiNode {
    let mut content_length = Px(0);
    let mut viewport_length = Px(0);
    let mut thumb_length = Px(0);
    let mut scale_factor = 1.fct();

    let mut mouse_down = None::<(Px, Factor)>;

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_event(&MOUSE_MOVE_EVENT)
                .sub_event(&MOUSE_INPUT_EVENT)
                .sub_var_layout(&THUMB_OFFSET_VAR);
        }
        UiNodeOp::Event { update } => {
            if let Some((md, start_offset)) = mouse_down {
                if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                    let bounds = WIDGET.bounds().inner_bounds();
                    let (mut offset, cancel_offset, bounds_min, bounds_max) = match ORIENTATION_VAR.get() {
                        scrollbar::Orientation::Vertical => (
                            args.position.y.to_px(scale_factor),
                            args.position.x.to_px(scale_factor),
                            bounds.min_x(),
                            bounds.max_x(),
                        ),
                        scrollbar::Orientation::Horizontal => (
                            args.position.x.to_px(scale_factor),
                            args.position.y.to_px(scale_factor),
                            bounds.min_y(),
                            bounds.max_y(),
                        ),
                    };

                    let cancel_margin = Dip::new(40).to_px(scale_factor);
                    let offset = if cancel_offset < bounds_min - cancel_margin || cancel_offset > bounds_max + cancel_margin {
                        // pointer moved outside of the thumb + 40, snap back to initial
                        start_offset
                    } else {
                        offset -= md;

                        let max_length = viewport_length - thumb_length;
                        let start_offset = max_length * start_offset.0;

                        let offset = offset + start_offset;
                        let offset = (offset.0 as f32 / max_length.0 as f32).clamp(0.0, 1.0);

                        // snap to pixel
                        let max_length = viewport_length - content_length;
                        let offset = max_length * offset;
                        let offset = offset.0 as f32 / max_length.0 as f32;
                        offset.fct()
                    };

                    THUMB_OFFSET_VAR.set(offset);
                    WIDGET.layout();

                    args.propagation().stop();
                } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    if args.is_primary() && args.is_mouse_up() {
                        mouse_down = None;

                        args.propagation().stop();
                    }
                }
            } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                if args.is_primary() && args.is_mouse_down() {
                    let a = match ORIENTATION_VAR.get() {
                        scrollbar::Orientation::Vertical => args.position.y.to_px(scale_factor),
                        scrollbar::Orientation::Horizontal => args.position.x.to_px(scale_factor),
                    };
                    mouse_down = Some((a, THUMB_OFFSET_VAR.get()));

                    args.propagation().stop();
                }
            }
        }
        UiNodeOp::Layout { wl, .. } => {
            let bar_size = LAYOUT.constraints().fill_size();
            let mut final_offset = PxVector::zero();
            let (bar_length, final_d) = match ORIENTATION_VAR.get() {
                scrollbar::Orientation::Vertical => (bar_size.height, &mut final_offset.y),
                scrollbar::Orientation::Horizontal => (bar_size.width, &mut final_offset.x),
            };

            let ratio = THUMB_VIEWPORT_RATIO_VAR.get();
            let tl = bar_length * ratio;
            *final_d = (bar_length - tl) * THUMB_OFFSET_VAR.get();

            scale_factor = LAYOUT.scale_factor();
            content_length = bar_length / ratio;
            viewport_length = bar_length;
            thumb_length = tl;

            wl.translate(final_offset);
        }
        _ => {}
    })
}

context_var! {
    static THUMB_VIEWPORT_RATIO_VAR: Factor = 1.fct();
    static THUMB_OFFSET_VAR: Factor = 0.fct();
}
