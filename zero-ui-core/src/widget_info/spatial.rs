use std::fmt;

use linear_map::LinearMap;
use smallvec::SmallVec;

use super::tree;
use crate::{
    crate_util::FxHashSet,
    render::{FrameBinding, FrameValue},
    units::*,
    WidgetId,
};

// The QuadTree is implemented as a grid of 2048px squares that is each an actual quad-tree.
//
// The actual quad-trees can have any number of "leaves" in each level, depending on the `QLevel`.

const MIN_QUAD: Px = Px(128);
const ROOT_QUAD: Px = Px(2048);

/// Items can be inserted in multiple quads, but only of the same level or larger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct QLevel(u8);
impl QLevel {
    fn from_size(size: PxSize) -> Self {
        Self::from_length(size.width.min(size.height))
    }

    fn from_length(length: Px) -> Self {
        if length <= MIN_QUAD {
            QLevel(0)
        } else if length <= Px(256) {
            QLevel(1)
        } else if length <= Px(512) {
            QLevel(2)
        } else if length <= Px(1024) {
            QLevel(3)
        } else {
            QLevel(4)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PxSquare {
    origin: PxPoint,
    length: Px,
}
impl PxSquare {
    fn split(mut self) -> Option<[Self; 4]> {
        self.length /= Px(2);

        if self.length >= MIN_QUAD {
            let mut r = [self; 4];
            r[1].origin.x += self.length;
            r[2].origin.y += self.length;
            r[3].origin += PxVector::splat(self.length);
            Some(r)
        } else {
            None
        }
    }

    fn to_box(self) -> PxBox {
        let max = self.origin + PxVector::splat(self.length);
        PxBox::new(self.origin, max)
    }

    fn intersects(self, b: PxBox) -> bool {
        self.to_box().intersects(&b)
    }
}

type QuadItems = SmallVec<[tree::NodeId; 8]>;

/// Items > 1024.
// ~16 KB, is this a problem? nah
#[derive(Default)]
struct Quad2048 {
    items: QuadItems,
    nodes: [Quad1024; 4],
}
/// Items > 512 && <= 1024.
#[derive(Default)]
struct Quad1024 {
    items: QuadItems,
    nodes: [Quad512; 4],
}
/// Items > 256 && <= 512.
#[derive(Default)]
struct Quad512 {
    items: QuadItems,
    nodes: [Quad256; 4],
}
/// Items > 128 && <= 256.
#[derive(Default)]
struct Quad256 {
    items: QuadItems,
    nodes: [Quad128; 4],
}
/// Items <= 128.
#[derive(Default)]
struct Quad128 {
    items: QuadItems,
}

macro_rules! impl_quad {
    ($Quad:ty, $n:tt) => {
        impl $Quad {
            fn level(&self) -> QLevel {
                QLevel::from_length(Px($n))
            }
            fn insert(&mut self, self_bounds: PxSquare, item: tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
                if item_level == self.level() {
                    self.items.push(item);
                } else {
                    for (inner, inner_bounds) in self.nodes.iter_mut().zip(self_bounds.split().unwrap()) {
                        if inner_bounds.intersects(item_bounds) {
                            inner.insert(inner_bounds, item, item_bounds, item_level);
                        }
                    }
                }
            }

            fn query(
                &self,
                self_bounds: PxSquare,
                include: impl Fn(PxBox) -> bool + Copy + 'static,
            ) -> impl Iterator<Item = tree::NodeId> + '_ {
                let inner = self
                    .nodes
                    .iter()
                    .zip(self_bounds.split().unwrap())
                    .filter_map(move |(inner, inner_bounds)| {
                        if include(inner_bounds.to_box()) {
                            Some(inner.query(inner_bounds, include))
                        } else {
                            None
                        }
                    })
                    .flatten();

                self.items.iter().copied().chain(inner)
            }

            fn query_debug(
                &self,
                self_bounds: PxSquare,
                include: impl Fn(PxBox) -> bool + Copy + 'static,
            ) -> impl Iterator<Item = PxBox> + '_ {
                let self_r = if self.items.is_empty() {
                    None
                } else {
                    Some(self_bounds.to_box())
                };

                let inner = self
                    .nodes
                    .iter()
                    .zip(self_bounds.split().unwrap())
                    .filter_map(move |(inner, inner_bounds)| {
                        if include(inner_bounds.to_box()) {
                            Some(inner.query_debug(inner_bounds, include))
                        } else {
                            None
                        }
                    })
                    .flatten();

                self_r.into_iter().chain(inner)
            }

            fn clear(&mut self) {
                self.items.clear();
                for n in &mut self.nodes {
                    n.clear();
                }
            }
        }
    };
}
impl Quad128 {
    fn level(&self) -> QLevel {
        QLevel::from_length(Px(128))
    }

    fn insert(&mut self, self_bounds: PxSquare, item: tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        debug_assert_eq!(self.level(), item_level);
        debug_assert_eq!(self_bounds.length, MIN_QUAD);
        debug_assert!(self_bounds.intersects(item_bounds));

        self.items.push(item);
    }

    fn query(&self, _: PxSquare, _: impl Fn(PxBox) -> bool + Copy + 'static) -> impl Iterator<Item = tree::NodeId> + '_ {
        self.items.iter().copied()
    }

    fn query_debug(&self, self_bounds: PxSquare, _: impl Fn(PxBox) -> bool + Copy + 'static) -> impl Iterator<Item = PxBox> + '_ {
        if self.items.is_empty() { None } else { Some(self_bounds.to_box()) }.into_iter()
    }

    fn clear(&mut self) {
        self.items.clear();
    }
}
impl_quad!(Quad256, 256);
impl_quad!(Quad512, 512);
impl_quad!(Quad1024, 1024);
impl_quad!(Quad2048, 2048);

#[derive(Default)]
pub(super) struct QuadTree {
    quads: LinearMap<PxPoint, Quad2048>,
    bounds: PxBox,
}
impl fmt::Debug for QuadTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuadTree").field("bounds", &self.bounds).finish_non_exhaustive()
    }
}
impl QuadTree {
    pub(super) fn bounds(&self) -> PxBox {
        self.bounds
    }

    pub(super) fn is_empty(&self) -> bool {
        self.bounds == PxBox::zero()
    }

    pub(super) fn clear(&mut self) {
        for (_, q) in self.quads.iter_mut() {
            q.clear();
        }
        self.bounds = PxBox::zero();
    }

    pub(super) fn insert(&mut self, item: tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();

        if self.is_empty() {
            self.bounds = item_bounds;
        } else {
            self.bounds.min = self.bounds.min.min(item_bounds.min);
            self.bounds.max = self.bounds.max.max(item_bounds.max);
        }

        // for each quad root.
        for root in self.root_quads(item_bounds) {
            self.quads
                .entry(root.origin)
                .or_insert_with(Default::default)
                .insert(root, item, item_bounds, item_level);
        }
    }

    fn root_quads(&self, bounds: PxBox) -> impl Iterator<Item = PxSquare> {
        let q = ROOT_QUAD.0;
        let min = (bounds.min.x.0 / q, bounds.min.y.0 / q);
        let max = (bounds.max.x.0 / q, bounds.max.y.0 / q);

        let x = min.0..=max.0;
        let y = min.1..=max.1;

        x.flat_map(move |x| {
            let y = y.clone();

            y.map(move |y| PxSquare {
                origin: PxPoint::new(Px(x * q), Px(y * q)),
                length: ROOT_QUAD,
            })
        })
    }

    pub(super) fn query(&self, include: impl Fn(PxBox) -> bool + Copy + 'static) -> impl Iterator<Item = tree::NodeId> + '_ {
        self.quads
            .iter()
            .filter_map(move |(origin, quad)| {
                let bounds = PxSquare {
                    origin: *origin,
                    length: ROOT_QUAD,
                };
                if include(bounds.to_box()) {
                    Some(quad.query(bounds, include))
                } else {
                    None
                }
            })
            .flatten()
    }

    pub(super) fn query_debug(&self, include: impl Fn(PxBox) -> bool + Copy + 'static) -> impl Iterator<Item = PxBox> + '_ {
        self.quads
            .iter()
            .filter_map(move |(origin, quad)| {
                let bounds = PxSquare {
                    origin: *origin,
                    length: ROOT_QUAD,
                };
                if include(bounds.to_box()) {
                    Some(quad.query_debug(bounds, include))
                } else {
                    None
                }
            })
            .flatten()
    }

    pub(super) fn query_unique(&self, include: impl Fn(PxBox) -> bool + Copy + 'static) -> impl Iterator<Item = tree::NodeId> + '_ {
        let mut visited = FxHashSet::default();
        self.query(include).filter(move |n| visited.insert(*n))
    }
}

/// Represents hit-test regions of a widget inner.
#[derive(Debug, Default)]
pub(crate) struct HitTestClips {
    items: Vec<HitTestItem>,
}
impl HitTestClips {
    /// Returns `true` if any hit-test clip is registered for this widget.
    pub fn is_hit_testable(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn push_rect(&mut self, rect: PxBox) {
        self.items.push(HitTestItem::Hit(HitTestPrimitive::Rect(rect)));
    }

    pub fn push_clip_rect(&mut self, clip_rect: PxBox, clip_out: bool) {
        self.items.push(HitTestItem::Clip(HitTestPrimitive::Rect(clip_rect), clip_out));
    }

    pub fn push_rounded_rect(&mut self, rect: PxBox, radii: PxCornerRadius) {
        if radii == PxCornerRadius::zero() {
            self.push_rect(rect);
        } else {
            self.items.push(HitTestItem::Hit(HitTestPrimitive::RoundedRect(rect, radii)));
        }
    }

    pub fn push_clip_rounded_rect(&mut self, clip_rect: PxBox, radii: PxCornerRadius, clip_out: bool) {
        if radii == PxCornerRadius::zero() {
            self.push_clip_rect(clip_rect, clip_out);
        } else {
            self.items
                .push(HitTestItem::Clip(HitTestPrimitive::RoundedRect(clip_rect, radii), clip_out));
        }
    }

    pub fn push_ellipse(&mut self, center: PxPoint, radii: PxSize) {
        self.items.push(HitTestItem::Hit(HitTestPrimitive::Ellipse(center, radii)));
    }

    pub fn push_clip_ellipse(&mut self, center: PxPoint, radii: PxSize, clip_out: bool) {
        self.items
            .push(HitTestItem::Clip(HitTestPrimitive::Ellipse(center, radii), clip_out));
    }

    pub fn pop_clip(&mut self) {
        self.items.push(HitTestItem::PopClip);
    }

    pub fn push_transform(&mut self, transform: FrameBinding<PxTransform>) {
        self.items.push(HitTestItem::Transform(transform))
    }

    pub fn pop_transform(&mut self) {
        self.items.push(HitTestItem::PopTransform);
    }

    #[must_use]
    pub fn push_child(&mut self, widget: WidgetId) -> usize {
        if let Some(HitTestItem::Child(c)) = self.items.last_mut() {
            *c = widget;
        } else {
            self.items.push(HitTestItem::Child(widget));
        }
        self.items.len() - 1
    }

    /// Hit-test the `point` against the items, returns the relative Z of the hit.
    pub fn hit_test_z(&self, inner_transform: &PxTransform, window_point: PxPoint) -> RelativeHitZ {
        let mut z = RelativeHitZ::NoHit;
        let mut child = None;

        let mut transform_stack = vec![];
        let mut current_transform = inner_transform;
        let mut local_point = match inv_transform_point(current_transform, window_point) {
            Some(p) => p,
            None => return RelativeHitZ::NoHit,
        };

        let mut items = self.items.iter();

        'hit_test: while let Some(item) = items.next() {
            match item {
                HitTestItem::Hit(prim) => {
                    if prim.contains(local_point) {
                        z = if let Some(inner) = child {
                            RelativeHitZ::Over(inner)
                        } else {
                            RelativeHitZ::Back
                        };
                    }
                }

                HitTestItem::Clip(prim, clip_out) => {
                    let skip = match clip_out {
                        true => prim.contains(local_point),
                        false => !prim.contains(local_point),
                    };

                    if skip {
                        // clip excluded point, skip all clipped shapes.
                        let mut clip_depth = 0;
                        'skip_clipped: for item in items.by_ref() {
                            match item {
                                HitTestItem::Clip(_, _) => {
                                    clip_depth += 1;
                                }
                                HitTestItem::PopClip => {
                                    if clip_depth == 0 {
                                        continue 'hit_test;
                                    }
                                    clip_depth -= 1;
                                }
                                HitTestItem::Child(w) => {
                                    child = Some(*w);
                                    continue 'skip_clipped;
                                }
                                _ => continue 'skip_clipped,
                            }
                        }
                    }
                }
                HitTestItem::PopClip => continue 'hit_test,

                HitTestItem::Transform(t) => {
                    let t = match t {
                        FrameBinding::Value(t) | FrameBinding::Binding(_, t) => t,
                    };
                    match inv_transform_point(t, local_point) {
                        Some(p) => {
                            // transform is valid, push previous transform and replace the local point.
                            transform_stack.push((current_transform, local_point));
                            current_transform = t;
                            local_point = p;
                        }
                        None => {
                            // non-invertible transform, skip all transformed shapes.
                            let mut transform_depth = 0;
                            'skip_transformed: for item in items.by_ref() {
                                match item {
                                    HitTestItem::Transform(_) => {
                                        transform_depth += 1;
                                    }
                                    HitTestItem::PopTransform => {
                                        if transform_depth == 0 {
                                            continue 'hit_test;
                                        }
                                        transform_depth -= 1;
                                    }
                                    HitTestItem::Child(w) => {
                                        child = Some(*w);
                                        continue 'skip_transformed;
                                    }
                                    _ => continue 'skip_transformed,
                                }
                            }
                        }
                    }
                }
                HitTestItem::PopTransform => {
                    (current_transform, local_point) = transform_stack.pop().unwrap();
                }

                HitTestItem::Child(w) => {
                    child = Some(*w);
                }
            }
        }

        if let (RelativeHitZ::Over(w), Some(c)) = (z, child) {
            if w == c {
                return RelativeHitZ::Front;
            }
        }
        z
    }

    pub fn update_transform(&mut self, value: FrameValue<PxTransform>) {
        for item in &mut self.items {
            if let HitTestItem::Transform(FrameBinding::Binding(key, t)) = item {
                if *key == value.key {
                    *t = value.value;
                    break;
                }
            }
        }
    }

    /// Returns `true` if a clip that affects the `child` clips out the `window_point`.
    pub fn clip_child(&self, child: usize, inner_transform: &PxTransform, window_point: PxPoint) -> bool {
        let mut transform_stack = vec![];
        let mut current_transform = inner_transform;
        let mut local_point = match inv_transform_point(current_transform, window_point) {
            Some(p) => p,
            None => return false,
        };

        let mut items = self.items[..child].iter();
        let mut clip = false;

        'clip: while let Some(item) = items.next() {
            match item {
                HitTestItem::Clip(prim, clip_out) => {
                    clip = match clip_out {
                        true => prim.contains(local_point),
                        false => !prim.contains(local_point),
                    };
                    if clip {
                        let mut clip_depth = 0;
                        'close_clip: for item in items.by_ref() {
                            match item {
                                HitTestItem::Clip(_, _) => clip_depth += 1,
                                HitTestItem::PopClip => {
                                    if clip_depth == 0 {
                                        clip = false; // was not a clip that covers the child.
                                        continue 'clip;
                                    }
                                    clip_depth -= 1;
                                }
                                _ => continue 'close_clip,
                            }
                        }
                    }
                }
                HitTestItem::PopClip => continue 'clip,
                HitTestItem::Transform(t) => {
                    let t = match t {
                        FrameBinding::Value(t) | FrameBinding::Binding(_, t) => t,
                    };
                    match inv_transform_point(t, local_point) {
                        Some(p) => {
                            // transform is valid, push previous transform and replace the local point.
                            transform_stack.push((current_transform, local_point));
                            current_transform = t;
                            local_point = p;
                        }
                        None => {
                            // non-invertible transform, skip all transformed shapes.
                            let mut transform_depth = 0;
                            'skip_transformed: for item in items.by_ref() {
                                match item {
                                    HitTestItem::Transform(_) => {
                                        transform_depth += 1;
                                    }
                                    HitTestItem::PopTransform => {
                                        if transform_depth == 0 {
                                            continue 'clip;
                                        }
                                        transform_depth -= 1;
                                    }
                                    _ => continue 'skip_transformed,
                                }
                            }
                        }
                    }
                }
                HitTestItem::PopTransform => {
                    (current_transform, local_point) = transform_stack.pop().unwrap();
                }
                _ => continue 'clip,
            }
        }

        clip
    }
}

/// Hit-test result on a widget relative to it's descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeHitZ {
    /// Widget was not hit.
    NoHit,
    /// Widget was hit on a hit-test shape rendered before the widget descendants.
    Back,
    /// Widget was hit on a hit-test shape rendered after the child.
    Over(WidgetId),
    /// Widget was hit on a hit-test shape rendered after the widget descendants.
    Front,
}

#[derive(Debug)]
enum HitTestPrimitive {
    Rect(PxBox),
    RoundedRect(PxBox, PxCornerRadius),
    Ellipse(PxPoint, PxSize),
}
impl HitTestPrimitive {
    fn contains(&self, point: PxPoint) -> bool {
        match self {
            HitTestPrimitive::Rect(r) => r.contains(point),
            HitTestPrimitive::RoundedRect(rect, radii) => rounded_rect_contains(rect, radii, point),
            HitTestPrimitive::Ellipse(center, radii) => ellipse_contains(*radii, *center, point),
        }
    }
}
#[derive(Debug)]
enum HitTestItem {
    Hit(HitTestPrimitive),

    Clip(HitTestPrimitive, bool),
    PopClip,

    Transform(FrameBinding<PxTransform>),
    PopTransform,

    Child(WidgetId),
}

fn rounded_rect_contains(rect: &PxBox, radii: &PxCornerRadius, point: PxPoint) -> bool {
    if !rect.contains(point) {
        return false;
    }

    let top_left_center = rect.min + radii.top_left.to_vector();
    if top_left_center.x > point.x && top_left_center.y > point.y && !ellipse_contains(radii.top_left, top_left_center, point) {
        return false;
    }

    let bottom_right_center = rect.max - radii.bottom_right.to_vector();
    if bottom_right_center.x < point.x
        && bottom_right_center.y < point.y
        && !ellipse_contains(radii.bottom_right, bottom_right_center, point)
    {
        return false;
    }

    let top_right = PxPoint::new(rect.max.x, rect.min.y);
    let top_right_center = top_right + PxVector::new(-radii.top_right.width, radii.top_right.height);
    if top_right_center.x < point.x && top_right_center.y > point.y && !ellipse_contains(radii.top_right, top_right_center, point) {
        return false;
    }

    let bottom_left = PxPoint::new(rect.min.x, rect.max.y);
    let bottom_left_center = bottom_left + PxVector::new(radii.bottom_left.width, -radii.bottom_left.height);
    if bottom_left_center.x > point.x && bottom_left_center.y < point.y && !ellipse_contains(radii.bottom_left, bottom_left_center, point) {
        return false;
    }

    true
}

fn ellipse_contains(radii: PxSize, center: PxPoint, point: PxPoint) -> bool {
    let h = center.x.0 as f64;
    let k = center.y.0 as f64;

    let a = radii.width.0 as f64;
    let b = radii.height.0 as f64;

    let x = point.x.0 as f64;
    let y = point.y.0 as f64;

    let p = ((x - h).powi(2) / a.powi(2)) + ((y - k).powi(2) / b.powi(2));

    p <= 1.0
}

fn inv_transform_point(t: &PxTransform, point: PxPoint) -> Option<PxPoint> {
    t.inverse()?.transform_point(point)
}
