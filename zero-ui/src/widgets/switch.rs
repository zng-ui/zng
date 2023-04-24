//! Switch widget, properties and nodes..

use std::mem;

use crate::prelude::new_widget::*;

/// Switch visibility of children nodes using an index variable.
///
/// All option nodes are children of the widget, but only the indexed child is layout and rendered.
///
/// If the index is out of range all children, and the widget, are collapsed.
#[widget($crate::widgets::Switch)]
pub struct Switch(WidgetBase);

impl Switch {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let index = wgt.capture_var_or_else(property_id!(Self::index), || 0);
            let options = wgt.capture_ui_node_list_or_empty(property_id!(Self::options));
            let child = switch_node(index, options);
            wgt.set_child(child);
        });
    }
}

/// Index of the active child.
#[property(CHILD, capture, widget_impl(Switch))]
pub fn index(child: impl UiNode, index: impl IntoVar<usize>) -> impl UiNode {}

/// List of nodes that can be switched too.
#[property(CHILD, capture, widget_impl(Switch))]
pub fn options(child: impl UiNode, options: impl UiNodeList) -> impl UiNode {}

/// Switch node.
///
/// This is the raw [`UiNode`] that implements the core [`Switch`] functionality
/// without defining a full widget.
///
/// [`Switch`]: struct@Switch
pub fn switch_node(index: impl Var<usize>, options: impl UiNodeList) -> impl UiNode {
    let mut collapse = true;
    match_node_list(options, move |options, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&index);
        }
        UiNodeOp::Update { updates } => {
            if index.is_new() {
                WIDGET.layout().render();
                collapse = true;

                options.update_all(updates, &mut ());
            } else {
                struct TouchedIndex {
                    index: usize,
                    touched: bool,
                }
                impl UiNodeListObserver for TouchedIndex {
                    fn is_reset_only(&self) -> bool {
                        false
                    }
                    fn reset(&mut self) {
                        self.touched = true;
                    }
                    fn inserted(&mut self, index: usize) {
                        self.touched |= self.index == index;
                    }
                    fn removed(&mut self, index: usize) {
                        self.touched |= self.index == index;
                    }
                    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
                        self.touched |= self.index == removed_index || self.index == inserted_index;
                    }
                }
                let mut check = TouchedIndex {
                    index: index.get(),
                    touched: false,
                };
                options.update_all(updates, &mut check);

                if check.touched {
                    WIDGET.layout().render();
                    collapse = true;
                }
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            options.delegated();

            let index = index.get();
            if index < options.len() {
                *desired_size = options.with_node(index, |n| n.measure(wm));
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            options.delegated();

            if mem::take(&mut collapse) {
                wl.collapse_descendants();
            }

            let index = index.get();
            if index < options.len() {
                *final_size = options.with_node(index, |n| n.layout(wl));
            }
        }
        UiNodeOp::Render { frame } => {
            options.delegated();

            let index = index.get();
            if index < options.len() {
                options.with_node(index, |n| n.render(frame))
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            options.delegated();

            let index = index.get();
            if index < options.len() {
                options.with_node(index, |n| n.render_update(update));
            }
        }
        _ => {}
    })
}
