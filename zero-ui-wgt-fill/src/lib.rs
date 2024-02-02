#![doc = include_str!("../../zero-ui-app/README.md")]
//!
//! Properties that fill the widget inner bounds and nodes that fill the available space.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zero_ui_wgt::prelude::gradient::{stops, GradientRadius, GradientStops, LinearGradientAxis};
use zero_ui_wgt::{hit_test_mode, node::interactive_node, prelude::*, HitTestMode};

pub mod node;

/// Custom background property. Allows using any other widget as a background.
///
/// Backgrounds are not interactive, but are hit-testable, they don't influence the layout being measured and
/// arranged with the widget size, and they are always clipped to the widget bounds.
///
/// See also [`background_fn`](fn@background_fn) for use in styles.
#[property(FILL)]
pub fn background(child: impl UiNode, background: impl UiNode) -> impl UiNode {
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
/// This is the equivalent of setting [`background`] to the [`presenter`] node, but if the property is cloned
/// in styles the `wgt_fn` will be called multiple times to create duplicates of the background nodes instead
/// of moving the node to the latest widget.
///
/// [`WidgetFn<()>`]: WidgetFn
/// [`background`]: fn@background
#[property(FILL, default(WidgetFn::nil()))]
pub fn background_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    background(child, presenter((), wgt_fn))
}

/// Single color background property.
///
/// This property applies a [`node::flood`] as [`background`].
///
/// [`background`]: fn@background
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    background(child, node::flood(color))
}

/// Linear gradient background property.
///
/// This property applies a [`node::linear_gradient`] as [`background`].
/// 
/// [`background`]: fn@background
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn background_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    background(child, node::linear_gradient(axis, stops))
}

/// Radial gradient background property.
///
/// This property applies a [`node::radial_gradient`] as [`background`].
/// 
/// [`background`]: fn@background
#[property(FILL, default((50.pct(), 50.pct()), 100.pct(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn background_radial(
    child: impl UiNode,
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    background(child, node::radial_gradient(center, radius, stops))
}

/// Conic gradient background property.
///
/// This property applies a [`node::conic_gradient`] as [`background`].
/// 
/// [`background`]: fn@background
#[property(FILL, default((50.pct(), 50.pct()), 0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn background_conic(
    child: impl UiNode,
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    background(child, node::conic_gradient(center, angle, stops))
}

/// Custom foreground fill property. Allows using any other widget as a foreground overlay.
///
/// The foreground is rendered over the widget content and background and under the widget borders.
///
/// Foregrounds are not interactive, not hit-testable and don't influence the widget layout.
#[property(FILL, default(NilUiNode))]
pub fn foreground(child: impl UiNode, foreground: impl UiNode) -> impl UiNode {
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
/// This is the equivalent of setting [`foreground`] to the [`presenter`] node.
///
/// [`WidgetFn<()>`]: WidgetFn
/// [`foreground`]: fn@foreground
#[property(FILL, default(WidgetFn::nil()))]
pub fn foreground_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    foreground(child, presenter((), wgt_fn))
}

/// Foreground highlight border overlay.
///
/// This property draws a border contour with extra `offsets` padding as an overlay.
#[property(FILL, default(0, 0, BorderStyle::Hidden))]
pub fn foreground_highlight(
    child: impl UiNode,
    offsets: impl IntoVar<SideOffsets>,
    widths: impl IntoVar<SideOffsets>,
    sides: impl IntoVar<BorderSides>,
) -> impl UiNode {
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

/// Fill color overlay property.
///
/// This property applies a [`node::flood`] as [`foreground`].
///
/// [`foreground`]: fn@foreground
#[property(FILL, default(colors::BLACK.transparent()))]
pub fn foreground_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    foreground(child, node::flood(color))
}

/// Linear gradient overlay property.
///
/// This property applies a [`node::linear_gradient`] as [`foreground`] using the [`Clamp`] extend mode.
///
/// [`foreground`]: fn@foreground
/// [`Clamp`]: zero_ui_wgt::prelude::gradient::ExtendMode::Clamp
#[property(FILL, default(0.deg(), {
    let c = colors::BLACK.transparent();
    stops![c, c]
}))]
pub fn foreground_gradient(child: impl UiNode, axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    foreground(child, node::linear_gradient(axis, stops))
}
