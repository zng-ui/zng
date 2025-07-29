#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Transform properties, [`scale`](fn@scale), [`rotate`](fn@rotate), [`transform`](fn@transform) and more.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use zng_wgt::prelude::*;

/// Custom transform.
///
/// See [`Transform`] for how to initialize a custom transform. The [`transform_origin`] is applied using the widget's inner size
/// for relative values.
///
/// [`transform_origin`]: fn@transform_origin
/// [`Transform`]: zng_wgt::prelude::Transform
#[property(LAYOUT, default(Transform::identity()))]
pub fn transform(child: impl UiNode, transform: impl IntoVar<Transform>) -> impl UiNode {
    let binding_key = FrameValueKey::new_unique();
    let transform = transform.into_var();
    let mut render_transform = PxTransform::identity();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&transform);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = child.layout(wl);

            let transform = transform.layout();

            let default_origin = PxPoint::new(size.width / 2.0, size.height / 2.0);
            let origin = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(size), || {
                TRANSFORM_ORIGIN_VAR.layout_dft(default_origin)
            });

            let x = origin.x.0 as f32;
            let y = origin.y.0 as f32;
            let transform = PxTransform::translation(-x, -y).then(&transform).then_translate(euclid::vec2(x, y));

            if transform != render_transform {
                render_transform = transform;
                WIDGET.render_update();
            }

            *final_size = size;
        }
        UiNodeOp::Render { frame } => {
            if frame.is_outer() {
                frame.push_inner_transform(&render_transform, |frame| child.render(frame));
            } else {
                frame.push_reference_frame(
                    binding_key.into(),
                    binding_key.bind_var_mapped(&transform, render_transform),
                    false,
                    false,
                    |frame| child.render(frame),
                );
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            if update.is_outer() {
                update.with_inner_transform(&render_transform, |update| child.render_update(update));
            } else {
                update.with_transform_opt(binding_key.update_var_mapped(&transform, render_transform), false, |update| {
                    child.render_update(update)
                })
            }
        }
        _ => {}
    })
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_rotate(angle)`](Transform::new_rotate) using variable mapping.
///
/// The rotation is done *around* the [`transform_origin`] in 2D.
///
/// [`transform`]: fn@transform
/// [`transform_origin`]: fn@transform_origin
#[property(LAYOUT, default(0.rad()))]
pub fn rotate(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| Transform::new_rotate(a)))
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_rotate_x(angle)`](Transform::new_rotate_x) using variable mapping.
///
/// The rotation is done *around* the ***x*** axis that passes trough the [`transform_origin`] in 3D.
///
/// [`transform`]: fn@transform
/// [`transform_origin`]: fn@transform_origin
#[property(LAYOUT, default(0.rad()))]
pub fn rotate_x(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| Transform::new_rotate_x(a)))
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_rotate_y(angle)`](Transform::new_rotate_y) using variable mapping.
///
/// The rotation is done *around* the ***y*** axis that passes trough the [`transform_origin`] in 3D.
///
/// [`transform`]: fn@transform
/// [`transform_origin`]: fn@transform_origin
#[property(LAYOUT, default(0.rad()))]
pub fn rotate_y(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| Transform::new_rotate_y(a)))
}

/// Same as [`rotate`].
///
/// [`rotate`]: fn@rotate
#[property(LAYOUT, default(0.rad()))]
pub fn rotate_z(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| Transform::new_rotate_z(a)))
}

/// Scale transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_scale(s)`](Transform::new_scale) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0))]
pub fn scale(child: impl UiNode, s: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, s.into_var().map(|&x| Transform::new_scale(x)))
}

/// Scale X and Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_scale_xy(x, y)`](Transform::new_scale) using variable merging.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0, 1.0))]
pub fn scale_xy(child: impl UiNode, x: impl IntoVar<Factor>, y: impl IntoVar<Factor>) -> impl UiNode {
    transform(
        child,
        merge_var!(x.into_var(), y.into_var(), |&x, &y| Transform::new_scale_xy(x, y)),
    )
}

/// Scale X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_scale_x(x)`](Transform::new_scale_x) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0))]
pub fn scale_x(child: impl UiNode, x: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, x.into_var().map(|&x| Transform::new_scale_x(x)))
}

/// Scale Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_scale_y(y)`](Transform::new_scale_y) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0))]
pub fn scale_y(child: impl UiNode, y: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, y.into_var().map(|&y| Transform::new_scale_y(y)))
}

/// Skew transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_skew(x, y)`](Transform::new_skew) using variable merging.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0.rad(), 0.rad()))]
pub fn skew(child: impl UiNode, x: impl IntoVar<AngleRadian>, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| Transform::new_skew(x, y)))
}

/// Skew X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_skew_x(x)`](Transform::new_skew_x) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0.rad()))]
pub fn skew_x(child: impl UiNode, x: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, x.into_var().map(|&x| Transform::new_skew_x(x)))
}

/// Skew Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_skew_y(y)`](Transform::new_skew_y) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT)]
pub fn skew_y(child: impl UiNode, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, y.into_var().map(|&y| Transform::new_skew_y(y)))
}

/// Translate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_translate(x, y)`](Transform::new_translate) using variable merging.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0, 0))]
pub fn translate(child: impl UiNode, x: impl IntoVar<Length>, y: impl IntoVar<Length>) -> impl UiNode {
    transform(
        child,
        merge_var!(x.into_var(), y.into_var(), |x, y| Transform::new_translate(x.clone(), y.clone())),
    )
}

/// Translate X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_translate_x(x)`](Transform::new_translate_x) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0))]
pub fn translate_x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    transform(child, x.into_var().map(|x| Transform::new_translate_x(x.clone())))
}

/// Translate Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_translate_y(y)`](Transform::new_translate_y) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0))]
pub fn translate_y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    transform(child, y.into_var().map(|y| Transform::new_translate_y(y.clone())))
}

/// Translate Z transform.
///
/// This property is a shorthand way of setting [`transform`] to [`new_translate_z(z)`](Transform::new_translate_z) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0))]
pub fn translate_z(child: impl UiNode, z: impl IntoVar<Length>) -> impl UiNode {
    transform(child, z.into_var().map(|z| Transform::new_translate_z(z.clone())))
}

/// Point relative to the widget inner bounds around which the [`transform`] is applied.
///
/// This property sets the [`TRANSFORM_ORIGIN_VAR`] context variable.
///
/// [`transform`]: fn@transform
#[property(CONTEXT, default(TRANSFORM_ORIGIN_VAR))]
pub fn transform_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    with_context_var(child, TRANSFORM_ORIGIN_VAR, origin)
}

///Distance from the Z plane (0) the viewer is, affects 3D transform on the widget's children.
///
/// [`Length::Default`] is an infinite distance, the lower the value the *closest* the viewer is and therefore
/// the 3D transforms are more noticeable. Distances less then `1.px()` are coerced to it.
///
/// [`Length::Default`]: zng_wgt::prelude::Length::Default
#[property(LAYOUT-20, default(Length::Default))]
pub fn perspective(child: impl UiNode, distance: impl IntoVar<Length>) -> impl UiNode {
    let distance = distance.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&distance);
        }
        UiNodeOp::Layout { wl, .. } => {
            let d = distance.layout_dft_z(Px::MAX);
            let d = LAYOUT.z_constraints().clamp(d).max(Px(1));
            wl.set_perspective(d.0 as f32);
        }
        _ => {}
    })
}

/// Vanishing point used by 3D transforms in the widget's children.
///
/// Is the widget center by default.
///
/// [`transform`]: fn@transform
#[property(LAYOUT-20, default(Point::default()))]
pub fn perspective_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    let origin = origin.into_var();

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&origin);
        }
        UiNodeOp::Layout { wl, final_size } => {
            let size = c.layout(wl);
            let default_origin = PxPoint::new(size.width / 2.0, size.height / 2.0);
            let origin = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(size), || origin.layout_dft(default_origin));
            wl.set_perspective_origin(origin);

            *final_size = size;
        }
        _ => {}
    })
}

/// Defines how the widget and children are positioned in 3D space.
///
/// This sets the style for the widget and children layout transform, the [`transform`] and other properties derived from [`transform`].
/// It does not affect any other descendant, only the widget and immediate children.
///
/// [`transform`]: fn@transform
#[property(CONTEXT, default(TransformStyle::Flat))]
pub fn transform_style(child: impl UiNode, style: impl IntoVar<TransformStyle>) -> impl UiNode {
    let style = style.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&style);
        }
        UiNodeOp::Layout { wl, .. } => {
            wl.set_transform_style(style.get());
        }
        _ => {}
    })
}

/// Sets if the widget is still visible when it is turned back towards the viewport due to rotations in X or Y axis in
/// the widget or in parent widgets.
///
/// Widget back face is visible by default, the back face is a mirror image of the front face, if `visible` is set
/// to `false` the widget is still layout and rendered, but it is not displayed on screen by the view-process if
/// the final global transform of the widget turns the backface towards the viewport.
///
/// This property affects any descendant widgets too, unless they also set `backface_visibility`.
#[property(CONTEXT, default(true))]
pub fn backface_visibility(child: impl UiNode, visible: impl IntoVar<bool>) -> impl UiNode {
    let visible = visible.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&visible);
        }
        UiNodeOp::Render { frame } => {
            frame.with_backface_visibility(visible.get(), |frame| c.render(frame));
        }
        _ => {}
    })
}

context_var! {
    /// Point relative to the widget inner bounds around which the [`transform`] is applied.
    ///
    /// Default origin is `Point::center`.
    ///
    /// [`transform`]: fn@transform
    pub static TRANSFORM_ORIGIN_VAR: Point = Point::center();

    /// Vanishing point used by [`transform`] when it is 3D.
    ///
    /// Default origin is `Point::center`.
    ///
    /// [`transform`]: fn@transform
    pub static PERSPECTIVE_ORIGIN_VAR: Point = Point::center();
}
