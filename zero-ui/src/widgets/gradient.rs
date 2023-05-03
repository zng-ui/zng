use crate::core::gradient::*;
use crate::prelude::new_widget::*;

/// Node that fills the widget area with a linear gradient defined by angle or points.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a linear gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a Linear gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_linear_gradient(axis: impl IntoVar<LinearGradientAxis>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    linear_gradient_ext(axis, stops, ExtendMode::Reflect)
}
/// Node that fills the widget area with a linear gradient with extend mode configurable.
pub fn linear_gradient_ext(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    let axis = axis.into_var();
    let stops = stops.into_var();
    let extend_mode = extend_mode.into_var();

    let mut render_line = PxLine::zero();
    let mut render_stops = vec![];
    let mut render_size = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&axis).sub_var_layout(&stops).sub_var_layout(&extend_mode);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;
                render_line = axis.layout();

                let length = render_line.length();

                LAYOUT.with_constraints(LAYOUT.constraints().with_new_exact_x(length), || {
                    stops.with(|s| s.layout_linear(true, extend_mode.get(), &mut render_line, &mut render_stops))
                });

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_linear_gradient(
                PxRect::from_size(render_size),
                render_line,
                &render_stops,
                extend_mode.get().into(),
                render_size,
                PxSize::zero(),
            );
        }
        _ => {}
    })
}
/// Node that fills the widget area with a linear gradient with all features configurable.
pub fn linear_gradient_full(
    axis: impl IntoVar<LinearGradientAxis>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    let axis = axis.into_var();
    let stops = stops.into_var();
    let extend_mode = extend_mode.into_var();
    let tile_size = tile_size.into_var();
    let tile_spacing = tile_spacing.into_var();

    let mut render_line = PxLine::zero();
    let mut render_stops = vec![];
    let mut render_size = PxSize::zero();
    let mut render_tile_size = PxSize::zero();
    let mut render_tile_spacing = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&axis)
                .sub_var_layout(&stops)
                .sub_var_layout(&extend_mode)
                .sub_var_layout(&tile_size)
                .sub_var_layout(&tile_spacing);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            let c = LAYOUT.constraints();
            *final_size = c.fill_size();
            if *final_size != render_size {
                render_size = *final_size;

                render_tile_size = tile_size.layout_dft(render_size);
                render_tile_spacing = tile_spacing.layout_dft(render_size);

                render_line = LAYOUT.with_constraints(c.with_exact_size(render_tile_size), || axis.layout());

                let length = render_line.length();
                LAYOUT.with_constraints(c.with_new_exact_x(length), || {
                    stops.with(|s| s.layout_linear(true, extend_mode.get(), &mut render_line, &mut render_stops))
                });

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_linear_gradient(
                PxRect::from_size(render_size),
                render_line,
                &render_stops,
                extend_mode.get().into(),
                render_tile_size,
                render_tile_spacing,
            );
        }
        _ => {}
    })
}

/// Node that fills the widget area with a radial gradient defined by the center point and radius.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a radial gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a radial gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_radial_gradient(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    radial_gradient_ext(center, radius, stops, ExtendMode::Reflect)
}
/// Node that fill the widget area with a radial gradient with extend mode configurable.
pub fn radial_gradient_ext(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    let center = center.into_var();
    let radius = radius.into_var();
    let stops = stops.into_var();
    let extend_mode = extend_mode.into_var();

    let mut render_stops = vec![];
    let mut render_center = PxPoint::zero();
    let mut render_radius = PxSize::zero();
    let mut render_size = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&center)
                .sub_var_layout(&radius)
                .sub_var_layout(&stops)
                .sub_var_layout(&extend_mode);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(render_size), || {
                    render_center = center.layout_dft(render_size.to_vector().to_point() * 0.5.fct());
                    render_radius = radius.get().layout(render_center);
                });

                LAYOUT.with_constraints(
                    LAYOUT.constraints().with_exact_x(render_radius.width.max(render_radius.height)),
                    || stops.with(|s| s.layout_radial(true, extend_mode.get(), &mut render_stops)),
                );

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_radial_gradient(
                PxRect::from_size(render_size),
                render_center,
                render_radius,
                &render_stops,
                extend_mode.get().into(),
                render_size,
                PxSize::zero(),
            );
        }
        _ => {}
    })
}
/// Node that fills the widget area with a radial gradient with all features configurable.
pub fn radial_gradient_full(
    center: impl IntoVar<Point>,
    radius: impl IntoVar<GradientRadius>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    let center = center.into_var();
    let radius = radius.into_var();
    let stops = stops.into_var();
    let extend_mode = extend_mode.into_var();
    let tile_size = tile_size.into_var();
    let tile_spacing = tile_spacing.into_var();

    let mut render_stops = vec![];
    let mut render_center = PxPoint::zero();
    let mut render_radius = PxSize::zero();
    let mut render_size = PxSize::zero();
    let mut render_tile_size = PxSize::zero();
    let mut render_tile_spacing = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&center)
                .sub_var_layout(&radius)
                .sub_var_layout(&stops)
                .sub_var_layout(&extend_mode)
                .sub_var_layout(&tile_size)
                .sub_var_layout(&tile_spacing);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;

                render_tile_size = tile_size.layout_dft(render_size);
                render_tile_spacing = tile_spacing.layout_dft(render_size);

                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(render_tile_size), || {
                    render_center = center.get().layout_dft(render_tile_size.to_vector().to_point() * 0.5.fct());
                    render_radius = radius.get().layout(render_center);
                });

                LAYOUT.with_constraints(
                    LAYOUT.constraints().with_exact_x(render_radius.width.max(render_radius.height)),
                    || stops.with(|s| s.layout_radial(true, extend_mode.get(), &mut render_stops)),
                );

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_radial_gradient(
                PxRect::from_size(render_size),
                render_center,
                render_radius,
                &render_stops,
                extend_mode.get().into(),
                render_tile_size,
                render_tile_spacing,
            );
        }
        _ => {}
    })
}

/// Node that fills the widget area with a conic gradient defined by center point and start angle.
///
/// The extend mode is [`Clamp`](ExtendMode::Clamp).
pub fn conic_gradient(center: impl IntoVar<Point>, angle: impl IntoVar<AngleRadian>, stops: impl IntoVar<GradientStops>) -> impl UiNode {
    conic_gradient_ext(center, angle, stops, ExtendMode::Clamp)
}
/// Node that fills the widget area with a conic gradient with extend mode [`Repeat`](ExtendMode::Repeat).
pub fn repeating_conic_gradient(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    conic_gradient_ext(center, angle, stops, ExtendMode::Repeat)
}
/// Node that fills the widget area with a conic gradient with extend mode [`Reflect`](ExtendMode::Reflect).
pub fn reflecting_conic_gradient(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
) -> impl UiNode {
    conic_gradient_ext(center, angle, stops, ExtendMode::Reflect)
}
/// Node that fill the widget area with a conic gradient with extend mode configurable.
pub fn conic_gradient_ext(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
) -> impl UiNode {
    let center = center.into_var();
    let angle = angle.into_var();
    let stops = stops.into_var();
    let extend_mode = extend_mode.into_var();

    let mut render_stops = vec![];
    let mut render_center = PxPoint::zero();
    let mut render_size = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&center)
                .sub_var_layout(&angle)
                .sub_var_layout(&stops)
                .sub_var_layout(&extend_mode);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;
                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(*final_size), || {
                    render_center = center.layout_dft(final_size.to_vector().to_point() * 0.5.fct());
                });

                let perimeter = Px({
                    let a = final_size.width.0 as f32;
                    let b = final_size.height.0 as f32;
                    std::f32::consts::PI * 2.0 * ((a * a + b * b) / 2.0).sqrt()
                } as _);
                LAYOUT.with_constraints(LAYOUT.constraints().with_exact_x(perimeter), || {
                    stops.with(|s| s.layout_radial(true, extend_mode.get(), &mut render_stops))
                });

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_conic_gradient(
                PxRect::from_size(render_size),
                render_center,
                angle.get(),
                &render_stops,
                extend_mode.get().into(),
                render_size,
                PxSize::zero(),
            );
        }
        _ => {}
    })
}
/// Node that fills the widget area with a conic gradient with all features configurable.
pub fn conic_gradient_full(
    center: impl IntoVar<Point>,
    angle: impl IntoVar<AngleRadian>,
    stops: impl IntoVar<GradientStops>,
    extend_mode: impl IntoVar<ExtendMode>,
    tile_size: impl IntoVar<Size>,
    tile_spacing: impl IntoVar<Size>,
) -> impl UiNode {
    let center = center.into_var();
    let angle = angle.into_var();
    let stops = stops.into_var();
    let extend_mode = extend_mode.into_var();
    let tile_size = tile_size.into_var();
    let tile_spacing = tile_spacing.into_var();

    let mut render_stops = vec![];
    let mut render_center = PxPoint::zero();
    let mut render_size = PxSize::zero();
    let mut render_tile_size = PxSize::zero();
    let mut render_tile_spacing = PxSize::zero();

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&center)
                .sub_var_layout(&angle)
                .sub_var_layout(&stops)
                .sub_var_layout(&extend_mode)
                .sub_var_layout(&tile_size)
                .sub_var_layout(&tile_spacing);
        }
        UiNodeOp::Measure { desired_size, .. } => {
            *desired_size = LAYOUT.constraints().fill_size();
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();
            if *final_size != render_size {
                render_size = *final_size;

                render_tile_size = tile_size.layout_dft(render_size);
                render_tile_spacing = tile_spacing.layout_dft(render_size);

                LAYOUT.with_constraints(PxConstraints2d::new_fill_size(render_tile_size), || {
                    render_center = center.get().layout_dft(render_tile_size.to_vector().to_point() * 0.5.fct());
                });

                let perimeter = Px({
                    let a = render_tile_size.width.0 as f32;
                    let b = render_tile_size.height.0 as f32;
                    std::f32::consts::PI * 2.0 * ((a * a + b * b) / 2.0).sqrt()
                } as _);
                LAYOUT.with_constraints(LAYOUT.constraints().with_exact_x(perimeter), || {
                    stops.with(|s| s.layout_radial(true, extend_mode.get(), &mut render_stops))
                });

                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            frame.push_conic_gradient(
                PxRect::from_size(render_size),
                render_center,
                angle.get(),
                &render_stops,
                extend_mode.get().into(),
                render_tile_size,
                render_tile_spacing,
            );
        }
        _ => {}
    })
}
