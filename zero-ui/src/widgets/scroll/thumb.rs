//! Thumb widget, properties and nodes..

use super::*;
use crate::core::mouse::*;
use scrollbar::ORIENTATION_VAR;

/// Scrollbar thumb widget.
#[widget($crate::widgets::scroll::Thumb)]
pub struct Thumb(WidgetBase);
impl Thumb {
    #[widget(on_start)]
    fn on_start(&mut self) {
        widget_dft! {
            self;
            crate::properties::background_color = rgba(200, 200, 200, 50.pct());
            capture_mouse = true;
            crate::properties::click_mode = ClickMode::Default; // scrollbar sets to repeat

            when *#is_hovered {
                background_color = rgba(200, 200, 200, 70.pct());
            }

            when *#is_cap_pressed  {
                background_color = rgba(200, 200, 200, 90.pct());
            }
        }

        self.builder().push_build_action(on_build);
    }
}

/// Viewport/content ratio.
///
/// This becomes the height for vertical and width for horizontal.
#[property(LAYOUT, capture, impl(Thumb))]
pub fn viewport_ratio(child: impl UiNode, ratio: impl IntoVar<Factor>) -> impl UiNode {}

/// Content offset.
#[property(LAYOUT, capture, impl(Thumb))]
pub fn offset(child: impl UiNode, offset: impl IntoVar<Factor>) -> impl UiNode {}

/// Width if orientation is vertical, otherwise height if orientation is horizontal.
#[property(SIZE, capture, default(16), impl(Thumb))]
pub fn cross_length(child: impl UiNode, length: impl IntoVar<Length>) -> impl UiNode {}

fn on_build(wgt: &mut WidgetBuilding) {
    let cross_length = wgt.capture_var_or_default::<Length>(property_id!(cross_length));
    wgt.push_intrinsic(NestGroup::SIZE, "orientation-size", move |child| {
        size(
            child,
            merge_var!(ORIENTATION_VAR, THUMB_VIEWPORT_RATIO_VAR, cross_length, |o, r, l| {
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
    #[ui_node(struct DragNode {
            child: impl UiNode,
            content_length: Px,
            viewport_length: Px,
            thumb_length: Px,
            scale_factor: Factor,

            mouse_down: Option<(Px, Factor)>,
        })]
    impl UiNode for DragNode {
        fn init(&mut self) {
            WIDGET
                .sub_event(&MOUSE_MOVE_EVENT)
                .sub_event(&MOUSE_INPUT_EVENT)
                .sub_var(&THUMB_OFFSET_VAR);
            self.child.init();
        }

        fn event(&mut self, update: &EventUpdate) {
            if let Some((mouse_down, start_offset)) = self.mouse_down {
                if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                    let bounds = WIDGET.bounds().inner_bounds();
                    let (mut offset, cancel_offset, bounds_min, bounds_max) = match ORIENTATION_VAR.get() {
                        scrollbar::Orientation::Vertical => (
                            args.position.y.to_px(self.scale_factor.0),
                            args.position.x.to_px(self.scale_factor.0),
                            bounds.min_x(),
                            bounds.max_x(),
                        ),
                        scrollbar::Orientation::Horizontal => (
                            args.position.x.to_px(self.scale_factor.0),
                            args.position.y.to_px(self.scale_factor.0),
                            bounds.min_y(),
                            bounds.max_y(),
                        ),
                    };

                    let cancel_margin = Dip::new(40).to_px(self.scale_factor.0);
                    let offset = if cancel_offset < bounds_min - cancel_margin || cancel_offset > bounds_max + cancel_margin {
                        // pointer moved outside of the thumb + 40, snap back to initial
                        start_offset
                    } else {
                        offset -= mouse_down;

                        let max_length = self.viewport_length - self.thumb_length;
                        let start_offset = max_length * start_offset.0;

                        let offset = offset + start_offset;
                        let offset = (offset.0 as f32 / max_length.0 as f32).clamp(0.0, 1.0);

                        // snap to pixel
                        let max_length = self.viewport_length - self.content_length;
                        let offset = max_length * offset;
                        let offset = offset.0 as f32 / max_length.0 as f32;
                        offset.fct()
                    };

                    THUMB_OFFSET_VAR.set_ne(offset).expect("THUMB_OFFSET_VAR is read-only");
                    WIDGET.layout();

                    args.propagation().stop();
                } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    if args.is_primary() && args.is_mouse_up() {
                        self.mouse_down = None;

                        args.propagation().stop();
                    }
                }
            } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                if args.is_primary() && args.is_mouse_down() {
                    let a = match ORIENTATION_VAR.get() {
                        scrollbar::Orientation::Vertical => args.position.y.to_px(self.scale_factor.0),
                        scrollbar::Orientation::Horizontal => args.position.x.to_px(self.scale_factor.0),
                    };
                    self.mouse_down = Some((a, THUMB_OFFSET_VAR.get()));

                    args.propagation().stop();
                }
            }
            self.child.event(update);
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if THUMB_OFFSET_VAR.is_new() {
                WIDGET.layout();
            }

            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.child.measure(wm)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let bar_size = LAYOUT.constraints().fill_size();
            let mut final_offset = PxVector::zero();
            let (bar_length, final_d) = match ORIENTATION_VAR.get() {
                scrollbar::Orientation::Vertical => (bar_size.height, &mut final_offset.y),
                scrollbar::Orientation::Horizontal => (bar_size.width, &mut final_offset.x),
            };

            let ratio = THUMB_VIEWPORT_RATIO_VAR.get();
            let thumb_length = bar_length * ratio;
            *final_d = (bar_length - thumb_length) * THUMB_OFFSET_VAR.get();

            self.scale_factor = LAYOUT.scale_factor();
            self.content_length = bar_length / ratio;
            self.viewport_length = bar_length;
            self.thumb_length = thumb_length;

            wl.translate(final_offset);

            self.child.layout(wl)
        }
    }
    DragNode {
        child,
        content_length: Px(0),
        viewport_length: Px(0),
        thumb_length: Px(0),
        scale_factor: 1.fct(),

        mouse_down: None,
    }
}

context_var! {
    static THUMB_VIEWPORT_RATIO_VAR: Factor = 1.fct();
    static THUMB_OFFSET_VAR: Factor = 0.fct();
}
