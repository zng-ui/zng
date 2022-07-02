use std::{mem, ops::ControlFlow};

use crate::{crate_util::FxHashSet, units::*};

const MIN_QUAD: Px = Px(20);

/// Items can be inserted in multiple quads, but only of the same level or greater.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct QLevel(u8);
impl QLevel {
    fn from_size(size: PxSize) -> Self {
        Self::from_length(size.width.max(size.height))
    }

    fn from_length(length: Px) -> Self {
        if length > Px(500) {
            QLevel(0)
        } else if length > Px(100) {
            QLevel(1)
        } else if length > Px(80) {
            QLevel(2)
        } else if length > Px(40) {
            QLevel(3)
        } else if length > Px(20) {
            QLevel(4)
        } else {
            QLevel(5)
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
    root: QuadNode,
    root_bounds: PxSquare,
    max_depth: u32,
}
impl QuadTree {
    pub(super) fn new() -> Self {
        Self {
            root: QuadNode::default(),
            root_bounds: PxSquare {
                origin: PxPoint::zero(),
                length: Px(1000),
            },
            max_depth: 8,
        }
    }

    pub(super) fn insert(&mut self, item: ego_tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();

        if self.root_bounds.contains(item_bounds) {
            self.root.insert(self.root_bounds, self.max_depth, item, item_bounds, item_level);
        } else {
            let grow_up = item_bounds.min.y < self.root_bounds.origin.y;
            let grow_left = item_bounds.min.x < self.root_bounds.origin.x;

            if grow_up {
                self.root_bounds.origin.y -= self.root_bounds.length;
            }
            if grow_left {
                self.root_bounds.origin.x -= self.root_bounds.length;
            }

            self.root_bounds.length *= Px(2);
            self.max_depth += 1;

            let mut root_inner = QuadNode::inner_defaults();
            let old_root = mem::take(&mut self.root);
            if grow_up && grow_left {
                root_inner[3] = old_root;
            } else if grow_up {
                root_inner[2] = old_root;
            } else if grow_left {
                root_inner[1] = old_root;
            } else {
                root_inner[0] = old_root;
            }
            self.root.inner = Some(root_inner);

            self.insert(item, item_bounds.to_rect());
        }
    }

    pub(super) fn remove(&mut self, item: ego_tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();
        self.root.remove(self.root_bounds, 0, item, item_bounds, item_level);

        if self.root.is_empty() {
            self.clear();
        }
    }

    pub(super) fn visit<B>(
        &self,
        mut include: impl FnMut(PxBox) -> bool,
        mut visit: impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        if include(self.root_bounds.to_box()) {
            self.root.visit(self.root_bounds, &mut include, &mut visit)?;
        }
        ControlFlow::Continue(())
    }

    pub(super) fn visit_dedup<B>(
        &self,
        mut include: impl FnMut(PxBox) -> bool,
        mut visit: impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        let mut visited = FxHashSet::default();
        self.visit(include, |id| if visited.insert(id) { visit(id) } else { ControlFlow::Continue(()) })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    pub(super) fn clear(&mut self) {
        *self = Self::new();
    }
}

#[derive(Default)]
struct QuadNode {
    inner: Option<Box<[QuadNode; 4]>>,
    items: Vec<ego_tree::NodeId>,
}
impl QuadNode {
    fn insert(&mut self, self_bounds: PxSquare, self_depth: u32, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if self_depth > 0 {
            if let Some(q) = self_bounds.split() {
                let q_level = QLevel::from_length(q[0].length);
                if q_level <= item_level {
                    let q_depth = self_depth - 1;
                    for (quad, q_bounds) in self.inner.get_or_insert_with(Self::inner_defaults).iter_mut().zip(q) {
                        if q_bounds.intersects(item_bounds) {
                            quad.insert(q_bounds, q_depth, item, item_bounds, item_level);
                        }
                    }
                    return;
                }
            }
        }
        self.items.push(item);
    }

    fn remove(&mut self, self_bounds: PxSquare, self_depth: u32, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if self_depth > 0 {
            if let Some(q) = self_bounds.split() {
                let q_level = QLevel::from_length(q[0].length);
                if q_level <= item_level {
                    let q_depth = self_depth + 1;
                    for (quad, q_bounds) in self.inner.as_mut().unwrap().iter_mut().zip(q) {
                        if q_bounds.intersects(item_bounds) {
                            quad.remove(q_bounds, q_depth, item, item_bounds, item_level);
                        }
                    }
                    if self.is_inner_empty() {
                        self.inner = None;
                    }
                    return;
                }
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
