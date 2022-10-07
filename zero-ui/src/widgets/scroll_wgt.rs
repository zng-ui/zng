use crate::prelude::new_widget::*;

pub mod commands;
pub mod nodes;
pub mod properties;

mod parts;
mod types;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::scroll)]
pub mod scroll {
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

        /// Clip content to only be visible within the scroll bounds, including under scrollbars.
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

        /// If the viewport size is used as the [`LayoutMetrics::viewport`] for the scrollable content.
        ///
        /// Note that this is only applied if the viewport size can be computed before the content size and is non-zero in both dimensions,
        /// this is the case in the normal usage where the scroll fills the parent or when it has an exact size.
        ///
        /// This is enabled by default.
        define_viewport_unit;
    }

    fn new_child(content: impl UiNode) -> impl UiNode {
        content
    }

    fn new_child_context(child: impl UiNode, mode: impl IntoVar<ScrollMode>, clip_to_viewport: impl IntoVar<bool>) -> impl UiNode {
        struct ScrollNode<N> {
            children: N,
            viewport: PxSize,
            joiner: PxSize,
            spatial_id: SpatialFrameId,
        }
        #[impl_ui_node(children)]
        impl<N: UiNodeList> UiNode for ScrollNode<N> {
            // # Layout
            //
            // +-----------------+---+
            // |                 |   |
            // | 0 - viewport    | 1 | - v_scrollbar
            // |                 |   |
            // +-----------------+---+
            // | 2 - h_scrollbar | 3 | - scrollbar_joiner
            // +-----------------+---+

            fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
                let constrains = ctx.constrains();
                if constrains.is_fill_max().all() {
                    return constrains.fill_size();
                }
                let size = self.children.item_measure(0, ctx);
                constrains.clamp_size(size)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                // scrollbars
                {
                    let mut ctx = ctx.as_measure();
                    self.joiner.width = ctx.with_constrains(
                        |c| c.with_min_x(Px(0)).with_fill(false, true),
                        |ctx| self.children.item_measure(1, ctx).width,
                    );
                    self.joiner.height = ctx.with_constrains(
                        |c| c.with_min_y(Px(0)).with_fill(true, false),
                        |ctx| self.children.item_measure(2, ctx).height,
                    );
                }
                self.joiner.width = ctx.with_constrains(
                    |c| c.with_min_x(Px(0)).with_fill(false, true).with_less_y(self.joiner.height),
                    |ctx| self.children.item_layout(1, ctx, wl).width,
                );
                self.joiner.height = ctx.with_constrains(
                    |c| c.with_min_y(Px(0)).with_fill(true, false).with_less_x(self.joiner.width),
                    |ctx| self.children.item_layout(2, ctx, wl).height,
                );

                // joiner
                let _ = ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(self.joiner),
                    |ctx| self.children.item_layout(3, ctx, wl),
                );

                // viewport
                let mut viewport = ctx.with_constrains(|c| c.with_less_size(self.joiner), |ctx| self.children.item_layout(0, ctx, wl));

                // arrange
                let final_size = viewport + self.joiner;
                let content_size = SCROLL_CONTENT_SIZE_VAR.get();

                if content_size.height > final_size.height {
                    SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set_ne(ctx, true).unwrap();
                    SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR
                        .set_ne(ctx, content_size.width > viewport.width)
                        .unwrap();
                } else if content_size.width > final_size.width {
                    SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set_ne(ctx, true).unwrap();
                    SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR
                        .set_ne(ctx, content_size.height > viewport.height)
                        .unwrap();
                } else {
                    SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set_ne(ctx, false).unwrap();
                    SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set_ne(ctx, false).unwrap();
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
                self.children.item_render(0, ctx, frame);

                if self.joiner.width > Px(0) {
                    let transform = PxTransform::from(PxVector::new(self.viewport.width, Px(0)));
                    frame.push_reference_frame_item(self.spatial_id, 1, FrameValue::Value(transform), true, false, |frame| {
                        self.children.item_render(1, ctx, frame);
                    });
                }

                if self.joiner.height > Px(0) {
                    let transform = PxTransform::from(PxVector::new(Px(0), self.viewport.height));
                    frame.push_reference_frame_item(self.spatial_id, 2, FrameValue::Value(transform), true, false, |frame| {
                        self.children.item_render(2, ctx, frame);
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    let transform = PxTransform::from(self.viewport.to_vector());
                    frame.push_reference_frame_item(self.spatial_id, 3, FrameValue::Value(transform), true, false, |frame| {
                        self.children.item_render(3, ctx, frame);
                    });
                }
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                self.children.item_render_update(0, ctx, update);

                if self.joiner.width > Px(0) {
                    let transform = PxTransform::from(PxVector::new(self.viewport.width, Px(0)));
                    update.with_transform_value(&transform, |update| {
                        self.children.item_render_update(1, ctx, update);
                    });
                }

                if self.joiner.height > Px(0) {
                    let transform = PxTransform::from(PxVector::new(Px(0), self.viewport.height));
                    update.with_transform_value(&transform, |update| {
                        self.children.item_render_update(2, ctx, update);
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    let transform = PxTransform::from(self.viewport.to_vector());
                    update.with_transform_value(&transform, |update| {
                        self.children.item_render_update(3, ctx, update);
                    });
                }
            }
        }

        use crate::core::context::UpdatesTraceUiNodeExt;
        ScrollNode {
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
        let child = nodes::scroll_to_node(child);
        let child = nodes::scroll_commands_node(child);
        let child = nodes::page_commands_node(child);
        let child = nodes::scroll_to_edge_commands_node(child);
        nodes::scroll_wheel_node(child)
    }

    fn new_context(child: impl UiNode) -> impl UiNode {
        let child = with_context_var(child, SCROLL_VIEWPORT_SIZE_VAR, var(PxSize::zero()));
        let child = with_context_var(child, SCROLL_CONTENT_SIZE_VAR, var(PxSize::zero()));

        let child = with_context_var(child, SCROLL_VERTICAL_RATIO_VAR, var(0.fct()));
        let child = with_context_var(child, SCROLL_HORIZONTAL_RATIO_VAR, var(0.fct()));

        let child = with_context_var(child, SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR, var(false));
        let child = with_context_var(child, SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR, var(false));

        let child = ScrollContext::config_node(child);

        let child = with_context_var(child, SCROLL_VERTICAL_OFFSET_VAR, var(0.fct()));
        with_context_var(child, SCROLL_HORIZONTAL_OFFSET_VAR, var(0.fct()))
    }
}

/// Shorthand [`scroll!`] with default properties.
///
/// [`scroll!`]: mod@scroll
pub fn scroll(content: impl UiNode) -> impl Widget {
    scroll!(content)
}
