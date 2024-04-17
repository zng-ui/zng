use std::{fmt, marker::PhantomData, ops};

use serde::{Deserialize, Serialize};

/// A group of 2D side offsets, which correspond to top/right/bottom/left for borders, padding,
/// and margins in CSS, optionally tagged with a unit.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct SideOffsets2D<T, U> {
    /// Top offset.
    pub top: T,
    /// Right offset.
    pub right: T,
    /// Bottom offset.
    pub bottom: T,
    /// Left offset.
    pub left: T,
    #[doc(hidden)]
    #[serde(skip)] // euclid does not skip this field
    pub _unit: PhantomData<U>,
}
impl<T, U> From<euclid::SideOffsets2D<T, U>> for SideOffsets2D<T, U> {
    fn from(value: euclid::SideOffsets2D<T, U>) -> Self {
        Self {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            _unit: PhantomData,
        }
    }
}
impl<T, U> From<SideOffsets2D<T, U>> for euclid::SideOffsets2D<T, U> {
    fn from(value: SideOffsets2D<T, U>) -> Self {
        Self {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            _unit: PhantomData,
        }
    }
}
impl<T: Copy, U> Copy for SideOffsets2D<T, U> {}
impl<T: Clone, U> Clone for SideOffsets2D<T, U> {
    fn clone(&self) -> Self {
        SideOffsets2D {
            top: self.top.clone(),
            right: self.right.clone(),
            bottom: self.bottom.clone(),
            left: self.left.clone(),
            _unit: PhantomData,
        }
    }
}
impl<T, U> Eq for SideOffsets2D<T, U> where T: Eq {}
impl<T, U> PartialEq for SideOffsets2D<T, U>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.top == other.top && self.right == other.right && self.bottom == other.bottom && self.left == other.left
    }
}
impl<T, U> std::hash::Hash for SideOffsets2D<T, U>
where
    T: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.top.hash(h);
        self.right.hash(h);
        self.bottom.hash(h);
        self.left.hash(h);
    }
}
impl<T: fmt::Debug, U> fmt::Debug for SideOffsets2D<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:?},{:?},{:?},{:?})", self.top, self.right, self.bottom, self.left)
    }
}
impl<T: Default, U> Default for SideOffsets2D<T, U> {
    fn default() -> Self {
        SideOffsets2D {
            top: Default::default(),
            right: Default::default(),
            bottom: Default::default(),
            left: Default::default(),
            _unit: PhantomData,
        }
    }
}
impl<T, U> SideOffsets2D<T, U> {
    /// Constructor taking a scalar for each side.
    ///
    /// Sides are specified in top-right-bottom-left order following
    /// CSS's convention.
    pub const fn new(top: T, right: T, bottom: T, left: T) -> Self {
        SideOffsets2D {
            top,
            right,
            bottom,
            left,
            _unit: PhantomData,
        }
    }

    /// Construct side offsets from min and a max vector offsets.
    ///
    /// The outer rect of the resulting side offsets is equivalent to translating
    /// a rectangle's upper-left corner with the min vector and translating the
    /// bottom-right corner with the max vector.
    pub fn from_vectors_outer(min: euclid::Vector2D<T, U>, max: euclid::Vector2D<T, U>) -> Self
    where
        T: ops::Neg<Output = T>,
    {
        SideOffsets2D {
            left: -min.x,
            top: -min.y,
            right: max.x,
            bottom: max.y,
            _unit: PhantomData,
        }
    }

    /// Construct side offsets from min and a max vector offsets.
    ///
    /// The inner rect of the resulting side offsets is equivalent to translating
    /// a rectangle's upper-left corner with the min vector and translating the
    /// bottom-right corner with the max vector.
    pub fn from_vectors_inner(min: euclid::Vector2D<T, U>, max: euclid::Vector2D<T, U>) -> Self
    where
        T: ops::Neg<Output = T>,
    {
        SideOffsets2D {
            left: min.x,
            top: min.y,
            right: -max.x,
            bottom: -max.y,
            _unit: PhantomData,
        }
    }

    /// Constructor, setting all sides to zero.
    pub fn zero() -> Self
    where
        T: euclid::num::Zero,
    {
        use euclid::num::Zero;
        SideOffsets2D::new(Zero::zero(), Zero::zero(), Zero::zero(), Zero::zero())
    }

    /// Returns `true` if all side offsets are zero.
    pub fn is_zero(&self) -> bool
    where
        T: euclid::num::Zero + PartialEq,
    {
        let zero = T::zero();
        self.top == zero && self.right == zero && self.bottom == zero && self.left == zero
    }

    /// Constructor setting the same value to all sides, taking a scalar value directly.
    pub fn new_all_same(all: T) -> Self
    where
        T: Copy,
    {
        SideOffsets2D::new(all, all, all, all)
    }

    /// Left + right.
    pub fn horizontal(&self) -> T
    where
        T: Copy + ops::Add<T, Output = T>,
    {
        self.left + self.right
    }

    /// Top + bottom.
    pub fn vertical(&self) -> T
    where
        T: Copy + ops::Add<T, Output = T>,
    {
        self.top + self.bottom
    }
}
impl<T, U> ops::Add for SideOffsets2D<T, U>
where
    T: ops::Add<T, Output = T>,
{
    type Output = Self;
    fn add(self, other: Self) -> Self {
        SideOffsets2D::new(
            self.top + other.top,
            self.right + other.right,
            self.bottom + other.bottom,
            self.left + other.left,
        )
    }
}
impl<T, U> ops::AddAssign<Self> for SideOffsets2D<T, U>
where
    T: ops::AddAssign<T>,
{
    fn add_assign(&mut self, other: Self) {
        self.top += other.top;
        self.right += other.right;
        self.bottom += other.bottom;
        self.left += other.left;
    }
}
impl<T, U> ops::Sub for SideOffsets2D<T, U>
where
    T: ops::Sub<T, Output = T>,
{
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        SideOffsets2D::new(
            self.top - other.top,
            self.right - other.right,
            self.bottom - other.bottom,
            self.left - other.left,
        )
    }
}
impl<T, U> ops::SubAssign<Self> for SideOffsets2D<T, U>
where
    T: ops::SubAssign<T>,
{
    fn sub_assign(&mut self, other: Self) {
        self.top -= other.top;
        self.right -= other.right;
        self.bottom -= other.bottom;
        self.left -= other.left;
    }
}

impl<T, U> ops::Neg for SideOffsets2D<T, U>
where
    T: ops::Neg<Output = T>,
{
    type Output = Self;
    fn neg(self) -> Self {
        SideOffsets2D {
            top: -self.top,
            right: -self.right,
            bottom: -self.bottom,
            left: -self.left,
            _unit: PhantomData,
        }
    }
}
impl<T: Copy + ops::Mul, U> ops::Mul<T> for SideOffsets2D<T, U> {
    type Output = SideOffsets2D<T::Output, U>;

    #[inline]
    fn mul(self, scale: T) -> Self::Output {
        SideOffsets2D::new(self.top * scale, self.right * scale, self.bottom * scale, self.left * scale)
    }
}
impl<T: Copy + ops::MulAssign, U> ops::MulAssign<T> for SideOffsets2D<T, U> {
    #[inline]
    fn mul_assign(&mut self, other: T) {
        self.top *= other;
        self.right *= other;
        self.bottom *= other;
        self.left *= other;
    }
}
impl<T: Copy + ops::Mul, U1, U2> ops::Mul<euclid::Scale<T, U1, U2>> for SideOffsets2D<T, U1> {
    type Output = SideOffsets2D<T::Output, U2>;

    #[inline]
    fn mul(self, scale: euclid::Scale<T, U1, U2>) -> Self::Output {
        SideOffsets2D::new(self.top * scale.0, self.right * scale.0, self.bottom * scale.0, self.left * scale.0)
    }
}
impl<T: Copy + ops::MulAssign, U> ops::MulAssign<euclid::Scale<T, U, U>> for SideOffsets2D<T, U> {
    #[inline]
    fn mul_assign(&mut self, other: euclid::Scale<T, U, U>) {
        *self *= other.0;
    }
}
impl<T: Copy + ops::Div, U> ops::Div<T> for SideOffsets2D<T, U> {
    type Output = SideOffsets2D<T::Output, U>;

    #[inline]
    fn div(self, scale: T) -> Self::Output {
        SideOffsets2D::new(self.top / scale, self.right / scale, self.bottom / scale, self.left / scale)
    }
}
impl<T: Copy + ops::DivAssign, U> ops::DivAssign<T> for SideOffsets2D<T, U> {
    #[inline]
    fn div_assign(&mut self, other: T) {
        self.top /= other;
        self.right /= other;
        self.bottom /= other;
        self.left /= other;
    }
}
impl<T: Copy + ops::Div, U1, U2> ops::Div<euclid::Scale<T, U1, U2>> for SideOffsets2D<T, U2> {
    type Output = SideOffsets2D<T::Output, U1>;

    #[inline]
    fn div(self, scale: euclid::Scale<T, U1, U2>) -> Self::Output {
        SideOffsets2D::new(self.top / scale.0, self.right / scale.0, self.bottom / scale.0, self.left / scale.0)
    }
}
impl<T: Copy + ops::DivAssign, U> ops::DivAssign<euclid::Scale<T, U, U>> for SideOffsets2D<T, U> {
    fn div_assign(&mut self, other: euclid::Scale<T, U, U>) {
        *self /= other.0;
    }
}
