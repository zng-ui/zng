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

pub mod hr;
pub mod vr;

mod collapse;
pub use collapse::*;

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
#[property(CONTEXT, default(LineOrientation::Horizontal), widget_impl(RuleLine))]
pub fn orientation(wgt: &mut WidgetBuilding, orientation: impl IntoVar<LineOrientation>) {
    let _ = orientation;
    wgt.expect_property_capture();
}

/// Line color.
#[property(CONTEXT, default(rgb(0, 0, 0)), widget_impl(RuleLine))]
pub fn color(wgt: &mut WidgetBuilding, color: impl IntoVar<Rgba>) {
    let _ = color;
    wgt.expect_property_capture();
}

/// Line stroke thickness.
#[property(CONTEXT, default(1), widget_impl(RuleLine))]
pub fn stroke_thickness(wgt: &mut WidgetBuilding, thickness: impl IntoVar<Length>) {
    let _ = thickness;
    wgt.expect_property_capture();
}

/// Line length.
///
/// Set to [`Default`] to fill available length.
///
/// [`Default`]: Length::Default
#[property(CONTEXT, default(Length::Default), widget_impl(RuleLine))]
pub fn length(wgt: &mut WidgetBuilding, length: impl IntoVar<Length>) {
    let _ = length;
    wgt.expect_property_capture();
}

/// Line style.
#[property(CONTEXT, default(LineStyle::Solid), widget_impl(RuleLine))]
pub fn line_style(wgt: &mut WidgetBuilding, style: impl IntoVar<LineStyle>) {
    let _ = style;
    wgt.expect_property_capture();
}

fn on_build(wgt: &mut WidgetBuilding) {
    let mut bounds = PxSize::zero();

    let orientation = wgt
        .capture_var(property_id!(orientation))
        .unwrap_or_else(|| LineOrientation::Horizontal.into_var());

    let length = wgt.capture_var(property_id!(length)).unwrap_or_else(|| const_var(Length::Default));

    let stroke_thickness = wgt
        .capture_var(property_id!(stroke_thickness))
        .unwrap_or_else(|| const_var(Length::from(1)));

    let color = wgt.capture_var(property_id!(color)).unwrap_or_else(|| const_var(rgb(0, 0, 0)));

    let style = wgt
        .capture_var(property_id!(line_style))
        .unwrap_or_else(|| LineStyle::Solid.into_var());

    wgt.set_child(match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&stroke_thickness)
                .sub_var_layout(&orientation)
                .sub_var_layout(&length)
                .sub_var_render(&color)
                .sub_var_render(&style);
        }
        UiNodeOp::Info { info } => {
            info.flag_meta(*COLLAPSABLE_LINE_ID);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            if COLLAPSE_SCOPE.collapse(WIDGET.id()) {
                *desired_size = PxSize::zero();
                return;
            }

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
        UiNodeOp::Layout { final_size, wl } => {
            if COLLAPSE_SCOPE.collapse(WIDGET.id()) {
                wl.collapse();
                *final_size = PxSize::zero();
                return;
            }

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
            if bounds.is_empty() {
                return;
            }
            let bounds = PxRect::from_size(bounds);
            let orientation = orientation.get();
            let color = color.get();
            let style = style.get();
            frame.push_line(bounds, orientation, color, style);
        }
        _ => {}
    }));
}
