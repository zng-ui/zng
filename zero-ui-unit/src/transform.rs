use serde::{Deserialize, Serialize};

use std::marker::PhantomData;

use crate::{Px, PxBox, PxPoint, PxVector};

/// Radian angle type used by webrender.
pub type RenderAngle = euclid::Angle<f32>;

/// A transform in device pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PxTransform {
    /// Simple offset.
    Offset(euclid::Vector2D<f32, Px>),
    /// Full transform.
    #[serde(with = "serde_px_transform3d")]
    Transform(euclid::Transform3D<f32, Px, Px>),
}

impl PartialEq for PxTransform {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Offset(l0), Self::Offset(r0)) => l0 == r0,
            (Self::Transform(l0), Self::Transform(r0)) => l0 == r0,
            (a, b) => a.is_identity() && b.is_identity() || a.to_transform() == b.to_transform(),
        }
    }
}
impl Default for PxTransform {
    /// Identity.
    fn default() -> Self {
        Self::identity()
    }
}
impl PxTransform {
    /// Identity transform.
    pub fn identity() -> Self {
        PxTransform::Offset(euclid::vec2(0.0, 0.0))
    }

    /// New simple 2D translation.
    pub fn translation(x: f32, y: f32) -> Self {
        PxTransform::Offset(euclid::vec2(x, y))
    }

    /// New 3D translation.
    pub fn translation_3d(x: f32, y: f32, z: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::translation(x, y, z))
    }

    /// New 2D rotation.
    pub fn rotation(x: f32, y: f32, theta: RenderAngle) -> Self {
        Self::rotation_3d(x, y, 1.0, theta)
    }

    /// New 3D rotation.
    pub fn rotation_3d(x: f32, y: f32, z: f32, theta: RenderAngle) -> Self {
        let [x, y, z] = euclid::vec3::<_, ()>(x, y, z).normalize().to_array();
        PxTransform::Transform(euclid::Transform3D::rotation(x, y, z, theta))
    }

    /// New 2D skew.
    pub fn skew(alpha: RenderAngle, beta: RenderAngle) -> Self {
        PxTransform::Transform(euclid::Transform3D::skew(alpha, beta))
    }

    /// New 2D scale.
    pub fn scale(x: f32, y: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::scale(x, y, 1.0))
    }

    /// New 3D scale.
    pub fn scale_3d(x: f32, y: f32, z: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::scale(x, y, z))
    }

    /// New 3D perspective distance.
    pub fn perspective(d: f32) -> Self {
        PxTransform::Transform(euclid::Transform3D::perspective(d))
    }

    /// To full transform.
    pub fn to_transform(self) -> euclid::Transform3D<f32, Px, Px> {
        match self {
            PxTransform::Offset(v) => euclid::Transform3D::translation(v.x, v.y, 0.0),
            PxTransform::Transform(t) => t,
        }
    }

    /// Returns `true` is is the identity transform.
    pub fn is_identity(&self) -> bool {
        match self {
            PxTransform::Offset(offset) => offset == &euclid::Vector2D::zero(),
            PxTransform::Transform(transform) => transform == &euclid::Transform3D::identity(),
        }
    }

    /// Returns the multiplication of the two matrices such that mat's transformation
    /// applies after self's transformation.
    #[must_use]
    pub fn then(&self, other: &PxTransform) -> PxTransform {
        match (self, other) {
            (PxTransform::Offset(a), PxTransform::Offset(b)) => PxTransform::Offset(*a + *b),
            (PxTransform::Offset(a), PxTransform::Transform(b)) => {
                PxTransform::Transform(euclid::Transform3D::translation(a.x, a.y, 0.0).then(b))
            }
            (PxTransform::Transform(a), PxTransform::Offset(b)) => PxTransform::Transform(a.then_translate(b.to_3d())),
            (PxTransform::Transform(a), PxTransform::Transform(b)) => PxTransform::Transform(a.then(b)),
        }
    }

    /// Returns a transform with a translation applied after self's transformation.
    #[must_use]
    pub fn then_translate(&self, offset: euclid::Vector2D<f32, Px>) -> PxTransform {
        match self {
            PxTransform::Offset(a) => PxTransform::Offset(*a + offset),
            PxTransform::Transform(a) => PxTransform::Transform(a.then_translate(offset.to_3d())),
        }
    }

    /// Returns a transform with a translation applied before self's transformation.
    #[must_use]
    pub fn pre_translate(&self, offset: euclid::Vector2D<f32, Px>) -> PxTransform {
        match self {
            PxTransform::Offset(b) => PxTransform::Offset(offset + *b),
            PxTransform::Transform(b) => PxTransform::Transform(euclid::Transform3D::translation(offset.x, offset.y, 0.0).then(b)),
        }
    }

    /// Returns whether it is possible to compute the inverse transform.
    pub fn is_invertible(&self) -> bool {
        match self {
            PxTransform::Offset(_) => true,
            PxTransform::Transform(t) => t.is_invertible(),
        }
    }

    /// Returns the inverse transform if possible.
    pub fn inverse(&self) -> Option<PxTransform> {
        match self {
            PxTransform::Offset(v) => Some(PxTransform::Offset(-*v)),
            PxTransform::Transform(t) => t.inverse().map(PxTransform::Transform),
        }
    }

    /// Returns `true` if this transform can be represented with a `Transform2D`.
    pub fn is_2d(&self) -> bool {
        match self {
            PxTransform::Offset(_) => true,
            PxTransform::Transform(t) => t.is_2d(),
        }
    }

    /// Transform the pixel point.
    ///
    /// Note that if the transform is 3D the point will be transformed with z=0, you can
    /// use [`project_point`] to find the 2D point in the 3D z-plane represented by the 3D
    /// transform.
    ///
    /// [`project_point`]: Self::project_point
    pub fn transform_point(&self, point: PxPoint) -> Option<PxPoint> {
        self.transform_point_f32(point.cast()).map(|p| p.cast())
    }

    /// Transform the pixel point.
    ///
    /// Note that if the transform is 3D the point will be transformed with z=0, you can
    /// use [`project_point_f32`] to find the 2D point in the 3D z-plane represented by the 3D
    /// transform.
    ///
    /// [`project_point_f32`]: Self::project_point_f32
    pub fn transform_point_f32(&self, point: euclid::Point2D<f32, Px>) -> Option<euclid::Point2D<f32, Px>> {
        match self {
            PxTransform::Offset(v) => Some(point + *v),
            PxTransform::Transform(t) => t.transform_point2d(point),
        }
    }

    /// Transform the pixel vector.
    pub fn transform_vector(&self, vector: PxVector) -> PxVector {
        self.transform_vector_f32(vector.cast()).cast()
    }

    /// Transform the pixel vector.
    pub fn transform_vector_f32(&self, vector: euclid::Vector2D<f32, Px>) -> euclid::Vector2D<f32, Px> {
        match self {
            PxTransform::Offset(v) => vector + *v,
            PxTransform::Transform(t) => t.transform_vector2d(vector),
        }
    }

    /// Project the 2D point onto the transform Z-plane.
    pub fn project_point(&self, point: PxPoint) -> Option<PxPoint> {
        self.project_point_f32(point.cast()).map(|p| p.cast())
    }

    /// Project the 2D point onto the transform Z-plane.
    pub fn project_point_f32(&self, point: euclid::Point2D<f32, Px>) -> Option<euclid::Point2D<f32, Px>> {
        match self {
            PxTransform::Offset(v) => Some(point + *v),
            PxTransform::Transform(t) => {
                // source: https://github.com/servo/webrender/blob/master/webrender/src/util.rs#L1181

                // Find a value for z that will transform to 0.

                // The transformed value of z is computed as:
                // z' = point.x * self.m13 + point.y * self.m23 + z * self.m33 + self.m43

                // Solving for z when z' = 0 gives us:
                let z = -(point.x * t.m13 + point.y * t.m23 + t.m43) / t.m33;

                t.transform_point3d(euclid::point3(point.x, point.y, z))
                    .map(|p3| euclid::point2(p3.x, p3.y))
            }
        }
    }

    /// Returns a 2D box that encompasses the result of transforming the given box by this
    /// transform, if the transform makes sense for it, or `None` otherwise.
    pub fn outer_transformed(&self, px_box: PxBox) -> Option<PxBox> {
        self.outer_transformed_f32(px_box.cast()).map(|p| p.cast())
    }

    /// Returns a 2D box that encompasses the result of transforming the given box by this
    /// transform, if the transform makes sense for it, or `None` otherwise.
    pub fn outer_transformed_f32(&self, px_box: euclid::Box2D<f32, Px>) -> Option<euclid::Box2D<f32, Px>> {
        match self {
            PxTransform::Offset(v) => {
                let v = *v;
                let mut r = px_box;
                r.min += v;
                r.max += v;
                Some(r)
            }
            PxTransform::Transform(t) => t.outer_transformed_box2d(&px_box),
        }
    }
}

impl From<euclid::Vector2D<f32, Px>> for PxTransform {
    fn from(offset: euclid::Vector2D<f32, Px>) -> Self {
        PxTransform::Offset(offset)
    }
}
impl From<PxVector> for PxTransform {
    fn from(offset: PxVector) -> Self {
        PxTransform::Offset(offset.cast())
    }
}
impl From<euclid::Transform3D<f32, Px, Px>> for PxTransform {
    fn from(transform: euclid::Transform3D<f32, Px, Px>) -> Self {
        PxTransform::Transform(transform)
    }
}

/// euclid does skip the _unit
mod serde_px_transform3d {
    use crate::Px;

    use super::*;
    use serde::*;

    #[derive(Serialize, Deserialize)]
    struct SerdeTransform3D {
        pub m11: f32,
        pub m12: f32,
        pub m13: f32,
        pub m14: f32,
        pub m21: f32,
        pub m22: f32,
        pub m23: f32,
        pub m24: f32,
        pub m31: f32,
        pub m32: f32,
        pub m33: f32,
        pub m34: f32,
        pub m41: f32,
        pub m42: f32,
        pub m43: f32,
        pub m44: f32,
    }

    pub fn serialize<S: Serializer>(t: &euclid::Transform3D<f32, Px, Px>, serializer: S) -> Result<S::Ok, S::Error> {
        SerdeTransform3D {
            m11: t.m11,
            m12: t.m12,
            m13: t.m13,
            m14: t.m14,
            m21: t.m21,
            m22: t.m22,
            m23: t.m23,
            m24: t.m24,
            m31: t.m31,
            m32: t.m32,
            m33: t.m33,
            m34: t.m34,
            m41: t.m41,
            m42: t.m42,
            m43: t.m43,
            m44: t.m44,
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<euclid::Transform3D<f32, Px, Px>, D::Error> {
        let t = SerdeTransform3D::deserialize(deserializer)?;
        Ok(euclid::Transform3D {
            m11: t.m11,
            m12: t.m12,
            m13: t.m13,
            m14: t.m14,
            m21: t.m21,
            m22: t.m22,
            m23: t.m23,
            m24: t.m24,
            m31: t.m31,
            m32: t.m32,
            m33: t.m33,
            m34: t.m34,
            m41: t.m41,
            m42: t.m42,
            m43: t.m43,
            m44: t.m44,
            _unit: PhantomData,
        })
    }
}
