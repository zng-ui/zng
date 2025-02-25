#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Rule line widgets.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_wgt::{margin, prelude::*};
use zng_wgt_access::{AccessRole, access_role};

/// Draws a horizontal or vertical rule line.
#[widget($crate::RuleLine)]
pub struct RuleLine(WidgetBase);
impl RuleLine {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(on_build);

        widget_set! {
            self;
            access_role = AccessRole::Separator;
        }
    }

    widget_impl! {
        /// Margin around the line.
        pub margin(margin: impl IntoVar<SideOffsets>);
    }
}

/// Line orientation.
#[property(CONTEXT, capture, default(LineOrientation::Horizontal), widget_impl(RuleLine))]
pub fn orientation(orientation: impl IntoVar<LineOrientation>) {}

/// Line color.
#[property(CONTEXT, capture, default(rgb(0, 0, 0)), widget_impl(RuleLine))]
pub fn color(color: impl IntoVar<Rgba>) {}

/// Line stroke thickness.
#[property(CONTEXT, capture, default(1), widget_impl(RuleLine))]
pub fn stroke_thickness(thickness: impl IntoVar<Length>) {}

/// Line length.
///
/// Set to [`Default`] to fill available length without requesting any length.
///
/// [`Default`]: Length::Default
#[property(CONTEXT, capture, default(Length::Default), widget_impl(RuleLine))]
pub fn length(length: impl IntoVar<Length>) {}

/// Line style.
#[property(CONTEXT, capture, default(LineStyle::Solid), widget_impl(RuleLine))]
pub fn line_style(style: impl IntoVar<LineStyle>) {}

fn on_build(wgt: &mut WidgetBuilding) {
    let mut bounds = PxSize::zero();

    let orientation = wgt
        .capture_var(property_id!(orientation))
        .unwrap_or_else(|| LineOrientation::Horizontal.into_var().boxed());

    let length = wgt
        .capture_var(property_id!(length))
        .unwrap_or_else(|| LocalVar(Length::Default).boxed());

    let stroke_thickness = wgt
        .capture_var(property_id!(stroke_thickness))
        .unwrap_or_else(|| LocalVar(Length::from(1)).boxed());

    let color = wgt
        .capture_var(property_id!(color))
        .unwrap_or_else(|| LocalVar(rgb(0, 0, 0)).boxed());

    let style = wgt
        .capture_var(property_id!(line_style))
        .unwrap_or_else(|| LineStyle::Solid.into_var().boxed());

    wgt.set_child(match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&stroke_thickness)
                .sub_var_layout(&orientation)
                .sub_var_layout(&length)
                .sub_var_render(&color)
                .sub_var_render(&style);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            let metrics = LAYOUT.metrics();
            let default_stroke = Dip::new(1).to_px(metrics.scale_factor());

            *desired_size = match orientation.get() {
                LineOrientation::Horizontal => PxSize::new(
                    length.layout_dft_x(metrics.constraints().x.fill()),
                    stroke_thickness.layout_dft_y(default_stroke),
                ),
                LineOrientation::Vertical => PxSize::new(
                    stroke_thickness.layout_dft_x(default_stroke),
                    length.layout_dft_y(metrics.constraints().y.fill()),
                ),
            };
        }
        UiNodeOp::Layout { final_size, .. } => {
            let metrics = LAYOUT.metrics();
            let default_stroke = Dip::new(1).to_px(metrics.scale_factor());

            let b = match orientation.get() {
                LineOrientation::Horizontal => PxSize::new(
                    length.layout_dft_x(metrics.constraints().x.fill()),
                    stroke_thickness.layout_dft_y(default_stroke),
                ),
                LineOrientation::Vertical => PxSize::new(
                    stroke_thickness.layout_dft_x(default_stroke),
                    length.layout_dft_y(metrics.constraints().y.fill()),
                ),
            };

            if b != bounds {
                bounds = b;
                WIDGET.render();
            }

            *final_size = b;
        }
        UiNodeOp::Render { frame } => {
            let bounds = PxRect::from_size(bounds);
            let orientation = orientation.get();
            let color = color.get();
            let style = style.get();
            frame.push_line(bounds, orientation, color, style);
        }
        _ => {}
    }));
}

/// Horizontal rule line.
pub mod hr {
    use zng_wgt::prelude::*;

    /// Draws an horizontal [`RuleLine!`](struct@crate::RuleLine).
    #[widget($crate::hr::Hr)]
    pub struct Hr(super::RuleLine);
    impl Hr {
        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                orientation = LineOrientation::Horizontal;
                color = COLOR_VAR;
                stroke_thickness = STROKE_THICKNESS_VAR;
                line_style = LINE_STYLE_VAR;
                margin = MARGIN_VAR;
            }
        }
    }

    context_var! {
        /// Line color, inherits from [`FONT_COLOR_VAR`].
        ///
        /// [`FONT_COLOR_VAR`]: zng_wgt_text::FONT_COLOR_VAR
        pub static COLOR_VAR: Rgba = zng_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(30.pct()));

        /// Line stroke thickness, default is `1.dip()`
        pub static STROKE_THICKNESS_VAR: Length = 1.dip();

        /// Line style, default is `Solid`.
        pub static LINE_STYLE_VAR: LineStyle = LineStyle::Solid;

        /// Margin around line.
        ///
        /// Is `(4, 0)` by default, 4 top-bottom, 0 left-right.
        pub static MARGIN_VAR: SideOffsets = (4, 0);
    }

    /// Sets the [`COLOR_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(COLOR_VAR))]
    pub fn color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, COLOR_VAR, color)
    }

    /// Sets the [`STROKE_THICKNESS_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(STROKE_THICKNESS_VAR))]
    pub fn stroke_thickness(child: impl UiNode, thickness: impl IntoVar<Length>) -> impl UiNode {
        with_context_var(child, STROKE_THICKNESS_VAR, thickness)
    }

    /// Sets the [`LINE_STYLE_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(LINE_STYLE_VAR))]
    pub fn line_style(child: impl UiNode, style: impl IntoVar<LineStyle>) -> impl UiNode {
        with_context_var(child, LINE_STYLE_VAR, style)
    }

    /// Sets the [`MARGIN_VAR`] that affects all horizontal rules inside the widget.
    #[property(CONTEXT, default(MARGIN_VAR))]
    pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
        with_context_var(child, MARGIN_VAR, margin)
    }
}

/// Vertical rule line.
pub mod vr {
    use zng_wgt::prelude::*;

    /// Draws a vertical [`RuleLine!`](struct@crate::RuleLine).
    #[widget($crate::vr::Vr)]
    pub struct Vr(super::RuleLine);
    impl Vr {
        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                orientation = LineOrientation::Vertical;
                color = COLOR_VAR;
                stroke_thickness = STROKE_THICKNESS_VAR;
                line_style = LINE_STYLE_VAR;
                margin = MARGIN_VAR;
            }
        }
    }

    context_var! {
        /// Line color, inherits from [`FONT_COLOR_VAR`].
        ///
        /// [`FONT_COLOR_VAR`]: zng_wgt_text::FONT_COLOR_VAR
        pub static COLOR_VAR: Rgba = zng_wgt_text::FONT_COLOR_VAR.map(|c| c.with_alpha(30.pct()));

        /// Line stroke thickness, default is `1.dip()`
        pub static STROKE_THICKNESS_VAR: Length = 1.dip();

        /// Line style, default is `Solid`.
        pub static LINE_STYLE_VAR: LineStyle = LineStyle::Solid;

        /// Margin around line.
        ///
        /// Is `(0, 4)` by default, 0 top-bottom, 4 left-right.
        pub static MARGIN_VAR: SideOffsets = (0, 4);
    }

    /// Sets the [`COLOR_VAR`] that affects all vertical rules inside the widget.
    #[property(CONTEXT, default(COLOR_VAR))]
    pub fn color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, COLOR_VAR, color)
    }

    /// Sets the [`STROKE_THICKNESS_VAR`] that affects all vertical rules inside the widget.
    #[property(CONTEXT, default(STROKE_THICKNESS_VAR))]
    pub fn stroke_thickness(child: impl UiNode, thickness: impl IntoVar<Length>) -> impl UiNode {
        with_context_var(child, STROKE_THICKNESS_VAR, thickness)
    }

    /// Sets the [`LINE_STYLE_VAR`] that affects all vertical rules inside the widget.
    #[property(CONTEXT, default(LINE_STYLE_VAR))]
    pub fn line_style(child: impl UiNode, style: impl IntoVar<LineStyle>) -> impl UiNode {
        with_context_var(child, LINE_STYLE_VAR, style)
    }

    /// Sets the [`MARGIN_VAR`] that affects all vertical rules inside the widget.
    #[property(CONTEXT, default(MARGIN_VAR))]
    pub fn margin(child: impl UiNode, margin: impl IntoVar<SideOffsets>) -> impl UiNode {
        with_context_var(child, MARGIN_VAR, margin)
    }
}
