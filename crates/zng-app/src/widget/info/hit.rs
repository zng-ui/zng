use std::sync::{Arc, atomic::AtomicUsize};

use zng_layout::unit::{PxBox, PxCornerRadius, PxPoint, PxSize, PxTransform, PxVector};
use zng_view_api::display_list::{FrameValue, FrameValueUpdate};

use crate::widget::WidgetId;

/// Represents hit-test regions of a widget inner.
#[derive(Debug, Default)]
pub(crate) struct HitTestClips {
    items: Vec<HitTestItem>,
    segments: ParallelSegmentOffsets,
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

    pub fn push_rounded_rect(&mut self, rect: PxBox, mut radii: PxCornerRadius) {
        if radii == PxCornerRadius::zero() {
            self.push_rect(rect);
        } else {
            ensure_no_corner_overlap(&mut radii, rect.size());
            self.items.push(HitTestItem::Hit(HitTestPrimitive::RoundedRect(rect, radii)));
        }
    }

    pub fn push_clip_rounded_rect(&mut self, clip_rect: PxBox, mut radii: PxCornerRadius, clip_out: bool) {
        if radii == PxCornerRadius::zero() {
            self.push_clip_rect(clip_rect, clip_out);
        } else {
            ensure_no_corner_overlap(&mut radii, clip_rect.size());
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

    pub fn push_transform(&mut self, transform: FrameValue<PxTransform>) {
        self.items.push(HitTestItem::Transform(transform))
    }

    pub fn pop_transform(&mut self) {
        self.items.push(HitTestItem::PopTransform);
    }

    #[must_use]
    pub fn push_child(&mut self, widget: WidgetId) -> HitChildIndex {
        if let Some(HitTestItem::Child(c)) = self.items.last_mut() {
            *c = widget;
        } else {
            self.items.push(HitTestItem::Child(widget));
        }
        HitChildIndex(self.segments.id(), self.items.len() - 1)
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
                    let t = t.value();
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

        if let RelativeHitZ::Over(w) = z
            && let Some(c) = child
            && w == c
        {
            return RelativeHitZ::Front;
        }
        z
    }

    pub fn update_transform(&mut self, value: FrameValueUpdate<PxTransform>) {
        for item in &mut self.items {
            if let HitTestItem::Transform(FrameValue::Bind { id, value: t, .. }) = item
                && *id == value.id
            {
                *t = value.value;
                break;
            }
        }
    }

    /// Returns `true` if a clip that affects the `child` clips out the `window_point`.
    pub fn clip_child(&self, child: HitChildIndex, inner_transform: &PxTransform, window_point: PxPoint) -> bool {
        let mut transform_stack = vec![];
        let mut current_transform = inner_transform;
        let mut local_point = match inv_transform_point(current_transform, window_point) {
            Some(p) => p,
            None => return false,
        };

        let child = child.1 + self.segments.offset(child.0);

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
                    let t = t.value();
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

    pub(crate) fn parallel_split(&self) -> Self {
        Self {
            items: vec![],
            segments: self.segments.parallel_split(),
        }
    }

    pub(crate) fn parallel_fold(&mut self, mut split: Self) {
        self.segments.parallel_fold(split.segments, self.items.len());

        if self.items.is_empty() {
            self.items = split.items;
        } else {
            self.items.append(&mut split.items)
        }
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

    Transform(FrameValue<PxTransform>),
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

// matches webrender's implementation of the CSS spec:
// https://github.com/servo/webrender/blob/b198248e2c836ec61c4dfdf443f97684bc281a0c/webrender/src/border.rs#L168
pub fn ensure_no_corner_overlap(radii: &mut PxCornerRadius, size: PxSize) {
    let mut ratio = 1.0;
    let top_left_radius = &mut radii.top_left;
    let top_right_radius = &mut radii.top_right;
    let bottom_right_radius = &mut radii.bottom_right;
    let bottom_left_radius = &mut radii.bottom_left;

    let sum = top_left_radius.width + top_right_radius.width;
    if size.width < sum {
        ratio = f32::min(ratio, size.width.0 as f32 / sum.0 as f32);
    }

    let sum = bottom_left_radius.width + bottom_right_radius.width;
    if size.width < sum {
        ratio = f32::min(ratio, size.width.0 as f32 / sum.0 as f32);
    }

    let sum = top_left_radius.height + bottom_left_radius.height;
    if size.height < sum {
        ratio = f32::min(ratio, size.height.0 as f32 / sum.0 as f32);
    }

    let sum = top_right_radius.height + bottom_right_radius.height;
    if size.height < sum {
        ratio = f32::min(ratio, size.height.0 as f32 / sum.0 as f32);
    }

    if ratio < 1. {
        top_left_radius.width *= ratio;
        top_left_radius.height *= ratio;

        top_right_radius.width *= ratio;
        top_right_radius.height *= ratio;

        bottom_left_radius.width *= ratio;
        bottom_left_radius.height *= ratio;

        bottom_right_radius.width *= ratio;
        bottom_right_radius.height *= ratio;
    }
}

fn inv_transform_point(t: &PxTransform, point: PxPoint) -> Option<PxPoint> {
    t.inverse()?.project_point(point)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HitChildIndex(ParallelSegmentId, usize);
impl Default for HitChildIndex {
    fn default() -> Self {
        Self(ParallelSegmentId::MAX, Default::default())
    }
}

/// See [`ParallelSegmentOffsets`].
pub(crate) type ParallelSegmentId = usize;

/// Tracks the position of a range of items in a list that was built in parallel.
#[derive(Debug, Clone)]
pub(crate) struct ParallelSegmentOffsets {
    id: ParallelSegmentId,
    id_gen: Arc<AtomicUsize>,
    used: bool,
    segments: Vec<(ParallelSegmentId, usize)>,
}
impl Default for ParallelSegmentOffsets {
    fn default() -> Self {
        Self {
            id: 0,
            used: false,
            id_gen: Arc::new(AtomicUsize::new(1)),
            segments: vec![],
        }
    }
}
impl ParallelSegmentOffsets {
    /// Gets the segment ID and flags the current tracking offsets as used.
    pub fn id(&mut self) -> ParallelSegmentId {
        self.used = true;
        self.id
    }

    /// Resolve the `id` offset, after build.
    pub fn offset(&self, id: ParallelSegmentId) -> usize {
        self.segments
            .iter()
            .find_map(|&(i, o)| if i == id { Some(o) } else { None })
            .unwrap_or_else(|| {
                if id != 0 {
                    tracing::error!(target: "parallel", "segment offset for `{id}` not found");
                }
                0
            })
    }

    /// Start new parallel segment.
    pub fn parallel_split(&self) -> Self {
        Self {
            used: false,
            id: self.id_gen.fetch_add(1, atomic::Ordering::Relaxed),
            id_gen: self.id_gen.clone(),
            segments: vec![],
        }
    }

    /// Merge parallel segment at the given `offset`.
    pub fn parallel_fold(&mut self, mut split: Self, offset: usize) {
        if !Arc::ptr_eq(&self.id_gen, &split.id_gen) {
            tracing::error!(target: "parallel", "cannot parallel fold segments not split from the same root");
            return;
        }

        if offset > 0 {
            for (_, o) in &mut split.segments {
                *o += offset;
            }
        }

        if self.segments.is_empty() {
            self.segments = split.segments;
        } else {
            self.segments.append(&mut split.segments);
        }
        if split.used {
            self.segments.push((split.id, offset));
        }
    }
}
