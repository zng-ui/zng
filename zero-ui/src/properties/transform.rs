//! Transform properties, [`scale`](fn@scale), [`rotate`](fn@rotate), [`transform`](fn@transform) and more.

use crate::prelude::new_property::*;

/// Custom transform.
///
/// See [`Transform`] for how to initialize a custom transform. The [`transform_origin`] is applied using the widget's inner size
/// for relative values.
///
/// [`transform_origin`]: fn@transform_origin
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
            let av_size = WIDGET.bounds().inner_size();
            let default_origin = PxPoint::new(av_size.width / 2.0, av_size.height / 2.0);
            let origin = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(av_size), || {
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
/// This property is a shorthand way of setting [`transform`] to [`rotate(angle)`](units::rotate) using variable mapping.
///
/// The rotation is done *around* the [`transform_origin`] in 2D.
///
/// [`transform`]: fn@transform
/// [`transform_origin`]: fn@transform_origin
#[property(LAYOUT, default(0.rad()))]
pub fn rotate(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| units::rotate(a)))
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`rotate_x(angle)`](units::rotate_x) using variable mapping.
///
/// The rotation is done *around* the ***x*** axis that passes trough the [`transform_origin`] in 3D.
///
/// [`transform`]: fn@transform
/// [`transform_origin`]: fn@transform_origin
#[property(LAYOUT, default(0.rad()))]
pub fn rotate_x(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| units::rotate_x(a)))
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`rotate_y(angle)`](units::rotate_y) using variable mapping.
///
/// The rotation is done *around* the ***y*** axis that passes trough the [`transform_origin`] in 3D.
///
/// [`transform`]: fn@transform
/// [`transform_origin`]: fn@transform_origin
#[property(LAYOUT, default(0.rad()))]
pub fn rotate_y(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| units::rotate_y(a)))
}

/// Same as [`rotate`].
///
/// [`rotate`]: fn@rotate
#[property(LAYOUT, default(0.rad()))]
pub fn rotate_z(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, angle.into_var().map(|&a| units::rotate_z(a)))
}

/// Scale transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale(s)`](units::scale) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0))]
pub fn scale(child: impl UiNode, s: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, s.into_var().map(|&x| units::scale(x)))
}

/// Scale X and Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_xy(x, y)`](units::scale) using variable merging.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0, 1.0))]
pub fn scale_xy(child: impl UiNode, x: impl IntoVar<Factor>, y: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::scale_xy(x, y)))
}

/// Scale X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_x(x)`](units::scale_x) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0))]
pub fn scale_x(child: impl UiNode, x: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, x.into_var().map(|&x| units::scale_x(x)))
}

/// Scale Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_y(y)`](units::scale_y) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(1.0))]
pub fn scale_y(child: impl UiNode, y: impl IntoVar<Factor>) -> impl UiNode {
    transform(child, y.into_var().map(|&y| units::scale_y(y)))
}

/// Skew transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew(x, y)`](units::skew) using variable merging.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0.rad(), 0.rad()))]
pub fn skew(child: impl UiNode, x: impl IntoVar<AngleRadian>, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::skew(x, y)))
}

/// Skew X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew_x(x)`](units::skew_x) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0.rad()))]
pub fn skew_x(child: impl UiNode, x: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, x.into_var().map(|&x| units::skew_x(x)))
}

/// Skew Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew_y(y)`](units::skew_y) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT)]
pub fn skew_y(child: impl UiNode, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform(child, y.into_var().map(|&y| units::skew_y(y)))
}

/// Translate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate(x, y)`](units::translate) using variable merging.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0, 0))]
pub fn translate(child: impl UiNode, x: impl IntoVar<Length>, y: impl IntoVar<Length>) -> impl UiNode {
    transform(
        child,
        merge_var!(x.into_var(), y.into_var(), |x, y| units::translate(x.clone(), y.clone())),
    )
}

/// Translate X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_x(x)`](units::translate_x) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0))]
pub fn translate_x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    transform(child, x.into_var().map(|x| units::translate_x(x.clone())))
}

/// Translate Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_y(y)`](units::translate_y) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0))]
pub fn translate_y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    transform(child, y.into_var().map(|y| units::translate_y(y.clone())))
}

/// Translate Z transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_z(z)`](units::translate_z) using variable mapping.
///
/// [`transform`]: fn@transform
#[property(LAYOUT, default(0))]
pub fn translate_z(child: impl UiNode, z: impl IntoVar<Length>) -> impl UiNode {
    transform(child, z.into_var().map(|z| units::translate_z(z.clone())))
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

///Distance from the Z plane (0) the viewer is, used by [`transform`] when it is 3D.
///
/// This property sets the [`PERSPECTIVE_VAR`] context variable.
///
/// [`transform`]: fn@transform
#[property(CONTEXT, default(PERSPECTIVE_VAR))]
pub fn perspective(child: impl UiNode, distance: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, PERSPECTIVE_VAR, distance)
}
/// Vanishing point used by [`transform`] when it is 3D.
///
/// This property sets the [`PERSPECTIVE_ORIGIN_VAR`] context variable.
///
/// [`transform`]: fn@transform
#[property(CONTEXT, default(PERSPECTIVE_ORIGIN_VAR))]
pub fn perspective_origin(child: impl UiNode, origin: impl IntoVar<Point>) -> impl UiNode {
    with_context_var(child, PERSPECTIVE_ORIGIN_VAR, origin)
}

context_var! {
    /// Point relative to the widget inner bounds around which the [`transform`] is applied.
    ///
    /// Default origin is [`Point::center()`].
    ///
    /// [`transform`]: fn@transform
    pub static TRANSFORM_ORIGIN_VAR: Point = Point::center();

    /// Distance from the Z plane (0) the viewer is, used by [`transform`] when it is 3D.
    ///
    /// Default is `1.px()` that is also the minimum.
    ///
    /// [`transform`]: fn@transform
    pub static PERSPECTIVE_VAR: Length = Length::Default;

    /// Vanishing point used by [`transform`] when it is 3D.
    ///
    /// Default origin is [`Point::center()`].
    ///
    /// [`transform`]: fn@transform
    pub static PERSPECTIVE_ORIGIN_VAR: Point = Point::center();
}
