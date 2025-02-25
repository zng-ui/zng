use crate::{Px, PxBox, PxPoint, PxRect, PxSize, PxVector};

use serde::{Deserialize, Serialize};

/// Orientation of two 2D items.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Orientation2D {
    /// Point is above the origin.
    Above,
    /// Point is to the right of the origin.
    Right,
    /// Point is below the origin.
    Below,
    /// Point is to the left of the origin.
    Left,
}
impl Orientation2D {
    /// Check if `point` is orientation from `origin`.
    ///
    /// Returns `true` if the point is hit by a 45º frustum cast from origin in the direction defined by the orientation.
    pub fn point_is(self, origin: PxPoint, point: PxPoint) -> bool {
        let (a, b, c, d) = match self {
            Orientation2D::Above => (point.y, origin.y, point.x, origin.x),
            Orientation2D::Right => (origin.x, point.x, point.y, origin.y),
            Orientation2D::Below => (origin.y, point.y, point.x, origin.x),
            Orientation2D::Left => (point.x, origin.x, point.y, origin.y),
        };

        let mut is = false;

        // for 'Above' this is:
        // is above line?
        if a < b {
            // is to the right?
            if c > d {
                // is in the 45º 'frustum'
                // │?╱
                // │╱__
                is = c <= d + (b - a);
            } else {
                //  ╲?│
                // __╲│
                is = c >= d - (b - a);
            }
        }

        is
    }

    /// Check if `b` is orientation from `origin`.
    ///
    /// Returns `true` if the box `b` collides with the box `origin` in the direction defined by orientation. Also
    /// returns `true` if the boxes already overlap.
    pub fn box_is(self, origin: PxBox, b: PxBox) -> bool {
        fn d_intersects(a_min: Px, a_max: Px, b_min: Px, b_max: Px) -> bool {
            a_min < b_max && a_max > b_min
        }
        match self {
            Orientation2D::Above => b.min.y <= origin.min.y && d_intersects(b.min.x, b.max.x, origin.min.x, origin.max.x),
            Orientation2D::Left => b.min.x <= origin.min.x && d_intersects(b.min.y, b.max.y, origin.min.y, origin.max.y),
            Orientation2D::Below => b.max.y >= origin.max.y && d_intersects(b.min.x, b.max.x, origin.min.x, origin.max.x),
            Orientation2D::Right => b.max.x >= origin.max.x && d_intersects(b.min.y, b.max.y, origin.min.y, origin.max.y),
        }
    }

    /// Iterator that yields quadrants for efficient search in a quad-tree, if a point is inside a quadrant and
    /// passes the [`Orientation2D::point_is`] check it is in the orientation, them if it is within the `max_distance` it is valid.
    pub fn search_bounds(self, origin: PxPoint, max_distance: Px, spatial_bounds: PxBox) -> impl Iterator<Item = PxBox> + 'static {
        let mut bounds = PxRect::new(origin, PxSize::splat(max_distance));
        match self {
            Orientation2D::Above => {
                bounds.origin.x -= max_distance / Px(2);
                bounds.origin.y -= max_distance;
            }
            Orientation2D::Right => bounds.origin.y -= max_distance / Px(2),
            Orientation2D::Below => bounds.origin.x -= max_distance / Px(2),
            Orientation2D::Left => {
                bounds.origin.y -= max_distance / Px(2);
                bounds.origin.x -= max_distance;
            }
        }

        // oriented search is a 45º square in the direction specified, so we grow and cut the search quadrant like
        // in the "nearest with bounds" algorithm, but then cut again to only the part that fully overlaps the 45º
        // square, points found are then matched with the `Orientation2D::is` method.

        let max_quad = spatial_bounds.intersection_unchecked(&bounds.to_box2d());
        let mut is_none = max_quad.is_empty();

        let mut source_quad = PxRect::new(origin - PxVector::splat(Px(64)), PxSize::splat(Px(128))).to_box2d();
        let mut search_quad = source_quad.intersection_unchecked(&max_quad);
        is_none |= search_quad.is_empty();

        let max_diameter = max_distance * Px(2);

        let mut is_first = true;

        std::iter::from_fn(move || {
            let source_width = source_quad.width();
            if is_none {
                None
            } else if is_first {
                is_first = false;
                Some(search_quad)
            } else if source_width >= max_diameter {
                is_none = true;
                None
            } else {
                source_quad = source_quad.inflate(source_width, source_width);
                let mut new_search = source_quad.intersection_unchecked(&max_quad);
                if new_search == source_quad || new_search.is_empty() {
                    is_none = true; // filled bounds
                    return None;
                }

                match self {
                    Orientation2D::Above => {
                        new_search.max.y = search_quad.min.y;
                    }
                    Orientation2D::Right => {
                        new_search.min.x = search_quad.max.x;
                    }
                    Orientation2D::Below => {
                        new_search.min.y = search_quad.max.y;
                    }
                    Orientation2D::Left => {
                        new_search.max.x = search_quad.min.x;
                    }
                }

                search_quad = new_search;

                Some(search_quad)
            }
        })
    }
}
