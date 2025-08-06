#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Properties that fill the widget inner bounds and nodes that fill the available space.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_wgt::prelude::gradient::{GradientRadius, GradientStops, LinearGradientAxis, stops};
use zng_wgt::{HitTestMode, hit_test_mode, node::interactive_node, prelude::*};

pub mod node;

/// Custom background. Allows using any other UI node as a background.
///
/// The `background` is not interactive, it is hit-testable only as a visual of the widget. The background
/// is layout to fill the widget, it does not affect the size of the widget.
///
/// Note that nodes can only exist in a single place in the UI tree at a time, so if you set this property
/// in a style the background node will only appear in the last widget that uses the style, the
/// [`background_fn`](fn@background_fn) property does not have this issue.
#[property(FILL)]
pub fn background(child: impl IntoUiNode, background: impl IntoUiNode) -> UiNode {
    let background = interactive_node(background, false);
    let background = fill_node(background);

    match_node_list(ui_vec![background, child], |children, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = children.with_node(1, |n| n.measure(wm));
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = children.with_node(1, |n| n.layout(wl));

            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || {
                children.with_node(0, |n| n.layout(wl));
            });
            *final_size = size;
        }
        _ => {}
    })
}

/// Custom background generated using a [`WidgetFn<()>`].
///
/// This is the equivalent of setting [`background`] to the [`presenter`] node, but if the property is
/// set in a style that is used by multiple widgets at the same time the `wgt_fn` will be called for each widget
/// to create duplicates of the background nodes instead of moving the node to the last widget.
///
/// [`WidgetFn<()>`]: WidgetFn
/// [`background`]: fn@background
/// [`presenter`]: zng_wgt::prelude::presenter
#[property(FILL, default(WidgetFn::nil()))]
pub fn background_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> UiNode {
    background(child, presenter((), wgt_fn))
}

/// Fill color background.
///
/// This property applies a [`node::flood`] as [`background`].
///
/// [`background`]: fn@background
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
    background(child, node::flood(color))
}

/// Linear gradient background.
///
/// This property applies a [`node::linear_gradient`] as [`background`].
///
/// [`background`]: fn@background
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn background_gradient(child: impl IntoUiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> UiNode {
    background(child, node::linear_gradient(axis, stops))
}

/// Radial gradient background.
///
/// This property applies a [`node::radial_gradient`] as [`background`].
///
/// [`background`]: fn@background
#[property(FILL, default((50.pct(), 50.pct()), 100.pct(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn background_radial(
    child: impl IntoUiNode,
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> UiNode {
    background(child, node::radial_gradient(center, radius, stops))
}

/// Conic gradient background.
///
/// This property applies a [`node::conic_gradient`] as [`background`].
///
/// [`background`]: fn@background
#[property(FILL, default((50.pct(), 50.pct()), 0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn background_conic(
    child: impl IntoUiNode,
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> UiNode {
    background(child, node::conic_gradient(center, angle, stops))
}

/// Custom foreground fill. Allows using any other UI node as a foreground overlay.
///
/// The `foreground` is not interactive and not hit-testable.
/// The foreground is layout to fill the widget, it does not affect the size of the widget. It is rendered over
/// the widget child and background, it is rendered under borders by default.
///
/// Note that nodes can only exist in a single place in the UI tree at a time, so if you set this property in a style
/// the foreground node will only appear in the last widget that uses the style, the [`foreground_fn`] property does not have this issue.
///
/// [`foreground_fn`]: fn@foreground_fn
#[property(FILL, default(UiNode::nil()))]
pub fn foreground(child: impl IntoUiNode, foreground: impl IntoUiNode) -> UiNode {
    let foreground = interactive_node(foreground, false);
    let foreground = fill_node(foreground);
    let foreground = hit_test_mode(foreground, HitTestMode::Disabled);

    match_node_list(ui_vec![child, foreground], |children, op| match op {
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = children.with_node(0, |n| n.measure(wm));
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = children.with_node(0, |n| n.layout(wl));
            LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || {
                children.with_node(1, |n| n.layout(wl));
            });
            *final_size = size;
        }
        _ => {}
    })
}

/// Custom foreground generated using a [`WidgetFn<()>`].
///
/// This is the equivalent of setting [`foreground`] to the [`presenter`] node, but if the property is set in a style that is used
/// by multiple widgets at the same time the `wgt_fn` will be called for each widget to create duplicates of the foreground nodes
/// instead of moving the node to the last widget.
///
/// [`WidgetFn<()>`]: WidgetFn
/// [`foreground`]: fn@foreground
/// [`presenter`]: zng_wgt::prelude::presenter
#[property(FILL, default(WidgetFn::nil()))]
pub fn foreground_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> UiNode {
    foreground(child, presenter((), wgt_fn))
}

/// Foreground highlight border overlay.
///
/// This property draws a border contour overlay that can be positioned using `offsets`.
#[property(FILL, default(0, 0, BorderStyle::Hidden))]
pub fn foreground_highlight(
    child: impl IntoUiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> UiNode {
    let offsets = offsets.into_var();
    let widths = widths.into_var();
    let sides = sides.into_var();

    let mut render_bounds = PxRect::zero();
    let mut render_widths = PxSideOffsets::zero();
    let mut render_radius = PxCornerRadius::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&offsets).sub_var_layout(&widths).sub_var_render(&sides);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let radius = BORDER.inner_radius();
            let offsets = offsets.layout();
            let radius = radius.deflate(offsets);

            let mut bounds = PxRect::zero();
            if let Some(inline) = wl.inline() {
                if let Some(first) = inline.rows.iter().find(|r| !r.size.is_empty()) {
                    bounds = *first;
                }
            }
            if bounds.size.is_empty() {
                let border_offsets = BORDER.inner_offsets();

                bounds = PxRect::new(
                    PxPoint::new(offsets.left + border_offsets.left, offsets.top + border_offsets.top),
                    size - PxSize::new(offsets.horizontal(), offsets.vertical()),
                );
            }

            let widths = LAYOUT.with_constraints(PxConstraints2d::new_exact_size(size), || widths.layout());

            if render_bounds != bounds || render_widths != widths || render_radius != radius {
                render_bounds = bounds;
                render_widths = widths;
                render_radius = radius;
                WIDGET.render();
            }

            *final_size = size;
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);
            frame.push_border(render_bounds, render_widths, sides.get(), render_radius);
        }
        _ => {}
    })
}

/// Fill color overlay.
///
/// This property applies a [`node::flood`] as [`foreground`].
///
/// [`foreground`]: fn@foreground
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn foreground_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
    foreground(child, node::flood(color))
}

/// Linear gradient overlay.
///
/// This property applies a [`node::linear_gradient`] as [`foreground`].
///
/// [`foreground`]: fn@foreground
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn foreground_gradient(child: impl IntoUiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> UiNode {
    foreground(child, node::linear_gradient(axis, stops))
}

/// Radial gradient foreground.
///
/// This property applies a [`node::radial_gradient`] as [`foreground`].
///
/// [`foreground`]: fn@foreground
#[property(FILL, default((50.pct(), 50.pct()), 100.pct(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn foreground_radial(
    child: impl IntoUiNode,
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> UiNode {
    foreground(child, node::radial_gradient(center, radius, stops))
}

/// Conic gradient foreground.
///
/// This property applies a [`node::conic_gradient`] as [`foreground`].
///
/// [`foreground`]: fn@foreground
#[property(FILL, default((50.pct(), 50.pct()), 0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn foreground_conic(
    child: impl IntoUiNode,
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> UiNode {
    foreground(child, node::conic_gradient(center, angle, stops))
}
