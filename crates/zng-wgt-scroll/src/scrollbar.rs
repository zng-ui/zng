//! Scrollbar widget, properties and nodes..

use zng_ext_input::mouse::{ClickMode, MouseClickArgs};
use zng_ext_window::WINDOW_Ext as _;
use zng_wgt::{align, prelude::*};
use zng_wgt_access::{AccessRole, access_role};
use zng_wgt_fill::background_color;
use zng_wgt_input::{click_mode, mouse::on_mouse_click};

/// Scrollbar widget.
#[widget($crate::Scrollbar)]
pub struct Scrollbar(WidgetBase);
impl Scrollbar {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            background_color = vis::BACKGROUND_VAR;
            click_mode = ClickMode::repeat();
            on_mouse_click = scroll_click_handler();
            access_role = AccessRole::ScrollBar;
        }

        self.widget_builder().push_build_action(|wgt| {
            // scrollbar is larger than thumb, align inserts the extra space.
            let thumb = wgt.capture_ui_node_or_else(property_id!(Self::thumb), || super::Thumb!());
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
                let child = access_node(child);
                with_context_var(child, ORIENTATION_VAR, orientation)
            });
        });
    }
}

/// Thumb widget.
///
/// Recommended widget is [`Thumb!`], but can be any widget that implements
/// thumb behavior and tags itself in the frame.
///
/// [`Thumb!`]: struct@super::Thumb
#[property(CHILD, capture, default(super::Thumb!()), widget_impl(Scrollbar))]
pub fn thumb(node: impl UiNode) {}

/// Scrollbar orientation.
///
/// This sets the scrollbar alignment to fill its axis and take the cross-length from the thumb.
#[property(CONTEXT, capture, default(Orientation::Vertical), widget_impl(Scrollbar))]
pub fn orientation(orientation: impl IntoVar<Orientation>) {}

context_var! {
    pub(super) static ORIENTATION_VAR: Orientation = Orientation::Vertical;
}

/// Context scrollbar info.
pub struct SCROLLBAR;
impl SCROLLBAR {
    /// Gets the context scrollbar orientation.
    pub fn orientation(&self) -> Var<Orientation> {
        ORIENTATION_VAR.read_only()
    }
}

/// Style variables and properties.
pub mod vis {
    use super::*;

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

fn scroll_click_handler() -> impl WidgetHandler<MouseClickArgs> {
    use std::cmp::Ordering;

    let mut ongoing_direction = Ordering::Equal;
    hn!(|args: &MouseClickArgs| {
        use crate::*;

        let orientation = ORIENTATION_VAR.get();
        let bounds = WIDGET.bounds().inner_bounds();
        let scale_factor = WINDOW.vars().scale_factor().get();
        let position = args.position.to_px(scale_factor) - bounds.origin;

        let (offset, mid_pt, mid_offset) = match orientation {
            Orientation::Vertical => (
                bounds.origin.y + bounds.size.height * SCROLL_VERTICAL_OFFSET_VAR.get(),
                position.y,
                position.y.0 as f32 / bounds.size.height.0 as f32,
            ),
            Orientation::Horizontal => (
                bounds.origin.x + bounds.size.width * SCROLL_HORIZONTAL_OFFSET_VAR.get(),
                position.x,
                position.x.0 as f32 / bounds.size.width.0 as f32,
            ),
        };

        let direction = mid_pt.cmp(&offset);

        // don't overshoot the pointer.
        let clamp = match direction {
            Ordering::Less => (mid_offset, 1.0),
            Ordering::Greater => (0.0, mid_offset),
            Ordering::Equal => (0.0, 0.0),
        };
        let request = cmd::ScrollRequest {
            clamp,
            ..Default::default()
        };

        if args.click_count.get() == 1 {
            ongoing_direction = direction;
        }
        if ongoing_direction == direction {
            match orientation {
                Orientation::Vertical => match direction {
                    Ordering::Less => cmd::PAGE_UP_CMD.scoped(SCROLL.id()).notify_param(request),
                    Ordering::Greater => cmd::PAGE_DOWN_CMD.scoped(SCROLL.id()).notify_param(request),
                    Ordering::Equal => {}
                },
                Orientation::Horizontal => match direction {
                    Ordering::Less => cmd::PAGE_LEFT_CMD.scoped(SCROLL.id()).notify_param(request),
                    Ordering::Greater => cmd::PAGE_RIGHT_CMD.scoped(SCROLL.id()).notify_param(request),
                    Ordering::Equal => {}
                },
            }
        }

        args.propagation().stop();
    })
}

fn access_node(child: impl UiNode) -> impl UiNode {
    let mut handle = VarHandle::dummy();
    match_node(child, move |_, op| {
        if let UiNodeOp::Info { info } = op {
            if let Some(mut info) = info.access() {
                use crate::*;

                if handle.is_dummy() {
                    handle = ORIENTATION_VAR.subscribe(UpdateOp::Info, WIDGET.id());
                }

                match ORIENTATION_VAR.get() {
                    Orientation::Horizontal => info.set_scroll_horizontal(SCROLL_HORIZONTAL_OFFSET_VAR.current_context()),
                    Orientation::Vertical => info.set_scroll_vertical(SCROLL_VERTICAL_OFFSET_VAR.current_context()),
                }

                info.push_controls(SCROLL.id());
            }
        }
    })
}
