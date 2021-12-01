use crate::prelude::new_widget::*;

/// Switch between children nodes using an index variable.
///
/// This is a shorthand call to [`switch!`](mod@switch).
pub fn switch<I: Var<usize>, W: UiNodeList>(index: I, options: W) -> impl Widget {
    switch!(index; options)
}

/// Switch between children nodes using an index variable.
///
/// All children nodes are kept up-to-date, but only the indexed child is in the widget info, layout and render.
///
/// If the index is out of range no node is rendered and this widget takes no space.
#[widget($crate::widgets::switch)]
pub mod switch {
    use super::*;

    struct SwitchNode<I, W> {
        index: I,
        options: W,
    }
    #[impl_ui_node(
        delegate_list = &self.options,
        delegate_list_mut = &mut self.options,
    )]
    impl<I: Var<usize>, W: UiNodeList> UiNode for SwitchNode<I, W> {
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.options.update_all(ctx);
            if self.index.is_new(ctx) {
                ctx.updates.info_layout_and_render();
            }
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            let index = self.index.copy(ctx);
            if index < self.options.len() {
                self.options.widget_info(index, ctx, info);
            }
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
            let index = self.index.copy(ctx);
            if index < self.options.len() {
                self.options.widget_measure(index, ctx, available_size)
            } else {
                PxSize::zero()
            }
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, widget_offset: &mut WidgetOffset, final_size: PxSize) {
            let index = self.index.copy(ctx);
            if index < self.options.len() {
                self.options.widget_arrange(index, ctx, widget_offset, final_size)
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let index = self.index.copy(ctx);
            if index < self.options.len() {
                self.options.widget_render(index, ctx, frame)
            }
        }
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            let index = self.index.copy(ctx);
            if index < self.options.len() {
                self.options.widget_render_update(index, ctx, update)
            }
        }
    }

    /// New switch node.
    ///
    /// This is the raw [`UiNode`] that implements the core `switch` functionality
    /// without defining a full widget.
    pub fn new_node(index: impl Var<usize>, options: impl UiNodeList) -> impl UiNode {
        SwitchNode { index, options }
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
