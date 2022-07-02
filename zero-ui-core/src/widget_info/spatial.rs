use std::ops::ControlFlow;

use crate::{crate_util::FxHashSet, units::*};

// The QuadTree is implemented as a grid of 2048px squares that is each an actual quad-tree, the grid cell "trees" are
// sparsely allocated and loosely matched in a linear search for visits, items belong to the first grid cell that intersects.
//
// The actual quad-trees can have any number of "leaves" in each level, depending on the `QLevel`.

const MIN_QUAD: Px = Px(128);
const ROOT_QUAD: Px = Px(2024);

/// Items can be inserted in multiple quads, but only of the same level or larger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct QLevel(u8);
impl QLevel {
    fn from_size(size: PxSize) -> Self {
        Self::from_length(size.width.min(size.height))
    }

    fn from_length(length: Px) -> Self {
        if length <= MIN_QUAD {
            QLevel(250)
        } else if length <= Px(256) {
            QLevel(251)
        } else if length <= Px(512) {
            QLevel(253)
        } else if length <= Px(1024) {
            QLevel(254)
        } else {
            QLevel(255)
        }
    }
}

#[derive(Clone, Copy)]
struct PxSquare {
    origin: PxPoint,
    length: Px,
}

type PxBox = euclid::Box2D<Px, ()>;

pub(super) struct QuadTree {
    grid: Vec<QuadRoot>,
}
impl QuadTree {
    pub(super) fn new() -> Self {
        Self {
            grid: Vec::with_capacity(1),
        }
    }

    pub(super) fn insert(&mut self, item: ego_tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();

        for r in &mut self.grid {
            if r.root_bounds.contains(item_bounds) {
                r.insert(item, item_bounds, item_level);
                return;
            }
        }

        let q = ROOT_QUAD.0;
        let min = (item_bounds.min.x.0 / q, item_bounds.min.y.0 / q);
        let max = (item_bounds.max.x.0 / q, item_bounds.max.y.0 / q);
        for cell_x in min.0..=max.0 {
            'next_cell: for cell_y in min.1..=max.1 {
                let root_origin = PxPoint::new(Px(cell_x * q), Px(cell_y * q));

                for r in &mut self.grid {
                    if r.root_bounds.origin == root_origin {
                        r.insert(item, item_bounds, item_level);
                        continue 'next_cell;
                    }
                }

                let mut root = QuadRoot::new(root_origin);
                root.insert(item, item_bounds, item_level);
                self.grid.push(root);
            }
        }
    }

    pub(super) fn remove(&mut self, item: ego_tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();

        self.grid.retain_mut(|r| {
            if r.loose_bounds.contains_box(&item_bounds) {
                r.remove(item, item_bounds, item_level);
                !r.is_empty()
            } else {
                true
            }
        });
    }

    pub(super) fn visit<B>(
        &self,
        mut include: impl FnMut(PxBox) -> bool,
        mut visit: impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for r in &self.grid {
            if include(r.loose_bounds) {
                r.root.visit(r.root_bounds, &mut include, &mut visit)?;
            }
        }
        ControlFlow::Continue(())
    }

    pub(super) fn visit_dedup<B>(
        &self,
        mut include: impl FnMut(PxBox) -> bool,
        mut visit: impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for r in &self.grid {
            if include(r.loose_bounds) {
                let mut visited = FxHashSet::default();
                let mut visit = |id| {
                    if visited.insert(id) {
                        visit(id)
                    } else {
                        ControlFlow::Continue(())
                    }
                };

                r.root.visit(r.root_bounds, &mut include, &mut visit)?;
            }
        }
        ControlFlow::Continue(())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }

    pub(super) fn clear(&mut self) {
        self.grid.clear();
    }
}

struct QuadRoot {
    root_bounds: PxSquare,
    root: QuadNode,
    loose_bounds: PxBox,
    len: usize,
}
impl QuadRoot {
    pub(super) fn new(grid_origin: PxPoint) -> Self {
        Self {
            root: QuadNode::default(),
            root_bounds: PxSquare {
                origin: grid_origin,
                length: ROOT_QUAD,
            },
            loose_bounds: PxBox::zero(),
            len: 0,
        }
    }

    fn insert(&mut self, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if self.is_empty() {
            self.loose_bounds = item_bounds;
        } else {
            self.loose_bounds.min = self.loose_bounds.min.min(item_bounds.min);
            self.loose_bounds.max = self.loose_bounds.max.max(item_bounds.max);
        }

        self.root.insert(self.root_bounds, item, item_bounds, item_level);
        self.len += 1;
    }

    fn remove(&mut self, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        self.root.remove(self.root_bounds, item, item_bounds, item_level);
        self.len -= 1;
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Default)]
struct QuadNode {
    inner: Option<Box<[QuadNode; 4]>>,
    items: Vec<ego_tree::NodeId>,
}
impl QuadNode {
    fn insert(&mut self, self_bounds: PxSquare, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if let Some(q) = self_bounds.split() {
            let q_level = QLevel::from_length(q[0].length);
            if q_level >= item_level {
                for (quad, q_bounds) in self.inner.get_or_insert_with(Self::inner_defaults).iter_mut().zip(q) {
                    if q_bounds.intersects(item_bounds) {
                        quad.insert(q_bounds, item, item_bounds, item_level);
                    }
                }
                return;
            }
        }
        self.items.push(item);
    }

    fn remove(&mut self, self_bounds: PxSquare, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if let Some(q) = self_bounds.split() {
            let q_level = QLevel::from_length(q[0].length);
            if q_level >= item_level {
                for (quad, q_bounds) in self.inner.as_mut().unwrap().iter_mut().zip(q) {
                    if q_bounds.intersects(item_bounds) {
                        quad.remove(q_bounds, item, item_bounds, item_level);
                    }
                }
                if self.is_inner_empty() {
                    self.inner = None;
                }
                return;
            }
        }

        if let Some(i) = self.items.iter().position(|i| *i == item) {
            self.items.remove(i);
        }
    }

    fn visit<B>(
        &self,
        self_bounds: PxSquare,
        include: &mut impl FnMut(PxBox) -> bool,
        visit: &mut impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for item in &self.items {
            visit(*item)?;
        }

        if let Some(inner) = &self.inner {
            for (inner, bounds) in inner.iter().zip(self_bounds.split().unwrap()) {
                if include(bounds.to_box()) {
                    inner.visit(bounds, include, visit)?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    fn is_inner_empty(&self) -> bool {
        self.inner.as_ref().map(|i| i.iter().all(|a| a.is_empty())).unwrap_or(true)
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty() && self.is_inner_empty()
    }

    fn inner_defaults() -> Box<[QuadNode; 4]> {
        Box::new([QuadNode::default(), QuadNode::default(), QuadNode::default(), QuadNode::default()])
    }
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

    fn contains(self, b: PxBox) -> bool {
        self.to_box().contains_box(&b)
    }
}
