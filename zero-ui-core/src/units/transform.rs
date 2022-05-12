use zero_ui_view_api::webrender_api;

use crate::context::LayoutMetrics;

use super::{euclid, AngleRadian, AngleUnits, Factor, Length, Px, PxPoint, PxToWr, PxVector};

/// Computed [`Transform`].
///
/// See also [`webrender_api::units::LayoutTransform`] and [`RenderTransformExt`].
pub type RenderTransform = webrender_api::units::LayoutTransform;

/// Extension methods for [`RenderTransform`].
pub trait RenderTransformExt {
    /// New translation transform from a pixel vector.
    fn translation_px(offset: PxVector) -> RenderTransform;

    /// Returns a transform with a translation applied before `self`, the translation is defined from a pixel vector.
    fn pre_translate_px(&self, offset: PxVector) -> RenderTransform;

    ///  Returns a transform with a translation applied after `self`, the translation is defined from a pixel vector.
    fn then_translate_px(&self, offset: PxVector) -> RenderTransform;

    /// Returns the given [`PxPoint`] transformed by this transform, if the transform makes sense,
    /// or `None` otherwise.
    fn transform_px_point(&self, point: PxPoint) -> Option<PxPoint>;

    /// Returns the given [`PxVector`] transformed by this matrix.
    fn transform_px_vector(&self, vector: PxVector) -> PxVector;
}
impl RenderTransformExt for RenderTransform {
    fn translation_px(offset: PxVector) -> RenderTransform {
        RenderTransform::translation(offset.x.0 as f32, offset.y.0 as f32, 0.0)
    }

    fn pre_translate_px(&self, offset: PxVector) -> RenderTransform {
        self.pre_translate(euclid::vec3(offset.x.0 as f32, offset.y.0 as f32, 0.0))
    }

    fn then_translate_px(&self, offset: PxVector) -> RenderTransform {
        self.then_translate(euclid::vec3(offset.x.0 as f32, offset.y.0 as f32, 0.0))
    }

    fn transform_px_point(&self, point: PxPoint) -> Option<PxPoint> {
        let point = euclid::point2(point.x.0 as f32, point.y.0 as f32);
        let point = self.transform_point2d(point)?;
        Some(PxPoint::new(Px(point.x as i32), Px(point.y as i32)))
    }

    fn transform_px_vector(&self, vector: PxVector) -> PxVector {
        let vector = euclid::vec2(vector.x.0 as f32, vector.y.0 as f32);
        let vector = self.transform_vector2d(vector);
        PxVector::new(Px(vector.x as i32), Px(vector.y as i32))
    }
}

/// A transform builder type.
///
/// # Builder
///
/// The transform can be started by one of this functions, [`rotate`], [`translate`], [`scale`] and [`skew`]. More
/// transforms can be chained by calling the methods of this type.
///
/// # Examples
///
/// Create a transform that
///
/// ```
/// # use zero_ui_core::units::*;
/// let rotate_then_move = rotate(10.deg()).translate(50, 30);
/// ```
///
///
#[derive(Clone, Default, Debug)]
pub struct Transform {
    parts: Vec<TransformPart>,
    needs_layout: bool,
}
#[derive(Clone, Debug)]
enum TransformPart {
    Computed(RenderTransform),
    Translate(Length, Length),
}
impl Transform {
    /// No transform.
    pub fn identity() -> Self {
        Self::default()
    }

    /// Change `self` to apply `other` after its transformation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::units::*;
    /// rotate(10.deg()).then(translate(50, 30));
    /// ```
    ///
    /// Is the equivalent of:
    ///
    /// ```
    /// # use zero_ui_core::units::*;
    /// rotate(10.deg()).translate(50, 30);
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

    fn then_transform(&mut self, transform: RenderTransform) {
        if let Some(TransformPart::Computed(last)) = self.parts.last_mut() {
            *last = last.then(&transform);
        } else {
            self.parts.push(TransformPart::Computed(transform));
        }
    }

    /// Change `self` to apply a 2d rotation after its transformation.
    pub fn rotate<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.then_transform(RenderTransform::rotation(0.0, 0.0, -1.0, angle.into().layout()));
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

    /// Change `self` to apply a 2d skew after its transformation.
    pub fn skew<X: Into<AngleRadian>, Y: Into<AngleRadian>>(mut self, x: X, y: Y) -> Self {
        self.then_transform(RenderTransform::skew(x.into().layout(), y.into().layout()));
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
        self.then_transform(RenderTransform::scale(x.into().0, y.into().0, 1.0));
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

    /// Compute a [`RenderTransform`].
    pub fn layout(&self, ctx: &LayoutMetrics) -> RenderTransform {
        let mut r = RenderTransform::identity();
        for step in &self.parts {
            r = match step {
                TransformPart::Computed(m) => r.then(m),
                TransformPart::Translate(x, y) => r.then(&RenderTransform::translation(
                    x.layout(ctx.for_x(), Px(0)).to_wr().get(),
                    y.layout(ctx.for_y(), Px(0)).to_wr().get(),
                    0.0,
                )),
            };
        }
        r
    }

    /// Compute a [`RenderTransform`] if it is not affected by the layout context.
    pub fn try_render(&self) -> Option<RenderTransform> {
        if self.needs_layout {
            return None;
        }

        let mut r = RenderTransform::identity();
        for step in &self.parts {
            r = match step {
                TransformPart::Computed(m) => r.then(m),
                TransformPart::Translate(_, _) => unreachable!(),
            }
        }
        Some(r)
    }

    /// Returns `true` if this filter is affected by the layout context where it is evaluated.
    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }
}

/// Create a 2d rotation transform.
pub fn rotate<A: Into<AngleRadian>>(angle: A) -> Transform {
    Transform::default().rotate(angle)
}

/// Create a 2d translation transform.
pub fn translate<X: Into<Length>, Y: Into<Length>>(x: X, y: Y) -> Transform {
    Transform::default().translate(x, y)
}

/// Create a 2d translation transform in the X dimension.
pub fn translate_x<X: Into<Length>>(x: X) -> Transform {
    translate(x, 0.0)
}

/// Create a 2d translation transform in the Y dimension.
pub fn translate_y<Y: Into<Length>>(y: Y) -> Transform {
    translate(0.0, y)
}

/// Create a 2d skew transform.
pub fn skew<X: Into<AngleRadian>, Y: Into<AngleRadian>>(x: X, y: Y) -> Transform {
    Transform::default().skew(x, y)
}

/// Create a 2d skew transform in the X dimension.
pub fn skew_x<X: Into<AngleRadian>>(x: X) -> Transform {
    skew(x, 0.rad())
}

/// Create a 2d skew transform in the Y dimension.
pub fn skew_y<Y: Into<AngleRadian>>(y: Y) -> Transform {
    skew(0.rad(), y)
}

/// Create a 2d scale transform.
///
/// The same `scale` is applied to both dimensions.
pub fn scale<S: Into<Factor>>(scale: S) -> Transform {
    let scale = scale.into();
    scale_xy(scale, scale)
}

/// Create a 2d scale transform on the X dimension.
pub fn scale_x<X: Into<Factor>>(x: X) -> Transform {
    scale_xy(x, 1.0)
}

/// Create a 2d scale transform on the Y dimension.
pub fn scale_y<Y: Into<Factor>>(y: Y) -> Transform {
    scale_xy(1.0, y)
}

/// Create a 2d scale transform.
pub fn scale_xy<X: Into<Factor>, Y: Into<Factor>>(x: X, y: Y) -> Transform {
    Transform::default().scale_xy(x, y)
}
