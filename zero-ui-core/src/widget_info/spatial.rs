use std::{num::NonZeroUsize, ops::ControlFlow};

use crate::{crate_util::FxHashSet, units::*};

// The QuadTree is implemented as a grid of 2048px squares that is each an actual quad-tree.
//
// The actual quad-trees can have any number of "leaves" in each level, depending on the `QLevel`.

const MIN_QUAD: Px = Px(128);
const ROOT_QUAD: Px = Px(2048);

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
    bounds: PxBox,
}
impl QuadTree {
    pub(super) fn new() -> Self {
        Self {
            grid: Vec::with_capacity(1),
            bounds: PxBox::zero(),
        }
    }

    pub(super) fn bounds(&self) -> PxBox {
        self.bounds
    }

    pub(super) fn insert(&mut self, item: ego_tree::NodeId, item_bounds: PxRect) {
        let item_level = QLevel::from_size(item_bounds.size);
        let item_bounds = item_bounds.to_box2d();

        if self.is_empty() {
            self.bounds = item_bounds;
        } else {
            self.bounds.min = self.bounds.min.min(item_bounds.min);
            self.bounds.max = self.bounds.max.max(item_bounds.max);
        }

        for r in &mut self.grid {
            if r.root_bounds().contains_box(item_bounds) {
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
                    if r.grid_origin == root_origin {
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

    pub(super) fn visit<B>(
        &self,
        mut include: impl FnMut(PxBox) -> bool,
        mut visit: impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for r in &self.grid {
            if include(r.root_bounds().to_box()) {
                QuadNode(0).visit(&r.storage, r.root_bounds(), &mut include, &mut visit)?;
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
            if include(r.root_bounds().to_box()) {
                let mut visited = FxHashSet::default();
                let mut visit = |id| {
                    if visited.insert(id) {
                        visit(id)
                    } else {
                        ControlFlow::Continue(())
                    }
                };

                QuadNode(0).visit(&r.storage, r.root_bounds(), &mut include, &mut visit)?;
            }
        }
        ControlFlow::Continue(())
    }

    pub(super) fn visit_point<B>(&self, point: PxPoint, mut visit: impl FnMut(ego_tree::NodeId) -> ControlFlow<B>) -> ControlFlow<B> {
        for r in &self.grid {
            if r.root_bounds().contains(point) {
                return QuadNode(0).visit_point(&r.storage, r.root_bounds(), point, &mut visit);
            }
        }
        ControlFlow::Continue(())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }

    pub(super) fn clear(&mut self) {
        self.grid.clear();
        self.bounds = PxBox::zero();
    }
}

struct QuadStorage {
    nodes: Vec<QuadNodeData>,
    items: Vec<QuadItemData>,
}

struct QuadRoot {
    grid_origin: PxPoint,
    storage: QuadStorage,
}

#[derive(Clone, Copy)]
struct QuadNode(usize);

impl QuadRoot {
    pub(super) fn new(grid_origin: PxPoint) -> Self {
        Self {
            storage: QuadStorage::new(),
            grid_origin,
        }
    }

    fn root_bounds(&self) -> PxSquare {
        PxSquare {
            origin: self.grid_origin,
            length: ROOT_QUAD,
        }
    }

    fn insert(&mut self, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        let root_bounds = self.root_bounds();
        QuadNode(0).insert(&mut self.storage, root_bounds, item, item_bounds, item_level);
    }
}

impl QuadNode {
    fn insert(self, storage: &mut QuadStorage, self_bounds: PxSquare, item: ego_tree::NodeId, item_bounds: PxBox, item_level: QLevel) {
        if let Some(q) = self_bounds.split() {
            let q_level = QLevel::from_length(q[0].length);
            if q_level >= item_level {
                for (quad, q_bounds) in storage.split(self.0).zip(q) {
                    if q_bounds.intersects(item_bounds) {
                        QuadNode(quad).insert(storage, q_bounds, item, item_bounds, item_level);
                    }
                }
                return;
            }
            return;
        }
        storage.push_item(self.0, item);
    }

    fn visit<B>(
        self,
        storage: &QuadStorage,
        self_bounds: PxSquare,
        include: &mut impl FnMut(PxBox) -> bool,
        visit: &mut impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for item in storage.items(self.0) {
            visit(item)?;
        }

        let children = storage.children(self.0);
        if !children.is_empty() {
            for (child, bounds) in children.zip(self_bounds.split().unwrap()) {
                if include(bounds.to_box()) {
                    QuadNode(child).visit(storage, bounds, include, visit)?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    fn visit_point<B>(
        self,
        storage: &QuadStorage,
        self_bounds: PxSquare,
        point: PxPoint,
        visit: &mut impl FnMut(ego_tree::NodeId) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for item in storage.items(self.0) {
            visit(item)?;
        }

        let children = storage.children(self.0);
        if !children.is_empty() {
            if let Some((middle, length)) = self_bounds.middle() {
                let mut bounds = PxSquare {
                    origin: self_bounds.origin,
                    length,
                };
                let c = children.start;
                return if point.x < middle.x {
                    if point.y < middle.y {
                        QuadNode(c).visit_point(storage, bounds, point, visit)
                    } else {
                        bounds.origin.y += length;
                        QuadNode(c + 2).visit_point(storage, bounds, point, visit)
                    }
                } else if point.y < middle.y {
                    bounds.origin.x += length;
                    QuadNode(c + 1).visit_point(storage, bounds, point, visit)
                } else {
                    bounds.origin.x += length;
                    bounds.origin.y += length;
                    QuadNode(c + 3).visit_point(storage, bounds, point, visit)
                };
            }
        }
        ControlFlow::Continue(())
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

    fn middle(mut self) -> Option<(PxPoint, Px)> {
        self.length /= Px(2);

        if self.length >= MIN_QUAD {
            Some((self.origin + PxVector::new(self.length, self.length), self.length))
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

    fn contains_box(self, b: PxBox) -> bool {
        self.to_box().contains_box(&b)
    }

    fn contains(self, p: PxPoint) -> bool {
        self.to_box().contains(p)
    }
}

impl QuadStorage {
    fn new() -> Self {
        let mut r = QuadStorage {
            nodes: vec![],
            items: vec![],
        };
        r.nodes.push(QuadNodeData {
            children: None,
            items: None,
        });
        r
    }

    fn split(&mut self, node: usize) -> impl Iterator<Item = usize> {
        if let Some(c) = self.nodes[node].children {
            let c = c.get() - 1;
            c..c + 4
        } else {
            let c = self.nodes.len();
            for _ in 0..4 {
                self.nodes.push(QuadNodeData::default());
            }
            self.nodes[node].children = NonZeroUsize::new(c + 1);
            c..c + 4
        }
    }

    fn children(&self, node: usize) -> std::ops::Range<usize> {
        if let Some(c) = self.nodes[node].children {
            let c = c.get() - 1;
            c..c + 4
        } else {
            0..0
        }
    }

    fn items(&self, node: usize) -> impl Iterator<Item = ego_tree::NodeId> + '_ {
        struct ItemsIter<'a> {
            items: &'a Vec<QuadItemData>,
            next: Option<NonZeroUsize>,
        }
        impl<'a> Iterator for ItemsIter<'a> {
            type Item = ego_tree::NodeId;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(i) = self.next {
                    let r = &self.items[i.get() - 1];
                    self.next = r.next;
                    Some(r.item)
                } else {
                    None
                }
            }
        }
        ItemsIter {
            items: &self.items,
            next: self.nodes[node].items,
        }
    }

    fn push_item(&mut self, node: usize, item: ego_tree::NodeId) {
        let item_i = NonZeroUsize::new(self.items.len() + 1);
        self.items.push(QuadItemData { item, next: None });

        if let Some(ii) = self.nodes[node].items {
            let mut i = ii.get() - 1;
            while let Some(ii) = self.items[i].next {
                i = ii.get() - 1;
            }
            self.items[i].next = item_i;
        } else {
            self.nodes[node].items = item_i;
        }
    }
}
#[derive(Default)]
struct QuadNodeData {
    children: Option<NonZeroUsize>,
    items: Option<NonZeroUsize>,
}
struct QuadItemData {
    item: ego_tree::NodeId,
    next: Option<NonZeroUsize>,
}
