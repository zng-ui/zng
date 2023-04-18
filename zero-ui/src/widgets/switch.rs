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
            let child = self::new_node(index, options);
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

struct SwitchNode<I, W> {
    index: I,
    options: W,
    collapse: bool,
}
#[ui_node(delegate_list = &mut self.options)]
impl<I: Var<usize>, W: UiNodeList> UiNode for SwitchNode<I, W> {
    fn update(&mut self, updates: &WidgetUpdates) {
        if self.index.is_new() {
            WIDGET.layout().render();
            self.collapse = true;

            self.options.update_all(updates, &mut ());
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
                index: self.index.get(),
                touched: false,
            };
            self.options.update_all(updates, &mut check);

            if check.touched {
                WIDGET.layout().render();
                self.collapse = true;
            }
        }
    }

    fn measure(&mut self, wm: &mut WidgetMeasure) -> PxSize {
        let index = self.index.get();
        if index < self.options.len() {
            self.options.with_node(index, |n| n.measure(wm))
        } else {
            PxSize::zero()
        }
    }
    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if mem::take(&mut self.collapse) {
            wl.collapse_descendants();
        }

        let index = self.index.get();
        if index < self.options.len() {
            self.options.with_node(index, |n| n.layout(wl))
        } else {
            PxSize::zero()
        }
    }

    fn render(&mut self, frame: &mut FrameBuilder) {
        let index = self.index.get();
        if index < self.options.len() {
            self.options.with_node(index, |n| n.render(frame))
        }
    }
    fn render_update(&mut self, update: &mut FrameUpdate) {
        let index = self.index.get();
        if index < self.options.len() {
            self.options.with_node(index, |n| n.render_update(update));
        }
    }
}

/// New switch node.
///
/// This is the raw [`UiNode`] that implements the core `switch` functionality
/// without defining a full widget.
pub fn new_node(index: impl Var<usize>, options: impl UiNodeList) -> impl UiNode {
    SwitchNode {
        index,
        options,
        collapse: true,
    }
    .cfg_boxed()
}
