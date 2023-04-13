//! Scroll widgets, properties and nodes..

use crate::prelude::new_widget::*;

pub mod commands;
pub mod nodes;
pub mod scrollbar;
pub mod thumb;

mod scroll_properties;
mod types;

pub use scroll_properties::*;
pub use types::*;

pub use scrollbar::Scrollbar;
pub use thumb::Thumb;

/// A single content container that can be larger on the inside.
#[widget($crate::widgets::Scroll)]
pub struct Scroll(Container);
impl Scroll {
    #[widget(on_start)]
    fn on_start(&mut self) {
        defaults! {
            self;
            child_align = Align::CENTER;
            clip_to_bounds = true;
            focusable = true;
        }
        self.builder().push_build_action(on_build);
    }

    impl_properties! {
        /// Content alignment when it is smaller then the viewport.
        pub fn child_align(align: impl IntoVar<Align>);

        /// Clip content to only be visible within the scroll bounds, including under scrollbars.
        ///
        /// Enabled by default.
        pub fn clip_to_bounds(clip: impl IntoVar<bool>);

        /// Enables keyboard controls.
        pub fn focusable(focusable: impl IntoVar<bool>);
    }
}

/// Clip content to only be visible within the viewport, not under scrollbars.
///
/// Disabled by default.
#[property(CONTEXT, capture, default(false), impl(Scroll))]
pub fn clip_to_viewport(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {}

/// Scroll mode.
///
/// By default scrolls in both dimensions.
#[property(CONTEXT, capture, default(ScrollMode::ALL), impl(Scroll))]
pub fn mode(child: impl UiNode, mode: impl IntoVar<ScrollMode>) -> impl UiNode {}

fn on_build(wgt: &mut WidgetBuilding) {
    let mode = wgt.capture_var_or_else(property_id!(mode), || ScrollMode::ALL);

    let clip_to_viewport = wgt.capture_var_or_default(property_id!(clip_to_viewport));

    wgt.push_intrinsic(NestGroup::CHILD_CONTEXT, "scroll_node", |child| {
        scroll_node(child, mode, clip_to_viewport)
    });

    wgt.push_intrinsic(NestGroup::EVENT, "commands", |child| {
        let child = nodes::scroll_to_node(child);
        let child = nodes::scroll_commands_node(child);
        let child = nodes::page_commands_node(child);
        let child = nodes::scroll_to_edge_commands_node(child);
        nodes::scroll_wheel_node(child)
    });

    wgt.push_intrinsic(NestGroup::CONTEXT, "context", |child| {
        let child = with_context_var(child, SCROLL_VIEWPORT_SIZE_VAR, var(PxSize::zero()));
        let child = with_context_var(child, SCROLL_CONTENT_SIZE_VAR, var(PxSize::zero()));

        let child = with_context_var(child, SCROLL_VERTICAL_RATIO_VAR, var(0.fct()));
        let child = with_context_var(child, SCROLL_HORIZONTAL_RATIO_VAR, var(0.fct()));

        let child = with_context_var(child, SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR, var(false));
        let child = with_context_var(child, SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR, var(false));

        let child = SCROLL.config_node(child);

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

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            let constraints = LAYOUT.constraints();
            if constraints.is_fill_max().all() {
                return constraints.fill_size();
            }
            let size = self.children.with_node(0, |n| n.measure(wm));
            constraints.clamp_size(size)
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            // scrollbars
            let c = LAYOUT.constraints();
            {
                self.joiner.width = LAYOUT.with_constraints(c.with_min_x(Px(0)).with_fill(false, true), || {
                    self.children.with_node(1, |n| n.measure(&mut WidgetMeasure::new())).width
                });
                self.joiner.height = LAYOUT.with_constraints(c.with_min_y(Px(0)).with_fill(true, false), || {
                    self.children.with_node(2, |n| n.measure(&mut WidgetMeasure::new())).height
                });
            }
            self.joiner.width = LAYOUT.with_constraints(c.with_min_x(Px(0)).with_fill(false, true).with_less_y(self.joiner.height), || {
                self.children.with_node_mut(1, |n| n.layout(wl)).width
            });
            self.joiner.height = LAYOUT.with_constraints(c.with_min_y(Px(0)).with_fill(true, false).with_less_x(self.joiner.width), || {
                self.children.with_node_mut(2, |n| n.layout(wl)).height
            });

            // joiner
            let _ = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(self.joiner), || {
                self.children.with_node_mut(3, |n| n.layout(wl))
            });

            // viewport
            let mut viewport = LAYOUT.with_constraints(c.with_less_size(self.joiner), || self.children.with_node_mut(0, |n| n.layout(wl)));

            // arrange
            let final_size = viewport + self.joiner;
            let content_size = SCROLL_CONTENT_SIZE_VAR.get();

            if content_size.height > final_size.height {
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set_ne(true).unwrap();
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR
                    .set_ne(content_size.width > viewport.width)
                    .unwrap();
            } else if content_size.width > final_size.width {
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set_ne(true).unwrap();
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR
                    .set_ne(content_size.height > viewport.height)
                    .unwrap();
            } else {
                SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR.set_ne(false).unwrap();
                SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR.set_ne(false).unwrap();
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
                WIDGET.render();
            }

            self.viewport + self.joiner
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.children.with_node(0, |n| n.render(frame));

            if self.joiner.width > Px(0) {
                let transform = PxTransform::from(PxVector::new(self.viewport.width, Px(0)));
                frame.push_reference_frame((self.spatial_id, 1).into(), FrameValue::Value(transform), true, false, |frame| {
                    self.children.with_node(1, |n| n.render(frame));
                });
            }

            if self.joiner.height > Px(0) {
                let transform = PxTransform::from(PxVector::new(Px(0), self.viewport.height));
                frame.push_reference_frame((self.spatial_id, 2).into(), FrameValue::Value(transform), true, false, |frame| {
                    self.children.with_node(2, |n| n.render(frame));
                });
            }

            if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                let transform = PxTransform::from(self.viewport.to_vector());
                frame.push_reference_frame((self.spatial_id, 3).into(), FrameValue::Value(transform), true, false, |frame| {
                    self.children.with_node(3, |n| n.render(frame));
                });
            }
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            self.children.with_node(0, |n| n.render_update(update));

            if self.joiner.width > Px(0) {
                let transform = PxTransform::from(PxVector::new(self.viewport.width, Px(0)));
                update.with_transform_value(&transform, |update| {
                    self.children.with_node(1, |n| n.render_update(update));
                });
            }

            if self.joiner.height > Px(0) {
                let transform = PxTransform::from(PxVector::new(Px(0), self.viewport.height));
                update.with_transform_value(&transform, |update| {
                    self.children.with_node(2, |n| n.render_update(update));
                });
            }

            if self.joiner.width > Px(0) && self.joiner.height > Px(0) {
                let transform = PxTransform::from(self.viewport.to_vector());
                update.with_transform_value(&transform, |update| {
                    self.children.with_node(3, |n| n.render_update(update));
                });
            }
        }
    }

    ScrollNode {
        children: ui_vec![
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
