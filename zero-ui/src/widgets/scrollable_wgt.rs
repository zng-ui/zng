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
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        implicit_base::nodes::leaf_transform(content)
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
            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                let v_scroll = self.children.widget_measure(1, ctx, available_size);
                let h_scroll = self.children.widget_measure(2, ctx, available_size);

                self.joiner = PxSize::new(v_scroll.width, h_scroll.height);
                let viewport = self.children.widget_measure(0, ctx, available_size.sub_px(self.joiner));

                let _ = self.children.widget_measure(3, ctx, AvailableSize::from_size(self.joiner));

                available_size.clip(viewport + self.joiner)
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                let mut viewport = final_size - self.joiner;

                if viewport.width < self.joiner.width * 3.0.fct() {
                    self.joiner.width = Px(0);
                    viewport.width = final_size.width;
                }
                if viewport.height < self.joiner.height * 3.0.fct() {
                    self.joiner.height = Px(0);
                    viewport.height = final_size.height;
                }

                if viewport != self.viewport {
                    self.viewport = viewport;
                    ctx.updates.render();
                }

                self.children.widget_arrange(0, ctx, widget_layout, self.viewport);

                let joiner_offset = self.viewport.to_vector();
                widget_layout.with_custom_transform(&RenderTransform::translation_px(PxVector::new(joiner_offset.x, Px(0))), |wo| {
                    self.children
                        .widget_arrange(1, ctx, wo, PxSize::new(self.joiner.width, self.viewport.height))
                });
                widget_layout.with_custom_transform(&RenderTransform::translation_px(PxVector::new(Px(0), joiner_offset.y)), |wo| {
                    self.children
                        .widget_arrange(2, ctx, wo, PxSize::new(self.viewport.width, self.joiner.height))
                });

                widget_layout.with_custom_transform(&RenderTransform::translation_px(joiner_offset), |wo| {
                    self.children.widget_arrange(3, ctx, wo, self.joiner)
                });
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

    fn new_context(child: impl UiNode) -> impl UiNode {
        let child = nodes::scroll_to_command_node(child);
        let child = nodes::scroll_commands_node(child);
        let child = nodes::page_commands_node(child);
        let child = nodes::scroll_to_edge_commands_node(child);
        let child = nodes::scroll_wheel_node(child);

        let viewport_size = var(PxSize::zero());
        let child = with_context_var(child, ScrollViewportSizeWriteVar, viewport_size.clone());
        let child = with_context_var(child, ScrollViewportSizeVar, viewport_size.into_read_only());

        let content_size = var(PxSize::zero());
        let child = with_context_var(child, ScrollContentSizeWriteVar, content_size.clone());
        let child = with_context_var(child, ScrollContentSizeVar, content_size.into_read_only());

        let v_ratio = var(0.fct());
        let child = with_context_var(child, ScrollVerticalRatioWriteVar, v_ratio.clone());
        let child = with_context_var(child, ScrollVerticalRatioVar, v_ratio.into_read_only());

        let h_ratio = var(0.fct());
        let child = with_context_var(child, ScrollHorizontalRatioWriteVar, h_ratio.clone());
        let child = with_context_var(child, ScrollHorizontalRatioVar, h_ratio.into_read_only());

        let child = with_context_var(child, ScrollVerticalOffsetVar, var(0.fct()));
        with_context_var(child, ScrollHorizontalOffsetVar, var(0.fct()))
    }
}

/// Shorthand [`scrollable!`] with default properties.
///
/// [`scrollable!`]: mod@scrollable
pub fn scrollable(content: impl UiNode) -> impl UiNode {
    scrollable!(content)
}
