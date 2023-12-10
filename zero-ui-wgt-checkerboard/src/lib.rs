//! Checkerboard widget, properties and nodes.

use zero_ui_wgt::prelude::{
    gradient::{RenderExtendMode, RenderGradientStop},
    *,
};
/// A checkerboard visual.
///
/// This widget draws a checkerboard pattern, with configurable dimensions and colors.
#[widget($crate::Checkerboard)]
pub struct Checkerboard(WidgetBase);
impl Checkerboard {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| wgt.set_child(self::node()));
    }
}

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
#[property(CONTEXT, default(COLORS_VAR))]
pub fn colors(child: impl UiNode, colors: impl IntoVar<(Rgba, Rgba)>) -> impl UiNode {
    with_context_var(child, COLORS_VAR, colors)
}

/// Set the size of a checkerboard color rectangle.
///
/// This property sets the [`SIZE_VAR`] for all inner checkerboard widgets.
#[property(CONTEXT, default(SIZE_VAR))]
pub fn cb_size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    with_context_var(child, SIZE_VAR, size)
}

/// Sets the offset of the checkerboard pattern.
///
/// This property sets the [`OFFSET_VAR`] for all inner checkerboard widgets.
#[property(CONTEXT, default(OFFSET_VAR))]
pub fn cb_offset(child: impl UiNode, offset: impl IntoVar<Vector>) -> impl UiNode {
    with_context_var(child, OFFSET_VAR, offset)
}

/// Checkerboard node.
///
/// The node is configured by the contextual variables defined in the widget.
pub fn node() -> impl UiNode {
    let mut render_size = PxSize::zero();
    let mut tile_size = PxSize::zero();
    let mut center = PxPoint::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&COLORS_VAR)
                .sub_var_layout(&SIZE_VAR)
                .sub_var_layout(&OFFSET_VAR);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;
                WIDGET.render();
            }

            let ts = SIZE_VAR.layout_dft(PxSize::splat(Px(4)));

            let mut offset = OFFSET_VAR.layout();
            if offset.x > ts.width {
                offset.x /= ts.width;
            }
            if offset.y > ts.height {
                offset.y /= ts.height;
            }

            let mut c = ts.to_vector().to_point() / 2.0.fct();
            c += offset;

            if tile_size != ts || center != c {
                tile_size = ts;
                center = c;

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            let (c0, c1) = COLORS_VAR.get();
            let colors = [c0.into(), c1.into()];
            frame.push_conic_gradient(
                PxRect::from_size(render_size),
                center,
                0.rad(),
                &[
                    RenderGradientStop {
                        color: colors[0],
                        offset: 0.0,
                    },
                    RenderGradientStop {
                        color: colors[0],
                        offset: 0.25,
                    },
                    RenderGradientStop {
                        color: colors[1],
                        offset: 0.25,
                    },
                    RenderGradientStop {
                        color: colors[1],
                        offset: 0.5,
                    },
                    RenderGradientStop {
                        color: colors[0],
                        offset: 0.5,
                    },
                    RenderGradientStop {
                        color: colors[0],
                        offset: 0.75,
                    },
                    RenderGradientStop {
                        color: colors[1],
                        offset: 0.75,
                    },
                    RenderGradientStop {
                        color: colors[1],
                        offset: 1.0,
                    },
                ],
                RenderExtendMode::Repeat,
                tile_size,
                PxSize::zero(),
            );
        }
        _ => {}
    })
}
