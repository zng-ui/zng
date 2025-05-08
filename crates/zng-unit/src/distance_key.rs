use crate::{Px, PxPoint, PxRect};

use serde::{Deserialize, Serialize};

/// Comparable key that represents the absolute distance between two pixel points.
///
/// Computing the actual distance only for comparison is expensive, this key avoids the conversion to float and square-root operation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(transparent)]
#[serde(transparent)]
pub struct DistanceKey(u64);
impl DistanceKey {
    /// Value that is always greater than any distance key.
    pub const NONE_MAX: DistanceKey = DistanceKey(u64::MAX);

    /// Value that is always smaller than any distance key.
    pub const NONE_MIN: DistanceKey = DistanceKey(0);

    /// Maximum distance.
    pub const MAX: DistanceKey = DistanceKey((Px::MAX.0 as u64).pow(2));

    /// Minimum distance.
    pub const MIN: DistanceKey = DistanceKey(1);

    /// New distance key computed from two points.
    pub fn from_points(a: PxPoint, b: PxPoint) -> Self {
        let pa = ((a.x - b.x).0.unsigned_abs() as u64).pow(2);
        let pb = ((a.y - b.y).0.unsigned_abs() as u64).pow(2);

        Self((pa + pb) + 1)
    }

    /// New distance key computed from the nearest point inside `a` to `b`.
    pub fn from_rect_to_point(a: PxRect, b: PxPoint) -> Self {
        Self::from_points(b.clamp(a.min(), a.max()), b)
    }

    /// New distance key from already computed actual distance.
    ///
    /// Note that computing the actual distance is slower then using [`from_points`] to compute just the distance key.
    ///
    /// [`from_points`]: Self::from_points
    pub fn from_distance(d: Px) -> Self {
        let p = (d.0.unsigned_abs() as u64).pow(2);
        Self(p + 1)
    }

    /// If the key is the [`NONE_MAX`] or [`NONE_MIN`].
    ///
    /// [`NONE_MAX`]: Self::NONE_MAX
    /// [`NONE_MIN`]: Self::NONE_MIN
    pub fn is_none(self) -> bool {
        self == Self::NONE_MAX || self == Self::NONE_MIN
    }

    /// Completes the distance calculation.
    pub fn distance(self) -> Option<Px> {
        if self.is_none() {
            None
        } else {
            let p = self.0 - 1;
            let d = (p as f64).sqrt();

            Some(Px(d.round() as i32))
        }
    }

    /// Compares and returns the minimum distance.
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Compares and returns the maximum distance.
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}
