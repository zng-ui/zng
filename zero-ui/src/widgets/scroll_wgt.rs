use crate::prelude::new_widget::*;

pub mod commands;
pub mod nodes;
pub mod scroll_properties;

mod parts;
mod types;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::scroll)]
pub mod scroll {
    use super::*;

    inherit!(container);

    #[doc(inline)]
    pub use super::{
        commands, nodes,
        parts::{scrollbar, thumb},
        types::*,
    };

    pub mod properties {
        #[doc(inline)]
        pub use super::scroll_properties::*;
    }

    properties! {
        /// Content alignment when it is smaller then the viewport.
        child_align = Align::CENTER;

        /// Clip content to only be visible within the scroll bounds, including under scrollbars.
        ///
        /// Enabled by default.
        clip_to_bounds = true;

        /// Clip content to only be visible within the viewport, not under scrollbars.
        ///
        /// Disabled by default.
        pub clip_to_viewport(impl IntoVar<bool>) = false;

        /// Scroll mode.
        ///
        /// By default scrolls in both dimensions.
        pub mode(impl IntoVar<ScrollMode>) = ScrollMode::ALL;

        /// Enables keyboard controls.
        focusable = true;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(on_build);
    }
    fn on_build(wgt: &mut WidgetBuilding) {
        let mode = wgt.capture_var_or_else(property_id!(self.mode), || ScrollMode::ALL);

        let clip_to_viewport = wgt.capture_var_or_default(property_id!(self.clip_to_viewport));

        wgt.push_intrinsic(Priority::ChildContext, "scroll_node", |child| {
            scroll_node(child, mode, clip_to_viewport)
        });

        wgt.push_intrinsic(Priority::Event, "commands", |child| {
            let child = nodes::scroll_to_node(child);
            let child = nodes::scroll_commands_node(child);
            let child = nodes::page_commands_node(child);
            let child = nodes::scroll_to_edge_commands_node(child);
            nodes::scroll_wheel_node(child)
        });

        wgt.push_intrinsic(Priority::Context, "context", |child| {
            let child = with_context_var(child, SCROLL_VIEWPORT_SIZE_VAR, var(PxSize::zero()));
            let child = with_context_var(child, SCROLL_CONTENT_SIZE_VAR, var(PxSize::zero()));

            let child = with_context_var(child, SCROLL_VERTICAL_RATIO_VAR, var(0.fct()));
            let child = with_context_var(child, SCROLL_HORIZONTAL_RATIO_VAR, var(0.fct()));

            let child = with_context_var(child, SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR, var(false));
            let child = with_context_var(child, SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR, var(false));

            let child = ScrollContext::config_node(child);

            let child = with_context_var(child, SCROLL_VERTICAL_OFFSET_VAR, var(0.fct()));
            with_context_var(child, SCROLL_HORIZONTAL_OFFSET_VAR, var(0.fct()))
        });
    }

    fn scroll_node(child: impl UiNode, mode: impl IntoVar<ScrollMode>, clip_to_viewport: impl IntoVar<bool>) -> impl UiNode {
        #[ui_node(struct ScrollNode {
            children: impl UiNodeList,
            viewport: PxSize,
            joiner: PxSize,
            spatial_id: SpatialFrameId,
        })]
        impl UiNode for ScrollNode {
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
                let size = self.children.with_node(0, |n| n.measure(ctx));
                constrains.clamp_size(size)
            }
            fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
                // scrollbars
                {
                    let mut ctx = ctx.as_measure();
                    self.joiner.width = ctx.with_constrains(
                        |c| c.with_min_x(Px(0)).with_fill(false, true),
                        |ctx| self.children.with_node(1, |n| n.measure(ctx)).width,
                    );
                    self.joiner.height = ctx.with_constrains(
                        |c| c.with_min_y(Px(0)).with_fill(true, false),
                        |ctx| self.children.with_node(2, |n| n.measure(ctx)).height,
                    );
                }
                self.joiner.width = ctx.with_constrains(
                    |c| c.with_min_x(Px(0)).with_fill(false, true).with_less_y(self.joiner.height),
                    |ctx| self.children.with_node_mut(1, |n| n.layout(ctx, wl)).width,
                );
                self.joiner.height = ctx.with_constrains(
                    |c| c.with_min_y(Px(0)).with_fill(true, false).with_less_x(self.joiner.width),
                    |ctx| self.children.with_node_mut(2, |n| n.layout(ctx, wl)).height,
                );

                // joiner
                let _ = ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(self.joiner),
                    |ctx| self.children.with_node_mut(3, |n| n.layout(ctx, wl)),
                );

                // viewport
                let mut viewport = ctx.with_constrains(
                    |c| c.with_less_size(self.joiner),
                    |ctx| self.children.with_node_mut(0, |n| n.layout(ctx, wl)),
                );

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
                self.children.with_node(0, |n| n.render(ctx, frame));

                if self.joiner.width > Px(0) {
                    let transform = PxTransform::from(PxVector::new(self.viewport.width, Px(0)));
                    frame.push_reference_frame_item(self.spatial_id, 1, FrameValue::Value(transform), true, false, |frame| {
                        self.children.with_node(1, |n| n.render(ctx, frame));
                    });
                }

                if self.joiner.height > Px(0) {
                    let transform = PxTransform::from(PxVector::new(Px(0), self.viewport.height));
                    frame.push_reference_frame_item(self.spatial_id, 2, FrameValue::Value(transform), true, false, |frame| {
                        self.children.with_node(2, |n| n.render(ctx, frame));
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    let transform = PxTransform::from(self.viewport.to_vector());
                    frame.push_reference_frame_item(self.spatial_id, 3, FrameValue::Value(transform), true, false, |frame| {
                        self.children.with_node(3, |n| n.render(ctx, frame));
                    });
                }
            }

            fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                self.children.with_node(0, |n| n.render_update(ctx, update));

                if self.joiner.width > Px(0) {
                    let transform = PxTransform::from(PxVector::new(self.viewport.width, Px(0)));
                    update.with_transform_value(&transform, |update| {
                        self.children.with_node(1, |n| n.render_update(ctx, update));
                    });
                }

                if self.joiner.height > Px(0) {
                    let transform = PxTransform::from(PxVector::new(Px(0), self.viewport.height));
                    update.with_transform_value(&transform, |update| {
                        self.children.with_node(2, |n| n.render_update(ctx, update));
                    });
                }

                if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                    let transform = PxTransform::from(self.viewport.to_vector());
                    update.with_transform_value(&transform, |update| {
                        self.children.with_node(3, |n| n.render_update(ctx, update));
                    });
                }
            }
        }

        use crate::core::context::UpdatesTraceUiNodeExt;
        ScrollNode {
            children: ui_list![
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
}

/// Shorthand [`scroll!`] with default properties.
///
/// [`scroll!`]: mod@scroll
pub fn scroll(child: impl UiNode) -> impl UiNode {
    scroll!(child)
}
