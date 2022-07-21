use crate::context::LayoutMetrics;

use super::{AngleRadian, AngleUnits, Factor, Length, Px, PxToWr, PxTransform};

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
    Computed(PxTransform),
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

    fn then_transform(&mut self, transform: PxTransform) {
        if let Some(TransformPart::Computed(last)) = self.parts.last_mut() {
            *last = last.then(&transform);
        } else {
            self.parts.push(TransformPart::Computed(transform));
        }
    }

    /// Change `self` to apply a 2d rotation after its transformation.
    pub fn rotate<A: Into<AngleRadian>>(mut self, angle: A) -> Self {
        self.then_transform(PxTransform::rotation(0.0, 0.0, angle.into().layout()));
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
        self.then_transform(PxTransform::skew(x.into().layout(), y.into().layout()));
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

    /// Compute a [`PxTransform`].
    pub fn layout(&self, ctx: &LayoutMetrics) -> PxTransform {
        let mut r = PxTransform::identity();
        for step in &self.parts {
            r = match step {
                TransformPart::Computed(m) => r.then(m),
                TransformPart::Translate(x, y) => r.then(&PxTransform::translation(
                    x.layout(ctx.for_x(), |_| Px(0)).to_wr().get(),
                    y.layout(ctx.for_y(), |_| Px(0)).to_wr().get(),
                )),
            };
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
                TransformPart::Translate(_, _) => unreachable!(),
            }
        }
        Some(r)
    }

    /// Returns `true` if this transform is affected by the layout context where it is evaluated.
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
