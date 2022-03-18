use crate::prelude::new_widget::*;

/// A checkerboard visual.
///
/// This widget draws a checkerboard pattern, with configurable dimensions and colors.
#[widget($crate::widgets::checkerboard)]
pub mod checkerboard {
    use zero_ui_core::gradient::RenderGradientStop;

    use super::*;

    properties! {
        /// The two checkerboard colors.
        ///
        /// Default is black and white.
        properties::checkerboard_colors as colors;

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
        implicit_base::nodes::leaf_transform(self::node())
    }

    /// Contextual properties that affect checkerboards.
    pub mod properties {
        use crate::prelude::new_property::*;

        context_var! {
            /// The first checkerboard color.
            ///
            /// Default is [`BLACK`].
            ///
            /// [`BLACK`]: colors::BLACK
            pub struct CheckerboardColor0Var: Rgba = colors::BLACK;
            /// The second checkerboard color.
            ///
            /// Default is [`WHITE`].
            ///
            /// [`WHITE`]: colors::WHITE
            pub struct CheckerboardColor1Var: Rgba = colors::WHITE;

            /// The size of one color rectangle in the checkerboard.
            ///
            /// Default is `(20, 20)`.
            pub struct CheckerboardSizeVar: Size = (20, 20).into();

            /// Offset applied to the checkerboard pattern.
            ///
            /// Default is no offset `(0, 0)`.
            pub struct CheckerboardOffsetVar: Vector = Vector::zero();
        }

        /// Set both checkerboard colors.
        ///
        /// This property sets [`CheckerboardColor0Var`] and [`CheckerboardColor1Var`] for all inner checkerboard
        /// widgets. In a checkerboard widget it is called [`colors`].
        ///
        /// [`colors`]: mod@crate::widgets::checkerboard#wp-colors
        #[property(context, default(colors::BLACK, colors::WHITE))]
        pub fn checkerboard_colors(child: impl UiNode, color0: impl IntoVar<Rgba>, color1: impl IntoVar<Rgba>) -> impl UiNode {
            let node = with_context_var(child, CheckerboardColor0Var, color0);
            with_context_var(node, CheckerboardColor1Var, color1)
        }

        /// Set the size of a checkerboard color rectangle.
        ///
        /// This property sets the [`CheckerboardSizeVar`] for all inner checkerboard widgets. In a checkerboard widget
        /// it is called [`cb_size`].
        ///
        /// [`cb_size`]: mod@crate::widgets::checkerboard#wp-cb_size
        #[property(context, default((20, 20)))]
        pub fn checkerboard_size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
            with_context_var(child, CheckerboardSizeVar, size)
        }

        /// Sets the offset of the checkerboard pattern.
        ///
        /// This property sets the [`CheckerboardOffsetVar`] for all inner checkerboard widgets. In a checkerboard widget
        /// it is called [`cb_offset`].
        ///
        /// [`cb_offset`]: mod@crate::widgets::checkerboard#wp-cb_offset
        #[property(context, default(Vector::zero()))]
        pub fn checkerboard_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
            with_context_var(child, CheckerboardOffsetVar, offset)
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
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                subscriptions
                    .vars(ctx)
                    .var(&CheckerboardColor0Var::new())
                    .var(&CheckerboardColor1Var::new())
                    .var(&CheckerboardSizeVar::new())
                    .var(&CheckerboardOffsetVar::new());
            }

            fn init(&mut self, ctx: &mut WidgetContext) {
                self.colors = [(*CheckerboardColor0Var::get(ctx)).into(), (*CheckerboardColor1Var::get(ctx)).into()];
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                if let Some(&c0) = CheckerboardColor0Var::get_new(ctx) {
                    self.colors[0] = c0.into();
                    ctx.updates.render();
                }
                if let Some(&c1) = CheckerboardColor1Var::get_new(ctx) {
                    self.colors[1] = c1.into();
                    ctx.updates.render();
                }
                if CheckerboardSizeVar::is_new(ctx) || CheckerboardOffsetVar::is_new(ctx) {
                    ctx.updates.layout();
                }
            }

            fn arrange(&mut self, ctx: &mut LayoutContext, _: &mut WidgetLayout, final_size: PxSize) {
                self.final_size = final_size;
                let available_size = AvailableSize::from_size(final_size);

                let tile_size = CheckerboardSizeVar::get(ctx.vars).to_layout(ctx, available_size, PxSize::splat(Px(4)));

                let mut offset = CheckerboardOffsetVar::get(ctx.vars).to_layout(ctx, available_size, PxVector::zero());
                if offset.x > self.tile_size.width {
                    offset.x /= self.tile_size.width;
                }
                if offset.y > self.tile_size.height {
                    offset.y /= self.tile_size.height;
                }

                let mut center = self.tile_size.to_vector().to_point() / 2.0.fct();
                center += offset;

                if self.tile_size != tile_size || self.center != center {
                    self.tile_size = tile_size;
                    self.center = center;

                    ctx.updates.render();
                }
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
}
