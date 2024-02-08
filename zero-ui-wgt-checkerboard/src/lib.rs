#![doc = include_str!("../../zero-ui-app/README.md")]
//!
//! Checkerboard widget, properties and nodes.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::ops;

use zero_ui_color::COLOR_SCHEME_VAR;
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

/// Checker board colors.
///
/// See [`colors`](fn@colors) for more details.
#[derive(Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Colors(pub [ColorPair; 2]);
impl ops::Deref for Colors {
    type Target = [ColorPair; 2];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl_from_and_into_var! {
    fn from<C: Into<ColorPair>>([c0, c1]: [C; 2]) -> Colors {
        Colors([c0.into(), c1.into()])
    }
    fn from<C0: Into<ColorPair>, C1: Into<ColorPair>>((c0, c1): (C0, C1)) -> Colors {
        Colors([c0.into(), c1.into()])
    }
}

context_var! {
    /// The checkerboard colors.
    pub static COLORS_VAR: Colors = [
        ColorPair { dark: rgb(20, 20, 20), light: rgb(202, 202, 204) },
        ColorPair { dark: rgb(40, 40, 40), light: rgb(253, 253, 253) },
    ];

    /// Offset applied to the checkerboard pattern.
    ///
    /// Default is no offset, `0`.
    pub static ORIGIN_VAR: Point = Point::zero();

    /// The size of one color rectangle in the checkerboard.
    ///
    /// Default is `10`.
    pub static SIZE_VAR: Size = 10;
}

/// Set both checkerboard colors.
///
/// The values are the interchanging colors for a given color scheme, for example in the dark
/// color scheme the `(colors[0].dark, colors[1].dark)` colors are used.
///
/// This property sets [`COLORS_VAR`] for all inner checkerboard widgets.
#[property(CONTEXT, default(COLORS_VAR), widget_impl(Checkerboard))]
pub fn colors(child: impl UiNode, colors: impl IntoVar<Colors>) -> impl UiNode {
    with_context_var(child, COLORS_VAR, colors)
}

/// Set the size of a checkerboard color rectangle.
///
/// This property sets the [`SIZE_VAR`] for all inner checkerboard widgets.
#[property(CONTEXT, default(SIZE_VAR), widget_impl(Checkerboard))]
pub fn cb_size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    with_context_var(child, SIZE_VAR, size)
}

/// Sets the offset of the checkerboard pattern.
///
/// Relative values are resolved in the context of a [`cb_size`](fn@cb_size).
///
/// This property sets the [`ORIGIN_VAR`] for all inner checkerboard widgets.
#[property(CONTEXT, default(ORIGIN_VAR), widget_impl(Checkerboard))]
pub fn cb_origin(child: impl UiNode, offset: impl IntoVar<Point>) -> impl UiNode {
    with_context_var(child, ORIGIN_VAR, offset)
}

/// Checkerboard node.
///
/// The node is configured by the contextual variables defined in the widget.
pub fn node() -> impl UiNode {
    let mut render_size = PxSize::zero();
    let mut tile_origin = PxPoint::zero();
    let mut tile_size = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&COLORS_VAR)
                .sub_var_render(&COLOR_SCHEME_VAR)
                .sub_var_layout(&SIZE_VAR)
                .sub_var_layout(&ORIGIN_VAR);
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

            let mut ts = SIZE_VAR.layout_dft(PxSize::splat(Px(4)));
            let to = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(ts), || ORIGIN_VAR.layout());

            // each gradient tile has 4 color rectangles.
            ts *= 2.fct();

            if tile_origin != to || tile_size != ts {
                tile_origin = to;
                tile_size = ts;

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            let [c0, c1] = COLORS_VAR.get().0;
            let sch = COLOR_SCHEME_VAR.get();
            let colors = [c0.color(sch).into(), c1.color(sch).into()];

            frame.push_conic_gradient(
                PxRect::from_size(render_size),
                tile_size.to_vector().to_point() / 2.fct(),
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
                tile_origin,
                tile_size,
                PxSize::zero(),
            );
        }
        _ => {}
    })
}
