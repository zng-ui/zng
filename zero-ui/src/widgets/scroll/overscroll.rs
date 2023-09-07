//! Over-scroll visual indicator.

use crate::prelude::new_widget::*;

use super::{OVERSCROLL_HORIZONTAL_OFFSET_VAR, OVERSCROLL_VERTICAL_OFFSET_VAR};

/// Visual indicator when a touch scroll attempts to scroll past the limit.
#[widget($crate::widgets::scroll::overscroll::OverScroll)]
pub struct OverScroll(WidgetBase);
impl OverScroll {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            wgt.set_child(over_scroll_node());
        });

        widget_set! {
            self;
            interactive = false;
        }
    }
}

pub fn over_scroll_node() -> impl UiNode {
    let mut v_rect = PxRect::zero();
    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&OVERSCROLL_VERTICAL_OFFSET_VAR)
                .sub_var_layout(&OVERSCROLL_HORIZONTAL_OFFSET_VAR);
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();

            let mut new_v_rect = PxRect::zero();

            let v = OVERSCROLL_VERTICAL_OFFSET_VAR.get();
            if dbg!(v) < 0.fct() {
                new_v_rect.size = *final_size;
                new_v_rect.size.height *= v.abs().min(0.1.fct());
            } else if v > 0.fct() {
                new_v_rect.size = *final_size;
                new_v_rect.size.height *= v.abs().min(0.1.fct());
                new_v_rect.origin.y = final_size.height - v_rect.size.height;
            }

            if new_v_rect != v_rect {
                v_rect = new_v_rect;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            if !v_rect.size.is_empty() {
                frame.push_color(v_rect, FrameValue::Value(colors::RED.into()));
            }
        }
        _ => {}
    })
}
