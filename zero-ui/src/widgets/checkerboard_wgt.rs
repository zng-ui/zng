use crate::prelude::new_widget::*;
use zero_ui_core::gradient::RenderGradientStop;

/// A checkerboard visual.
///
/// This widget draws a checkerboard pattern, with configurable dimensions and colors.
#[widget($crate::widgets::checkerboard)]
pub mod checkerboard {
    use super::*;
    pub use super::{node, properties};

    properties! {
        /// The two checkerboard colors.
        ///
        /// Default is black and white.
        properties::checkerboard_colors as colors = color_scheme_map(
            (rgb(20, 20, 20), rgb(40, 40, 40)),
            (rgb(202, 202, 204), rgb(253, 253, 253))
        );

        /// The size of one color rectangle.
        ///
        /// Note, not to be confused with the [`size`] property that sets the widget dimensions.
        ///
        /// Default is `(20, 20)`.
        properties::checkerboard_size as cb_size;

        /// An offset applied to the checkerboard pattern.
        ///
        /// Default is `(0, 0)`.
        properties::checkerboard_offset as cb_offset;
    }

    fn new_child() -> impl UiNode {
        self::node()
    }
}

/// Contextual properties that affect checkerboards.
pub mod properties {
    use crate::prelude::new_property::*;

    context_var! {
        /// The checkerboard colors.
        ///
        /// Default is ([`BLACK`], [`WHITE`]).
        ///
        /// [`BLACK`]: colors::BLACK
        /// [`WHITE`]: colors::WHITE
        pub static CHECKERBOARD_COLORS_VAR: (Rgba, Rgba) = (colors::BLACK, colors::WHITE);

        /// The size of one color rectangle in the checkerboard.
        ///
        /// Default is `(16, 16)`.
        pub static CHECKERBOARD_SIZE_VAR: Size = (16, 16);

        /// Offset applied to the checkerboard pattern.
        ///
        /// Default is no offset `(0, 0)`.
        pub static CHECKERBOARD_OFFSET_VAR: Vector = Vector::zero();
    }

    /// Set both checkerboard colors.
    ///
    /// This property sets [`CHECKERBOARD_COLORS_VAR`] for all inner checkerboard
    /// widgets. In a checkerboard widget it is called [`colors`].
    ///
    /// [`colors`]: mod@crate::widgets::checkerboard#wp-colors
    #[property(context, default(CHECKERBOARD_COLORS_VAR))]
    pub fn checkerboard_colors(child: impl UiNode, colors: impl IntoVar<(Rgba, Rgba)>) -> impl UiNode {
        with_context_var(child, CHECKERBOARD_COLORS_VAR, colors)
    }

    /// Set the size of a checkerboard color rectangle.
    ///
    /// This property sets the [`CHECKERBOARD_SIZE_VAR`] for all inner checkerboard widgets. In a checkerboard widget
    /// it is called [`cb_size`].
    ///
    /// [`cb_size`]: mod@crate::widgets::checkerboard#wp-cb_size
    #[property(context, default(CHECKERBOARD_SIZE_VAR))]
    pub fn checkerboard_size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
        with_context_var(child, CHECKERBOARD_SIZE_VAR, size)
    }

    /// Sets the offset of the checkerboard pattern.
    ///
    /// This property sets the [`CHECKERBOARD_OFFSET_VAR`] for all inner checkerboard widgets. In a checkerboard widget
    /// it is called [`cb_offset`].
    ///
    /// [`cb_offset`]: mod@crate::widgets::checkerboard#wp-cb_offset
    #[property(context, default(CHECKERBOARD_OFFSET_VAR))]
    pub fn checkerboard_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
        with_context_var(child, CHECKERBOARD_OFFSET_VAR, offset)
    }
}

/// Checkerboard node.
///
/// The node is configured by the contextual variables defined in [`properties`].
pub fn node() -> impl UiNode {
    use crate::core::gradient::RenderExtendMode;
    use properties::*;

    struct CheckerboardNode {
        final_size: PxSize,
        tile_size: PxSize,
        center: PxPoint,
        colors: [RenderColor; 2],
    }
    #[impl_ui_node(none)]
    impl UiNode for CheckerboardNode {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.vars(ctx)
                .var(&CHECKERBOARD_COLORS_VAR)
                .var(&CHECKERBOARD_SIZE_VAR)
                .var(&CHECKERBOARD_OFFSET_VAR);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            let (c0, c1) = CHECKERBOARD_COLORS_VAR.copy(ctx);
            self.colors = [c0.into(), c1.into()];
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some((c0, c1)) = CHECKERBOARD_COLORS_VAR.copy_new(ctx) {
                self.colors = [c0.into(), c1.into()];
                ctx.updates.render();
            }
            if CHECKERBOARD_SIZE_VAR.is_new(ctx) || CHECKERBOARD_OFFSET_VAR.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.constrains().fill_size()
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            self.final_size = ctx.constrains().fill_size();

            let tile_size = CHECKERBOARD_SIZE_VAR.get(ctx.vars).layout(ctx, |_| PxSize::splat(Px(4)));

            let mut offset = CHECKERBOARD_OFFSET_VAR.get(ctx.vars).layout(ctx, |_| PxVector::zero());
            if offset.x > self.tile_size.width {
                offset.x /= self.tile_size.width;
            }
            if offset.y > self.tile_size.height {
                offset.y /= self.tile_size.height;
            }

            let mut center = tile_size.to_vector().to_point() / 2.0.fct();
            center += offset;

            if self.tile_size != tile_size || self.center != center {
                self.tile_size = tile_size;
                self.center = center;

                ctx.updates.render();
            }

            self.final_size
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            frame.push_conic_gradient(
                PxRect::from_size(self.final_size),
                self.center,
                0.rad(),
                &[
                    RenderGradientStop {
                        color: self.colors[0],
                        offset: 0.0,
                    },
                    RenderGradientStop {
                        color: self.colors[0],
                        offset: 0.25,
                    },
                    RenderGradientStop {
                        color: self.colors[1],
                        offset: 0.25,
                    },
                    RenderGradientStop {
                        color: self.colors[1],
                        offset: 0.5,
                    },
                    RenderGradientStop {
                        color: self.colors[0],
                        offset: 0.5,
                    },
                    RenderGradientStop {
                        color: self.colors[0],
                        offset: 0.75,
                    },
                    RenderGradientStop {
                        color: self.colors[1],
                        offset: 0.75,
                    },
                    RenderGradientStop {
                        color: self.colors[1],
                        offset: 1.0,
                    },
                ],
                RenderExtendMode::Repeat,
                self.tile_size,
                PxSize::zero(),
            );
        }
    }
    CheckerboardNode {
        final_size: PxSize::zero(),
        tile_size: PxSize::zero(),
        center: PxPoint::zero(),
        colors: [RenderColor::BLACK, RenderColor::WHITE],
    }
}
