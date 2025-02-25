use super::{Px, euclid};

use zng_var::{
    animation::{Transitionable, easing::EasingStep},
    impl_from_and_into_var,
    types::{is_slerp_enabled, slerp_enabled},
};

use super::{AngleRadian, AngleUnits, Factor, FactorUnits, Layout1d, Length, PxTransform};

/// A transform builder type.
///
/// # Builder
///
/// The transform can be started by one of `Transform::new_*` associated functions or [`Transform::identity`]. More
/// transforms can be chained by calling the methods for each.
///
/// # Examples
///
/// Create a transform that
///
/// ```
/// # use zng_layout::unit::*;
/// let rotate_then_move = Transform::new_rotate(10.deg()).translate(50, 30);
/// ```
#[derive(Clone, Default, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transform {
    parts: Vec<TransformPart>,
    needs_layout: bool,
    lerp_to: Vec<(Self, EasingStep, bool)>,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum TransformPart {
    Computed(PxTransform),
    Translate(Length, Length),
    Translate3d(Length, Length, Length),
    Perspective(Length),
}

impl Transform {
    /// No transform.
    pub fn identity() -> Self {
        Self::default()
    }

    /// Create a 2d rotation transform.
    pub fn new_rotate<A: Into<AngleRadian>>(angle: A) -> Transform {
        Transform::identity().rotate(angle)
    }

    /// Create a 3d rotation transform around the ***x*** axis.
    pub fn new_rotate_x<A: Into<AngleRadian>>(angle: A) -> Transform {
        Transform::identity().rotate_x(angle)
    }

    /// Create a 3d rotation transform around the ***y*** axis.
    pub fn new_rotate_y<A: Into<AngleRadian>>(angle: A) -> Transform {
        Transform::identity().rotate_y(angle)
    }

    /// Same as `new_rotate`.
    pub fn new_rotate_z<A: Into<AngleRadian>>(angle: A) -> Transform {
        Transform::identity().rotate_z(angle)
    }

    /// Create a 3d rotation transform.
    pub fn new_rotate_3d<A: Into<AngleRadian>>(x: f32, y: f32, z: f32, angle: A) -> Transform {
        Transform::identity().rotate_3d(x, y, z, angle)
    }

    /// Create a 2d translation transform.
    pub fn new_translate<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Transform {
        Transform::identity().translate(x, y)
    }

    /// Create a 3d translation transform.
    pub fn new_translate_3d<X: Into<Length>, Y: Into<Length>, Z: Into<Length>>(x: X, y: Y, z: Z) -> Transform {
        Transform::identity().translate_3d(x, y, z)
    }

    /// Create a 2d translation transform in the X dimension.
    pub fn new_translate_x<X: Into<Length>>(x: X) -> Transform {
        Transform::new_translate(x, 0.0)
    }

    /// Create a 2d translation transform in the Y dimension.
    pub fn new_translate_y<Y: Into<Length>>(y: Y) -> Transform {
        Transform::new_translate(0.0, y)
    }

    /// Create a 3d translation transform in the z dimension.
    pub fn new_translate_z<Z: Into<Length>>(z: Z) -> Transform {
        Transform::new_translate_3d(0.0, 0.0, z)
    }

    /// Create a 3d perspective transform.
    pub fn new_perspective<D: Into<Length>>(d: D) -> Transform {
        Transform::identity().perspective(d)
    }

    /// Create a 2d skew transform.
    pub fn new_skew<X: Into<AngleRadian>, Y: Into<AngleRadian>>(x: X, y: Y) -> Transform {
        Transform::identity().skew(x, y)
    }

    /// Create a 2d skew transform in the X dimension.
    pub fn new_skew_x<X: Into<AngleRadian>>(x: X) -> Transform {
        Transform::new_skew(x, 0.rad())
    }

    /// Create a 2d skew transform in the Y dimension.
    pub fn new_skew_y<Y: Into<AngleRadian>>(y: Y) -> Transform {
        Transform::new_skew(0.rad(), y)
    }

    /// Create a 2d scale transform.
    ///
    /// The same `scale` is applied to both dimensions.
    pub fn new_scale<S: Into<Factor>>(scale: S) -> Transform {
        let scale = scale.into();
        Transform::new_scale_xy(scale, scale)
    }

    /// Create a 2d scale transform on the X dimension.
    pub fn new_scale_x<X: Into<Factor>>(x: X) -> Transform {
        Transform::new_scale_xy(x, 1.0)
    }

    /// Create a 2d scale transform on the Y dimension.
    pub fn new_scale_y<Y: Into<Factor>>(y: Y) -> Transform {
        Transform::new_scale_xy(1.0, y)
    }

    /// Create a 2d scale transform.
    pub fn new_scale_xy<X: Into<Factor>, Y: Into<Factor>>(x: X, y: Y) -> Transform {
        Transform::identity().scale_xy(x, y)
    }
}

impl Transform {
    /// Change `self` to apply `other` after its transformation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zng_layout::unit::*;
    /// Transform::new_rotate(10.deg()).then(Transform::new_translate(50, 30));
    /// ```
    ///
    /// Is the equivalent of:
    ///
    /// ```
    /// # use zng_layout::unit::*;
    /// Transform::new_rotate(10.deg()).translate(50, 30);
    /// ```
    pub fn then(mut self, other: Transform) -> Self {
        let mut other_parts = other.parts.into_iter();
        self.needs_layout |= other.needs_layout;
        if let Some(first) = other_parts.next() {
            match first {
                TransformPart::Computed(first) => self.then_transform(first),
                first => self.parts.push(first),
            }
            self.parts.extend(other_parts);
        }
        self
    }

    fn then_transform(&mut self, transform: PxTransform) {
        if let Some(TransformPart::Computed(last)) = self.parts.last_mut() {
            *last = last.then(&transform);
        } else {
            self.parts.push(TransformPart::Computed(transform));
        }
    }

    /// Change `self` to apply a 2d rotation after its transformation.
    pub fn rotate<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.then_transform(PxTransform::rotation(0.0, 0.0, angle.into().into()));
        self
    }

    /// Change `self` to apply a 3d rotation around the ***x*** axis.
    ///
    /// Note that the composition of 3D rotations is usually not commutative, so the order this is applied will affect the result.
    pub fn rotate_x<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.then_transform(PxTransform::rotation_3d(1.0, 0.0, 0.0, angle.into().into()));
        self
    }

    /// Change `self` to apply a 3d rotation around the ***y*** axis.
    ///
    /// Note that the composition of 3D rotations is usually not commutative, so the order this is applied will affect the result.
    pub fn rotate_y<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.then_transform(PxTransform::rotation_3d(0.0, 1.0, 0.0, angle.into().into()));
        self
    }

    /// Same as [`rotate`].
    ///
    /// [`rotate`]: Self::rotate
    pub fn rotate_z<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.then_transform(PxTransform::rotation_3d(0.0, 0.0, 1.0, angle.into().into()));
        self
    }

    /// Change `self` to apply a 3d rotation.
    ///
    /// Note that the composition of 3D rotations is usually not commutative, so the order this is applied will affect the result.
    pub fn rotate_3d<A: Into<AngleRadian>>(mut self, x: f32, y: f32, z: f32, angle: A) -> Self {
        self.then_transform(PxTransform::rotation_3d(x, y, z, angle.into().into()));
        self
    }

    /// Change `self` to apply a 2d translation after its transformation.
    pub fn translate<X: Into<Length>, Y: Into<Length>>(mut self, x: X, y: Y) -> Self {
        self.parts.push(TransformPart::Translate(x.into(), y.into()));
        self.needs_layout = true;
        self
    }
    /// Change `self` to apply a ***x*** translation after its transformation.
    pub fn translate_x<X: Into<Length>>(self, x: X) -> Self {
        self.translate(x, 0.0)
    }
    /// Change `self` to apply a ***y*** translation after its transformation.
    pub fn translate_y<Y: Into<Length>>(self, y: Y) -> Self {
        self.translate(0.0, y)
    }

    /// Change `self` to apply a ***z*** translation after its transformation.
    pub fn translate_z<Z: Into<Length>>(self, z: Z) -> Self {
        self.translate_3d(0.0, 0.0, z)
    }

    /// Change `self` to apply a 3d translation after its transformation.
    ///
    /// Note that the composition of 3D rotations is usually not commutative, so the order this is applied will affect the result.
    pub fn translate_3d<X: Into<Length>, Y: Into<Length>, Z: Into<Length>>(mut self, x: X, y: Y, z: Z) -> Self {
        self.parts.push(TransformPart::Translate3d(x.into(), y.into(), z.into()));
        self.needs_layout = true;
        self
    }

    /// Change `self` to apply a 2d skew after its transformation.
    pub fn skew<X: Into<AngleRadian>, Y: Into<AngleRadian>>(mut self, x: X, y: Y) -> Self {
        self.then_transform(PxTransform::skew(x.into().into(), y.into().into()));
        self
    }
    /// Change `self` to apply a ***x*** skew after its transformation.
    pub fn skew_x<X: Into<AngleRadian>>(self, x: X) -> Self {
        self.skew(x, 0.rad())
    }
    /// Change `self` to apply a ***y*** skew after its transformation.
    pub fn skew_y<Y: Into<AngleRadian>>(self, y: Y) -> Self {
        self.skew(0.rad(), y)
    }

    /// Change `self` to apply a 2d scale after its transformation.
    pub fn scale_xy<X: Into<Factor>, Y: Into<Factor>>(mut self, x: X, y: Y) -> Self {
        self.then_transform(PxTransform::scale(x.into().0, y.into().0));
        self
    }
    /// Change `self` to apply a ***x*** scale after its transformation.
    pub fn scale_x<X: Into<Factor>>(self, x: X) -> Self {
        self.scale_xy(x, 1.0)
    }
    /// Change `self` to apply a ***y*** scale after its transformation.
    pub fn scale_y<Y: Into<Factor>>(self, y: Y) -> Self {
        self.scale_xy(1.0, y)
    }
    /// Change `self` to apply a uniform 2d scale after its transformation.
    pub fn scale<S: Into<Factor>>(self, scale: S) -> Self {
        let s = scale.into();
        self.scale_xy(s, s)
    }

    /// Change `self` 3d perspective distance.
    pub fn perspective<D: Into<Length>>(mut self, d: D) -> Self {
        self.parts.push(TransformPart::Perspective(d.into()));
        self.needs_layout = true;
        self
    }
}
impl Transform {
    /// Compute a [`PxTransform`] in the current [`LAYOUT`] context.
    ///
    /// [`LAYOUT`]: crate::context::LAYOUT
    pub fn layout(&self) -> PxTransform {
        let mut r = PxTransform::identity();
        for step in &self.parts {
            r = match step {
                TransformPart::Computed(m) => r.then(m),
                TransformPart::Translate(x, y) => r.then(&PxTransform::translation(x.layout_f32_x(), y.layout_f32_y())),
                TransformPart::Translate3d(x, y, z) => {
                    r.then(&PxTransform::translation_3d(x.layout_f32_x(), y.layout_f32_y(), z.layout_f32_z()))
                }
                TransformPart::Perspective(d) => r.then(&PxTransform::perspective(d.layout_f32_z())),
            };
        }

        for (to, step, slerp) in self.lerp_to.iter() {
            let to = to.layout();
            r = slerp_enabled(*slerp, || lerp_px_transform(r, &to, *step));
        }

        r
    }

    /// Compute a [`PxTransform`] if it is not affected by the layout context.
    pub fn try_layout(&self) -> Option<PxTransform> {
        if self.needs_layout {
            return None;
        }

        let mut r = PxTransform::identity();
        for step in &self.parts {
            r = match step {
                TransformPart::Computed(m) => r.then(m),
                TransformPart::Translate(_, _) | TransformPart::Translate3d(_, _, _) | TransformPart::Perspective(_) => unreachable!(),
            }
        }
        Some(r)
    }

    /// Returns `true` if this transform is affected by the layout context where it is evaluated.
    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }
}
impl super::Layout2d for Transform {
    type Px = PxTransform;

    fn layout_dft(&self, _: Self::Px) -> Self::Px {
        self.layout()
    }

    fn affect_mask(&self) -> super::LayoutMask {
        let mut mask = super::LayoutMask::empty();
        for part in &self.parts {
            match part {
                TransformPart::Computed(_) => {}
                TransformPart::Translate(x, y) => {
                    mask |= x.affect_mask();
                    mask |= y.affect_mask();
                }
                TransformPart::Translate3d(x, y, z) => {
                    mask |= x.affect_mask();
                    mask |= y.affect_mask();
                    mask |= z.affect_mask();
                }
                TransformPart::Perspective(d) => mask |= d.affect_mask(),
            }
        }
        mask
    }
}

impl Transitionable for Transform {
    fn lerp(mut self, to: &Self, step: EasingStep) -> Self {
        if step == 0.fct() {
            self
        } else if step == 1.fct() {
            to.clone()
        } else {
            if self.needs_layout || to.needs_layout {
                self.needs_layout = true;
                self.lerp_to.push((to.clone(), step, is_slerp_enabled()));
            } else {
                let a = self.layout();
                let b = to.layout();
                self = Transform::from(lerp_px_transform(a, &b, step));
            }
            self
        }
    }
}

fn lerp_px_transform(s: PxTransform, to: &PxTransform, step: EasingStep) -> PxTransform {
    if step == 0.fct() {
        s
    } else if step == 1.fct() {
        *to
    } else {
        match (s, to) {
            (PxTransform::Offset(from), PxTransform::Offset(to)) => PxTransform::Offset(from.lerp(*to, step.0)),
            (from, to) => {
                match (
                    MatrixDecomposed3D::decompose(from.to_transform()),
                    MatrixDecomposed3D::decompose(to.to_transform()),
                ) {
                    (Some(from), Some(to)) => {
                        let l = from.lerp(&to, step);
                        PxTransform::Transform(l.recompose())
                    }
                    _ => {
                        if step.0 < 0.5 {
                            s
                        } else {
                            *to
                        }
                    }
                }
            }
        }
    }
}

impl_from_and_into_var! {
    fn from(t: PxTransform) -> Transform {
        Transform {
            parts: vec![TransformPart::Computed(t)],
            needs_layout: false,
            lerp_to: vec![],
        }
    }
}

// Matrix decomposition. Mostly copied from Servo code.
// https://github.com/servo/servo/blob/master/components/style/values/animated/transform.rs

type Scale = (f32, f32, f32);
type Skew = (f32, f32, f32);
type Perspective = (f32, f32, f32, f32);
type Quaternion = euclid::Rotation3D<f64, (), ()>;

/// A decomposed 3d matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MatrixDecomposed3D {
    /// A translation function.
    pub translate: euclid::Vector3D<f32, Px>,
    /// A scale function.
    pub scale: Scale,
    /// The skew component of the transformation.
    pub skew: Skew,
    /// The perspective component of the transformation.
    pub perspective: Perspective,
    /// The quaternion used to represent the rotation.
    pub quaternion: Quaternion,
}
impl MatrixDecomposed3D {
    pub fn decompose(mut matrix: euclid::Transform3D<f32, Px, Px>) -> Option<Self> {
        // Combine 2 point.
        let combine = |a: [f32; 3], b: [f32; 3], ascl: f32, bscl: f32| {
            [
                (ascl * a[0]) + (bscl * b[0]),
                (ascl * a[1]) + (bscl * b[1]),
                (ascl * a[2]) + (bscl * b[2]),
            ]
        };
        // Dot product.
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        // Cross product.
        let cross = |row1: [f32; 3], row2: [f32; 3]| {
            [
                row1[1] * row2[2] - row1[2] * row2[1],
                row1[2] * row2[0] - row1[0] * row2[2],
                row1[0] * row2[1] - row1[1] * row2[0],
            ]
        };

        if matrix.m44 == 0.0 {
            return None;
        }

        let scaling_factor = matrix.m44;

        // Normalize the matrix.
        matrix_scale_by_factor(&mut matrix, 1.0 / scaling_factor);

        // perspective_matrix is used to solve for perspective, but it also provides
        // an easy way to test for singularity of the upper 3x3 component.
        let mut perspective_matrix = matrix;

        perspective_matrix.m14 = 0.0;
        perspective_matrix.m24 = 0.0;
        perspective_matrix.m34 = 0.0;
        perspective_matrix.m44 = 1.0;

        if perspective_matrix.determinant() == 0.0 {
            return None;
        }

        // First, isolate perspective.
        let perspective = if matrix.m14 != 0.0 || matrix.m24 != 0.0 || matrix.m34 != 0.0 {
            let right_hand_side: [f32; 4] = [matrix.m14, matrix.m24, matrix.m34, matrix.m44];

            perspective_matrix = matrix_transpose(perspective_matrix.inverse().unwrap());
            let perspective = matrix_pre_mul_point4(&perspective_matrix, &right_hand_side);

            (perspective[0], perspective[1], perspective[2], perspective[3])
        } else {
            (0.0, 0.0, 0.0, 1.0)
        };

        // Next take care of translation (easy).
        let translate = euclid::Vector3D::new(matrix.m41, matrix.m42, matrix.m43);

        // Now get scale and shear. 'row' is a 3 element array of 3 component vectors
        let mut row = get_matrix_3x3_part(&matrix);

        // Compute X scale factor and normalize first row.
        let row0len = (row[0][0] * row[0][0] + row[0][1] * row[0][1] + row[0][2] * row[0][2]).sqrt();
        let mut scale = (row0len, 0.0, 0.0);
        row[0] = [row[0][0] / row0len, row[0][1] / row0len, row[0][2] / row0len];

        // Compute XY shear factor and make 2nd row orthogonal to 1st.
        let mut skew = (dot(row[0], row[1]), 0.0, 0.0);
        row[1] = combine(row[1], row[0], 1.0, -skew.0);

        // Now, compute Y scale and normalize 2nd row.
        let row1len = (row[1][0] * row[1][0] + row[1][1] * row[1][1] + row[1][2] * row[1][2]).sqrt();
        scale.1 = row1len;
        row[1] = [row[1][0] / row1len, row[1][1] / row1len, row[1][2] / row1len];
        skew.0 /= scale.1;

        // Compute XZ and YZ shears, orthogonalize 3rd row
        skew.1 = dot(row[0], row[2]);
        row[2] = combine(row[2], row[0], 1.0, -skew.1);
        skew.2 = dot(row[1], row[2]);
        row[2] = combine(row[2], row[1], 1.0, -skew.2);

        // Next, get Z scale and normalize 3rd row.
        let row2len = (row[2][0] * row[2][0] + row[2][1] * row[2][1] + row[2][2] * row[2][2]).sqrt();
        scale.2 = row2len;
        row[2] = [row[2][0] / row2len, row[2][1] / row2len, row[2][2] / row2len];
        skew.1 /= scale.2;
        skew.2 /= scale.2;

        // At this point, the matrix (in rows) is orthonormal.
        // Check for a coordinate system flip. If the determinant
        // is -1, then negate the matrix and the scaling factors.
        if dot(row[0], cross(row[1], row[2])) < 0.0 {
            scale.0 *= -1.0;
            scale.1 *= -1.0;
            scale.2 *= -1.0;

            #[expect(clippy::needless_range_loop)]
            for i in 0..3 {
                row[i][0] *= -1.0;
                row[i][1] *= -1.0;
                row[i][2] *= -1.0;
            }
        }

        // Now, get the rotations out.
        let mut quaternion = Quaternion::quaternion(
            0.5 * ((1.0 + row[0][0] - row[1][1] - row[2][2]).max(0.0) as f64).sqrt(),
            0.5 * ((1.0 - row[0][0] + row[1][1] - row[2][2]).max(0.0) as f64).sqrt(),
            0.5 * ((1.0 - row[0][0] - row[1][1] + row[2][2]).max(0.0) as f64).sqrt(),
            0.5 * ((1.0 + row[0][0] + row[1][1] + row[2][2]).max(0.0) as f64).sqrt(),
        );

        if row[2][1] > row[1][2] {
            quaternion.i = -quaternion.i
        }
        if row[0][2] > row[2][0] {
            quaternion.j = -quaternion.j
        }
        if row[1][0] > row[0][1] {
            quaternion.k = -quaternion.k
        }

        Some(MatrixDecomposed3D {
            translate,
            scale,
            skew,
            perspective,
            quaternion,
        })
    }

    pub fn recompose(self) -> euclid::Transform3D<f32, Px, Px> {
        let mut matrix = euclid::Transform3D::identity();

        // set perspective
        matrix.m14 = self.perspective.0;
        matrix.m24 = self.perspective.1;
        matrix.m34 = self.perspective.2;
        matrix.m44 = self.perspective.3;

        // apply translate
        matrix.m41 += self.translate.x * matrix.m11 + self.translate.y * matrix.m21 + self.translate.z * matrix.m31;
        matrix.m42 += self.translate.x * matrix.m12 + self.translate.y * matrix.m22 + self.translate.z * matrix.m32;
        matrix.m43 += self.translate.x * matrix.m13 + self.translate.y * matrix.m23 + self.translate.z * matrix.m33;
        matrix.m44 += self.translate.x * matrix.m14 + self.translate.y * matrix.m24 + self.translate.z * matrix.m34;

        // apply rotation
        {
            let x = self.quaternion.i;
            let y = self.quaternion.j;
            let z = self.quaternion.k;
            let w = self.quaternion.r;

            // Construct a composite rotation matrix from the quaternion values
            // rotationMatrix is a identity 4x4 matrix initially
            let mut rotation_matrix = euclid::Transform3D::identity();
            rotation_matrix.m11 = 1.0 - 2.0 * (y * y + z * z) as f32;
            rotation_matrix.m12 = 2.0 * (x * y + z * w) as f32;
            rotation_matrix.m13 = 2.0 * (x * z - y * w) as f32;
            rotation_matrix.m21 = 2.0 * (x * y - z * w) as f32;
            rotation_matrix.m22 = 1.0 - 2.0 * (x * x + z * z) as f32;
            rotation_matrix.m23 = 2.0 * (y * z + x * w) as f32;
            rotation_matrix.m31 = 2.0 * (x * z + y * w) as f32;
            rotation_matrix.m32 = 2.0 * (y * z - x * w) as f32;
            rotation_matrix.m33 = 1.0 - 2.0 * (x * x + y * y) as f32;

            matrix = rotation_matrix.then(&matrix);
        }

        // Apply skew
        {
            let mut temp = euclid::Transform3D::identity();
            if self.skew.2 != 0.0 {
                temp.m32 = self.skew.2;
                matrix = temp.then(&matrix);
                temp.m32 = 0.0;
            }

            if self.skew.1 != 0.0 {
                temp.m31 = self.skew.1;
                matrix = temp.then(&matrix);
                temp.m31 = 0.0;
            }

            if self.skew.0 != 0.0 {
                temp.m21 = self.skew.0;
                matrix = temp.then(&matrix);
            }
        }

        // apply scale
        matrix.m11 *= self.scale.0;
        matrix.m12 *= self.scale.0;
        matrix.m13 *= self.scale.0;
        matrix.m14 *= self.scale.0;
        matrix.m21 *= self.scale.1;
        matrix.m22 *= self.scale.1;
        matrix.m23 *= self.scale.1;
        matrix.m24 *= self.scale.1;
        matrix.m31 *= self.scale.2;
        matrix.m32 *= self.scale.2;
        matrix.m33 *= self.scale.2;
        matrix.m34 *= self.scale.2;

        matrix
    }

    pub fn lerp_scale(from: Scale, to: Scale, step: EasingStep) -> Scale {
        (from.0.lerp(&to.0, step), from.1.lerp(&to.1, step), from.2.lerp(&to.2, step))
    }

    pub fn lerp_skew(from: Skew, to: Skew, step: EasingStep) -> Skew {
        (from.0.lerp(&to.0, step), from.1.lerp(&to.1, step), from.2.lerp(&to.2, step))
    }

    pub fn lerp_perspective(from: Perspective, to: Perspective, step: EasingStep) -> Perspective {
        (
            from.0.lerp(&to.0, step),
            from.1.lerp(&to.1, step),
            from.2.lerp(&to.2, step),
            from.3.lerp(&to.3, step),
        )
    }

    pub fn lerp_quaternion(from: Quaternion, to: Quaternion, step: EasingStep) -> Quaternion {
        match is_slerp_enabled() {
            false => from.lerp(&to, step.0 as _),
            true => from.slerp(&to, step.0 as _),
        }
    }
}
impl Transitionable for MatrixDecomposed3D {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        Self {
            translate: self.translate.lerp(to.translate, step.0),
            scale: Self::lerp_scale(self.scale, to.scale, step),
            skew: Self::lerp_skew(self.skew, to.skew, step),
            perspective: Self::lerp_perspective(self.perspective, to.perspective, step),
            quaternion: Self::lerp_quaternion(self.quaternion, to.quaternion, step),
        }
    }
}

fn matrix_scale_by_factor(m: &mut euclid::Transform3D<f32, Px, Px>, scaling_factor: f32) {
    m.m11 *= scaling_factor;
    m.m12 *= scaling_factor;
    m.m13 *= scaling_factor;
    m.m14 *= scaling_factor;
    m.m21 *= scaling_factor;
    m.m22 *= scaling_factor;
    m.m23 *= scaling_factor;
    m.m24 *= scaling_factor;
    m.m31 *= scaling_factor;
    m.m32 *= scaling_factor;
    m.m33 *= scaling_factor;
    m.m34 *= scaling_factor;
    m.m41 *= scaling_factor;
    m.m42 *= scaling_factor;
    m.m43 *= scaling_factor;
    m.m44 *= scaling_factor;
}

fn matrix_transpose(m: euclid::Transform3D<f32, Px, Px>) -> euclid::Transform3D<f32, Px, Px> {
    euclid::Transform3D {
        m11: m.m11,
        m12: m.m21,
        m13: m.m31,
        m14: m.m41,
        m21: m.m12,
        m22: m.m22,
        m23: m.m32,
        m24: m.m42,
        m31: m.m13,
        m32: m.m23,
        m33: m.m33,
        m34: m.m43,
        m41: m.m14,
        m42: m.m24,
        m43: m.m34,
        m44: m.m44,
        _unit: std::marker::PhantomData,
    }
}

fn matrix_pre_mul_point4(m: &euclid::Transform3D<f32, Px, Px>, pin: &[f32; 4]) -> [f32; 4] {
    [
        pin[0] * m.m11 + pin[1] * m.m21 + pin[2] * m.m31 + pin[3] * m.m41,
        pin[0] * m.m12 + pin[1] * m.m22 + pin[2] * m.m32 + pin[3] * m.m42,
        pin[0] * m.m13 + pin[1] * m.m23 + pin[2] * m.m33 + pin[3] * m.m43,
        pin[0] * m.m14 + pin[1] * m.m24 + pin[2] * m.m34 + pin[3] * m.m44,
    ]
}

fn get_matrix_3x3_part(&m: &euclid::Transform3D<f32, Px, Px>) -> [[f32; 3]; 3] {
    [[m.m11, m.m12, m.m13], [m.m21, m.m22, m.m23], [m.m31, m.m32, m.m33]]
}
