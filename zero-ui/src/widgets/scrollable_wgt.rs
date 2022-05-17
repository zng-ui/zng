use crate::prelude::new_widget::*;

pub mod commands;
pub mod nodes;
pub mod properties;

mod parts;
mod types;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::scrollable)]
pub mod scrollable {
    use super::*;
    use properties::*;

    #[doc(inline)]
    pub use super::{
        commands, nodes,
        parts::{scrollbar, thumb},
        properties,
        types::*,
    };

    properties! {
        /// Content UI.
        ///
        /// Can be any type that implements [`UiNode`](zero_ui::core::UiNode), any widget.
        #[allowed_in_when = false]
        #[required]
        content(impl UiNode);

        /// Spacing around content, inside the scroll area.
        padding;

        /// Content alignment when it is smaller then the viewport.
        child_align as content_align = Align::CENTER;

        /// Scroll mode.
        ///
        /// By default scrolls in both dimensions.
        mode(impl IntoVar<ScrollMode>) = ScrollMode::ALL;

        /// Scrollbar widget generator for both orientations.
        ///
        /// This property sets both [`v_scrollbar_view`] and [`h_scrollbar_view`] to the same `generator`.
        ///
        /// [`v_scrollbar_view`]: #wp-v_scrollbar_view
        /// [`h_scrollbar_view`]: #wp-h_scrollbar_view
        scrollbar_view;

        /// Horizontal scrollbar widget generator.
        h_scrollbar_view;
        /// Vertical scrollbar widget generator.
        v_scrollbar_view;

        /// Horizontal and vertical offsets used when scrolling.
        ///
        /// This property sets the [`h_line_unit`] and [`v_line_unit`].
        ///
        /// [`h_line_unit`]: #wp-h_line_unit
        /// [`v_line_unit`]: #wp-v_line_unit
        line_units;
        h_line_unit;
        v_line_unit;

        /// Horizontal and vertical offsets used when page-scrolling.
        ///
        /// This property sets the [`h_page_unit`] and [`v_page_unit`].
        ///
        /// [`h_page_unit`]: fn@h_page_unit
        /// [`v_page_unit`]: fn@v_page_unit
        page_units;
        h_page_unit;
        v_page_unit;

        /// Scroll unit multiplier used when alternate scrolling.
        ///
        /// This value is used, for example, when `ALT` is pressed during an scroll-wheel event,
        alt_factor;

        /// Clip content to only be visible within the scrollable bounds, including under scrollbars.
        ///
        /// Enabled by default.
        clip_to_bounds = true;

        /// Clip content to only be visible within the viewport, not under scrollbars.
        ///
        /// Disabled by default.
        clip_to_viewport(impl IntoVar<bool>) = false;

        /// Enables keyboard controls.
        focusable = true;

        /// Smooth scrolling configuration.
        smooth_scrolling;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }

    fn new_child_context(child: impl UiNode, mode: impl IntoVar<ScrollMode>, clip_to_viewport: impl IntoVar<bool>) -> impl UiNode {
        struct ScrollableNode<N> {
            children: N,
            viewport: PxSize,
            joiner: PxSize,
            spatial_id: SpatialFrameId,
        }
        #[impl_ui_node(children)]
        impl<N: UiNodeList> UiNode for ScrollableNode<N> {
            // # Layout
            //
            // +-----------------+---+
            // |                 |   |
            // | 0 - viewport    | 1 | - v_scrollbar
            // |                 |   |
            // +-----------------+---+
            // | 2 - h_scrollbar | 3 | - scrollbar_joiner
            ///+-----------------+---+
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                // measure
                let v_scroll = self.children.widget_layout(1, ctx, wl);
                let h_scroll = self.children.widget_layout(2, ctx, wl);

                self.joiner = PxSize::new(v_scroll.width, h_scroll.height);
                let mut viewport = ctx.with_constrains(|c| c.with_less_size(self.joiner), |ctx| self.children.widget_layout(0, ctx, wl));

                let _ = ctx.with_constrains(|c| c.with_max(self.joiner), |ctx| self.children.widget_layout(3, ctx, wl));

                // arrange
                let final_size = viewport + self.joiner;
                let content_size = ScrollContentSizeVar::get_clone(ctx);

                if content_size.height > final_size.height {
                    ScrollVerticalContentOverflowsVar::new().set_ne(ctx, true).unwrap();
                    ScrollHorizontalContentOverflowsVar::new()
                        .set_ne(ctx, content_size.width > viewport.width)
                        .unwrap();
                } else if content_size.width > final_size.width {
                    ScrollHorizontalContentOverflowsVar::new().set_ne(ctx, true).unwrap();
                    ScrollVerticalContentOverflowsVar::new()
                        .set_ne(ctx, content_size.height > viewport.height)
                        .unwrap();
                } else {
                    ScrollVerticalContentOverflowsVar::new().set_ne(ctx, false).unwrap();
                    ScrollHorizontalContentOverflowsVar::new().set_ne(ctx, false).unwrap();
                }

                // collapse scrollbars if they take more the 1/3 of the total area.
                if viewport.width < self.joiner.width * 3.0.fct() {
                    viewport.width += self.joiner.width;
                    self.joiner.width = Px(0);
                }
                if viewport.height < self.joiner.height * 3.0.fct() {
                    viewport.height += self.joiner.height;
                    self.joiner.height = Px(0);
                }

                if viewport != self.viewport {
                    self.viewport = viewport;
                    ctx.updates.render();
                }

                self.viewport + self.joiner
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                self.children.widget_render(0, ctx, frame);

                if self.joiner.width > Px(0) {
                    let transform = RenderTransform::translation_px(PxVector::new(self.viewport.width, Px(0)));
                    frame.push_reference_frame_item(self.spatial_id, 1, FrameBinding::Value(transform), true, |frame| {
                        self.children.widget_render(1, ctx, frame);
                    });
                }

                if self.joiner.height > Px(0) {
                    let transform = RenderTransform::translation_px(PxVector::new(Px(0), self.viewport.height));
                    frame.push_reference_frame_item(self.spatial_id, 2, FrameBinding::Value(transform), true, |frame| {
                        self.children.widget_render(2, ctx, frame);
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    let transform = RenderTransform::translation_px(self.viewport.to_vector());
                    frame.push_reference_frame_item(self.spatial_id, 3, FrameBinding::Value(transform), true, |frame| {
                        self.children.widget_render(3, ctx, frame);
                    });
                }
            }
        }

        use crate::core::context::UpdatesTraceUiNodeExt;
        ScrollableNode {
            children: nodes![
                clip_to_bounds(
                    nodes::viewport(child, mode.into_var()).instrument("viewport"),
                    clip_to_viewport.into_var()
                ),
                nodes::v_scrollbar_presenter(),
                nodes::h_scrollbar_presenter(),
                nodes::scrollbar_joiner_presenter(),
            ],
            viewport: PxSize::zero(),
            joiner: PxSize::zero(),
            spatial_id: SpatialFrameId::new_unique(),
        }
    }

    fn new_event(child: impl UiNode) -> impl UiNode {
        let child = nodes::scroll_to_command_node(child);
        let child = nodes::scroll_commands_node(child);
        let child = nodes::page_commands_node(child);
        let child = nodes::scroll_to_edge_commands_node(child);
        nodes::scroll_wheel_node(child)
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        let child = with_context_var(child, ScrollViewportSizeVar, var(PxSize::zero()));
        let child = with_context_var(child, ScrollContentSizeVar, var(PxSize::zero()));

        let child = with_context_var(child, ScrollVerticalRatioVar, var(0.fct()));
        let child = with_context_var(child, ScrollHorizontalRatioVar, var(0.fct()));

        let child = with_context_var(child, ScrollVerticalContentOverflowsVar, var(false));
        let child = with_context_var(child, ScrollHorizontalContentOverflowsVar, var(false));

        let child = with_context_var(child, ScrollVerticalOffsetVar, var(0.fct()));
        with_context_var(child, ScrollHorizontalOffsetVar, var(0.fct()))
    }
}

/// Shorthand [`scrollable!`] with default properties.
///
/// [`scrollable!`]: mod@scrollable
pub fn scrollable(content: impl UiNode) -> impl Widget {
    scrollable!(content)
}
