use crate::prelude::new_widget::*;

use std::{cell::Cell, mem};

/// Switch visibility of children nodes using an index variable.
///
/// This is a shorthand call to [`switch!`](mod@switch).
pub fn switch<I: Var<usize>, W: UiNodeList>(index: I, options: W) -> impl UiNode {
    switch!(index; options)
}

/// Switch visibility of children nodes using an index variable.
///
/// All option nodes are children of the widget, but only the indexed child is layout and rendered.
///
/// If the index is out of range all children, and the widget, are collapsed.
#[widget($crate::widgets::switch)]
pub mod switch {
    use super::*;

    struct SwitchNode<I, W> {
        index: I,
        options: W,
        collapse: bool,
        render_collapse_once: Cell<bool>,
    }
    #[ui_node(
        delegate_list = &self.options,
        delegate_list_mut = &mut self.options,
    )]
    impl<I: Var<usize>, W: UiNodeList> UiNode for SwitchNode<I, W> {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.index.is_new(ctx) {
                ctx.updates.layout_and_render();
                self.collapse = true;

                self.options.update_all(ctx, updates, &mut ());
            } else if self.options.is_fixed() {
                self.options.update_all(ctx, updates, &mut ());
            } else {
                struct TouchedIndex {
                    index: usize,
                    touched: bool,
                }
                impl UiListObserver for TouchedIndex {
                    fn reseted(&mut self) {
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
                self.options.update_all(ctx, updates, &mut check);

                if check.touched {
                    ctx.updates.layout_and_render();
                    self.collapse = true;
                }
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let index = self.index.get();
            if index < self.options.len() {
                self.options.item_measure(index, ctx)
            } else {
                PxSize::zero()
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            if mem::take(&mut self.collapse) {
                wl.collapse_descendants(ctx);
                *self.render_collapse_once.get_mut() = true;
            }

            let index = self.index.get();
            if index < self.options.len() {
                self.options.item_layout(index, ctx, wl)
            } else {
                PxSize::zero()
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if self.render_collapse_once.take() {
                frame.collapse_descendants(ctx.info_tree);
            }
            let index = self.index.get();
            if index < self.options.len() {
                self.options.item_render(index, ctx, frame)
            }
        }
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            let index = self.index.get();
            if index < self.options.len() {
                self.options.item_render_update(index, ctx, update)
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
            render_collapse_once: Cell::new(true),
        }
        .cfg_boxed()
    }

    properties! {
        /// Index of the active child.
        index(impl Var<usize>);

        /// List of nodes that can be switched too.
        #[allowed_in_when = false]
        options(impl UiNodeList);
    }

    fn new_child(index: impl Var<usize>, options: impl UiNodeList) -> impl UiNode {
        self::new_node(index, options)
    }
}
