//! Scrollbar widget, properties and nodes..

use crate::core::mouse::{ClickMode, MouseClickArgs};

use crate::prelude::new_widget::*;

/// Scrollbar widget.
#[widget($crate::widgets::scroll::Scrollbar)]
pub struct Scrollbar(WidgetBase);
impl Scrollbar {
    #[widget(on_start)]
    fn on_start(&mut self) {
        defaults! {
            self;
            crate::properties::background_color = vis::BACKGROUND_VAR;
            crate::properties::click_mode = ClickMode::Repeat;
            crate::properties::events::mouse::on_mouse_click = {
                use std::cmp::Ordering;

                let mut ongoing_direction = Ordering::Equal;
                hn!(|args: &MouseClickArgs| {
                    use crate::widgets::scroll::*;
                    use crate::core::window::WINDOW_CTRL;

                    let orientation = ORIENTATION_VAR.get();
                    let bounds = WIDGET.bounds().inner_bounds();
                    let scale_factor = WINDOW_CTRL.vars().scale_factor().get();
                    let position = args.position.to_px(scale_factor.0);

                    let (offset, mid_pt, mid_offset) = match orientation {
                        Orientation::Vertical => (
                            bounds.origin.y + bounds.size.height * SCROLL_VERTICAL_OFFSET_VAR.get(),
                            position.y,
                            position.y.0 as f32 / bounds.size.height.0 as f32,
                        ),
                        Orientation::Horizontal => (
                            bounds.origin.x + bounds.size.width * SCROLL_HORIZONTAL_OFFSET_VAR.get(),
                            position.x,
                            position.x.0 as f32 /bounds.size.width.0 as f32,
                        )
                    };

                    let direction = mid_pt.cmp(&offset);

                    // don't overshoot the pointer.
                    let clamp = match direction {
                        Ordering::Less => (mid_offset, 1.0),
                        Ordering::Greater => (0.0, mid_offset),
                        Ordering::Equal => (0.0, 0.0),
                    };
                    let request = commands::ScrollRequest {
                        clamp,
                        ..Default::default()
                    };

                    if args.click_count.get() == 1 {
                        ongoing_direction = direction;
                    }
                    if ongoing_direction == direction {
                        match orientation {
                            Orientation::Vertical => {
                                match direction {
                                    Ordering::Less => commands::PAGE_UP_CMD.scoped(SCROLL.id()).notify_param(request),
                                    Ordering::Greater => commands::PAGE_DOWN_CMD.scoped(SCROLL.id()).notify_param(request),
                                    Ordering::Equal => {},
                                }
                            },
                            Orientation::Horizontal => {
                                match direction {
                                    Ordering::Less => commands::PAGE_LEFT_CMD.scoped(SCROLL.id()).notify_param(request),
                                    Ordering::Greater => commands::PAGE_RIGHT_CMD.scoped(SCROLL.id()).notify_param(request),
                                    Ordering::Equal => {},
                                }
                            }
                        }


                    }

                    args.propagation().stop();
                })
            };
        }

        self.builder().push_build_action(|wgt| {
            // scrollbar is larger than thumb, align inserts the extra space.
            let thumb = wgt.capture_ui_node_or_else(property_id!(Self::thumb_node), || NilUiNode);
            let thumb = align(thumb, Align::FILL);
            wgt.set_child(thumb);

            wgt.push_intrinsic(NestGroup::LAYOUT, "orientation-align", move |child| {
                align(
                    child,
                    ORIENTATION_VAR.map(|o| match o {
                        Orientation::Vertical => Align::FILL_RIGHT,
                        Orientation::Horizontal => Align::FILL_BOTTOM,
                    }),
                )
            });

            let orientation = wgt.capture_var_or_else(property_id!(Self::orientation), || Orientation::Vertical);
            wgt.push_intrinsic(NestGroup::CONTEXT, "scrollbar-context", move |child| {
                with_context_var(child, ORIENTATION_VAR, orientation)
            });
        });
    }
}

/// Thumb widget.
///
/// Recommended widget is [`Thumb!`], but can be any widget that implements
/// thumb behavior and tags it-self in the frame.
///
/// [`Thumb!`]: struct@thumb
#[property(CHILD, capture, default(super::Thumb!()), impl(Scrollbar))]
pub fn thumb_node(child: impl UiNode, node: impl UiNode) -> impl UiNode {}

/// Scrollbar orientation.
///
/// This sets the scrollbar alignment to fill its axis and take the cross-length from the thumb.
#[property(CONTEXT, capture, default(Orientation::Vertical), impl(Scrollbar))]
pub fn orientation(child: impl UiNode, orientation: impl IntoVar<Orientation>) -> impl UiNode {}

context_var! {
    pub(super) static ORIENTATION_VAR: Orientation = Orientation::Vertical;
}

/// Context scrollbar info.
pub struct SCROLLBAR;
impl SCROLLBAR {
    /// Gets the context scrollbar orientation.
    pub fn orientation(&self) -> BoxedVar<Orientation> {
        ORIENTATION_VAR.read_only().boxed()
    }
}

/// Style variables and properties.
pub mod vis {
    use crate::prelude::new_property::*;

    context_var! {
        /// Scrollbar track background color
        pub static BACKGROUND_VAR: Rgba = rgba(80, 80, 80, 50.pct());
    }
}

/// Orientation of a scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Bar fills the in the ***x*** dimension and scrolls left-right.
    Horizontal,
    /// Bar fills the in the ***y*** dimension and scrolls top-bottom.
    Vertical,
}
