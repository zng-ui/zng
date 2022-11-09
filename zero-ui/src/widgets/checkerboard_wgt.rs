use crate::prelude::new_widget::*;
use zero_ui_core::gradient::RenderGradientStop;

/// A checkerboard visual.
///
/// This widget draws a checkerboard pattern, with configurable dimensions and colors.
#[widget($crate::widgets::checkerboard)]
pub mod checkerboard {
    pub use super::node;
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::checkerboard_properties::*;

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| wgt.set_child(self::node()))
    }
}

mod checkerboard_properties {
    use crate::prelude::new_property::*;

    context_var! {
        /// The checkerboard colors.
        ///
        /// Default depends on the color scheme.
        ///
        /// [`BLACK`]: colors::BLACK
        /// [`WHITE`]: colors::WHITE
        pub static COLORS_VAR: (Rgba, Rgba) = color_scheme_map(
            (rgb(20, 20, 20), rgb(40, 40, 40)),
            (rgb(202, 202, 204), rgb(253, 253, 253))
        );

        /// The size of one color rectangle in the checkerboard.
        ///
        /// Default is `(20, 20)`.
        pub static SIZE_VAR: Size = (20, 20);

        /// Offset applied to the checkerboard pattern.
        ///
        /// Default is no offset `(0, 0)`.
        pub static OFFSET_VAR: Vector = Vector::zero();
    }

    /// Set both checkerboard colors.
    ///
    /// This property sets [`COLORS_VAR`] for all inner checkerboard widgets.
    ///
    /// [`colors`]: mod@crate::widgets::checkerboard#wp-colors
    #[property(CONTEXT, default(COLORS_VAR))]
    pub fn colors(child: impl UiNode, colors: impl IntoVar<(Rgba, Rgba)>) -> impl UiNode {
        with_context_var(child, COLORS_VAR, colors)
    }

    /// Set the size of a checkerboard color rectangle.
    ///
    /// This property sets the [`SIZE_VAR`] for all inner checkerboard widgets.
    ///
    /// [`cb_size`]: mod@crate::widgets::checkerboard#wp-cb_size
    #[property(CONTEXT, default(SIZE_VAR))]
    pub fn cb_size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
        with_context_var(child, SIZE_VAR, size)
    }

    /// Sets the offset of the checkerboard pattern.
    ///
    /// This property sets the [`OFFSET_VAR`] for all inner checkerboard widgets.
    ///
    /// [`cb_offset`]: mod@crate::widgets::checkerboard#wp-cb_offset
    #[property(CONTEXT, default(OFFSET_VAR))]
    pub fn cb_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
        with_context_var(child, OFFSET_VAR, offset)
    }
}

/// Checkerboard node.
///
/// The node is configured by the contextual variables defined in the widget.
pub fn node() -> impl UiNode {
    use crate::core::gradient::RenderExtendMode;
    use checkerboard_properties::*;

    #[ui_node(struct CheckerboardNode {
        final_size: PxSize,
        tile_size: PxSize,
        center: PxPoint,
        colors: [RenderColor; 2],
    })]
    impl UiNode for CheckerboardNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&COLORS_VAR).sub_var(&SIZE_VAR).sub_var(&OFFSET_VAR);

            let (c0, c1) = COLORS_VAR.get();
            self.colors = [c0.into(), c1.into()];
        }

        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if let Some((c0, c1)) = COLORS_VAR.get_new(ctx) {
                self.colors = [c0.into(), c1.into()];
                ctx.updates.render();
            }
            if SIZE_VAR.is_new(ctx) || OFFSET_VAR.is_new(ctx) {
                ctx.updates.layout();
            }
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            ctx.constrains().fill_size()
        }
        fn layout(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout) -> PxSize {
            self.final_size = ctx.constrains().fill_size();

            let tile_size = SIZE_VAR.get().layout(ctx, |_| PxSize::splat(Px(4)));

            let mut offset = OFFSET_VAR.get().layout(ctx, |_| PxVector::zero());
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
