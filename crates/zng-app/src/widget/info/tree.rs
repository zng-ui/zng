use std::{fmt, num::NonZeroU32};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct NodeId(NonZeroU32);

impl NodeId {
    fn new(i: usize) -> Self {
        debug_assert!(i < u32::MAX as usize);
        // SAFETY: +1
        Self(NonZeroU32::new((i + 1) as u32).unwrap())
    }

    pub fn get(self) -> usize {
        (self.0.get() - 1) as usize
    }

    pub fn next(self) -> Self {
        let mut id = self.0.get();
        id = id.saturating_add(1);
        Self(NonZeroU32::new(id).unwrap())
    }
}
impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.get())
    }
}

pub(super) struct Tree<T> {
    nodes: Vec<Node<T>>,
}
impl<T> Tree<T> {
    pub(super) fn new(root: T) -> Self {
        let nodes = vec![Node {
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            last_child: None,
            descendants_end: 1,
            value: root,
        }];

        Tree { nodes }
    }

    pub fn index(&self, id: NodeId) -> NodeRef<'_, T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[id.get()];
        NodeRef { tree: self, id }
    }

    pub fn index_mut(&mut self, id: NodeId) -> NodeMut<'_, T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[id.get()];
        NodeMut { tree: self, id }
    }

    pub fn root(&self) -> NodeRef<'_, T> {
        self.index(NodeId::new(0))
    }

    pub fn root_mut(&mut self) -> NodeMut<'_, T> {
        self.index_mut(NodeId::new(0))
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn iter(&self) -> impl std::iter::ExactSizeIterator<Item = (NodeId, &T)> {
        self.nodes.iter().enumerate().map(|(i, n)| (NodeId::new(i), &n.value))
    }
}
impl<T> fmt::Debug for Tree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tree").field("nodes", &self.nodes).finish()
    }
}

struct Node<T> {
    parent: Option<NodeId>,
    prev_sibling: Option<NodeId>,
    next_sibling: Option<NodeId>,
    last_child: Option<NodeId>,
    descendants_end: u32,
    value: T,
}

impl<T> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("prev_sibling", &self.prev_sibling)
            .field("next_sibling", &self.next_sibling)
            .field("last_child", &self.last_child)
            .field("descendant_end", &self.descendants_end)
            .finish_non_exhaustive()
    }
}

pub(super) struct NodeRef<'a, T> {
    tree: &'a Tree<T>,
    id: NodeId,
}
impl<T> Clone for NodeRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for NodeRef<'_, T> {}
impl<'a, T> NodeRef<'a, T> {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn parent(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.id.get()].parent.map(|p| NodeRef { tree: self.tree, id: p })
    }

    pub fn prev_sibling(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.id.get()]
            .prev_sibling
            .map(|p| NodeRef { tree: self.tree, id: p })
    }

    pub fn next_sibling(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.id.get()]
            .next_sibling
            .map(|p| NodeRef { tree: self.tree, id: p })
    }

    pub fn has_siblings(&self) -> bool {
        let node = &self.tree.nodes[self.id.get()];
        node.prev_sibling.is_some() || node.next_sibling.is_some()
    }

    pub fn first_child(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.id.get()].last_child.map(|_| NodeRef {
            tree: self.tree,
            id: self.id.next(), // if we have a last child, we have a first one, just after `self`
        })
    }

    pub fn last_child(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.id.get()]
            .last_child
            .map(|p| NodeRef { tree: self.tree, id: p })
    }

    pub fn has_children(&self) -> bool {
        self.tree.nodes[self.id.get()].last_child.is_some()
    }

    pub fn children_count(&self) -> usize {
        let mut r = 0;
        if let Some(mut c) = self.first_child() {
            r += 1;

            while let Some(n) = c.next_sibling() {
                c = n;
                r += 1;
            }
        }
        r
    }

    pub fn descendants_range(self) -> std::ops::Range<usize> {
        let self_i = self.id.get();
        let start = self_i + 1;
        let end = self.tree.nodes[self_i].descendants_end as usize;
        start..end
    }

    pub fn self_and_descendants(self) -> TreeIter {
        let node = self.id.get();
        TreeIter {
            node,
            next: node,
            end: self.tree.nodes[self.id.get()].descendants_end as usize,
        }
    }

    pub fn value(&self) -> &'a T {
        &self.tree.nodes[self.id.get()].value
    }
}
impl<T> PartialEq for NodeRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub(super) struct NodeMut<'a, T> {
    tree: &'a mut Tree<T>,
    id: NodeId,
}
impl<T> NodeMut<'_, T> {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn push_child(&mut self, value: T) -> NodeMut<'_, T> {
        let len = self.tree.nodes.len();
        let new_id = NodeId::new(len);

        let self_node = &mut self.tree.nodes[self.id.get()];
        let mut new_node = Node {
            parent: Some(self.id),
            prev_sibling: None,
            next_sibling: None,
            last_child: None,
            descendants_end: len as u32 + 1,
            value,
        };

        if let Some(last) = &mut self_node.last_child {
            let prev_last = *last;
            new_node.prev_sibling = Some(prev_last);
            *last = new_id;
            self.tree.nodes[prev_last.get()].next_sibling = Some(new_id);
        } else {
            self_node.last_child = Some(new_id);
        }

        self.tree.nodes.push(new_node);

        NodeMut {
            tree: self.tree,
            id: new_id,
        }
    }

    pub fn push_reuse(&mut self, child: NodeRef<T>, reuse: &mut impl FnMut(&T) -> T) {
        let mut clone = self.push_child(reuse(child.value()));

        if let Some(mut child) = child.first_child() {
            clone.push_reuse(child, reuse);

            while let Some(c) = child.next_sibling() {
                child = c;
                clone.push_reuse(c, reuse);
            }
        }

        clone.close();
    }

    fn first_child(&mut self) -> Option<NodeMut<'_, T>> {
        self.tree.nodes[self.id.get()].last_child.map(|_| NodeMut {
            tree: self.tree,
            id: self.id.next(), // if we have a last child, we have a first one, just after `self`
        })
    }

    pub fn parallel_fold(&mut self, mut split: Tree<T>, take: &mut impl FnMut(&mut T) -> T) {
        if let Some(mut c) = split.root_mut().first_child() {
            self.parallel_fold_node(&mut c, take);

            let tree = c.tree;
            let mut child_idx = c.id.get();
            while let Some(id) = tree.nodes[child_idx].next_sibling {
                self.parallel_fold_node(&mut NodeMut { tree, id }, take);
                child_idx = id.get();
            }
        }
    }

    fn parallel_fold_node(&mut self, split: &mut NodeMut<T>, take: &mut impl FnMut(&mut T) -> T) {
        let mut clone = self.push_child(take(split.value()));

        if let Some(mut child) = split.first_child() {
            clone.parallel_fold_node(&mut child, take);

            let tree = child.tree;
            let mut child_idx = child.id.get();
            while let Some(id) = tree.nodes[child_idx].next_sibling {
                clone.parallel_fold_node(&mut NodeMut { tree, id }, take);
                child_idx = id.get();
            }
        }

        clone.close();
    }

    pub fn close(self) {
        let len = self.tree.len();
        self.tree.nodes[self.id.get()].descendants_end = len as u32;
    }

    pub fn value(&mut self) -> &mut T {
        &mut self.tree.nodes[self.id.get()].value
    }
}

pub(super) struct TreeIter {
    node: usize, // used for creating reverse iterator.

    next: usize,
    end: usize,
}
impl TreeIter {
    /// for a tree (a(a.a, a.b, a.c), b)
    /// yield [a, a.a, a.b, a.c, b]
    pub fn next(&mut self) -> Option<NodeId> {
        if self.next < self.end {
            let next = NodeId::new(self.next);
            self.next += 1;
            Some(next)
        } else {
            None
        }
    }

    /// Skip to the next sibling of the node last yielded by `next`.
    pub fn close<T>(&mut self, tree: &Tree<T>, yielded: NodeId) {
        let node = &tree.nodes[yielded.get()];
        if let Some(next_sibling) = node.next_sibling {
            self.next = next_sibling.get();
        } else if let Some(parent) = node.parent {
            let node = &tree.nodes[parent.get()];
            self.next = self.end.min(node.descendants_end as usize);
        } else {
            self.next = self.end;
        }
    }

    pub fn skip_to(&mut self, node: NodeId) {
        let node = node.get();
        if node > self.next {
            if node > self.end {
                self.next = self.end;
            } else {
                self.next = node;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.next
    }

    pub fn rev<T>(self, tree: &Tree<T>) -> RevTreeIter {
        let mut count = self.next - self.node;

        let mut iter = RevTreeIter {
            next: self.node,
            end: self.node,
            started: false,
        };

        while count > 0 {
            count -= 1;
            iter.next(tree);
        }

        iter
    }

    pub fn empty() -> Self {
        Self { node: 0, next: 0, end: 0 }
    }
}

pub(super) struct RevTreeIter {
    next: usize,
    end: usize,
    started: bool,
}
impl RevTreeIter {
    /// for a tree (a(a.a, a.b, a.c), b)
    /// yield [b, a, a.c, a.b, a.a]
    pub fn next<T>(&mut self, tree: &Tree<T>) -> Option<NodeId> {
        if self.next != self.end || !self.started {
            self.started = true;

            let next = NodeId::new(self.next);
            let node = &tree.nodes[self.next];

            if let Some(last_child) = node.last_child {
                self.next = last_child.get();
            } else if let Some(prev) = node.prev_sibling {
                self.next = prev.get();
            } else {
                let mut node = node;
                let mut changed = false;
                while let Some(parent) = node.parent {
                    let parent = parent.get();
                    if parent == self.end {
                        self.next = self.end;
                        changed = true;
                        break;
                    }

                    node = &tree.nodes[parent];

                    if let Some(prev) = node.prev_sibling {
                        self.next = prev.get();
                        changed = true;
                        break;
                    }
                }
                if !changed {
                    // back from root/child
                    self.next = self.end;
                }
            }

            Some(next)
        } else {
            None
        }
    }

    /// Skip to the next sibling of the node last yielded by `next`.
    pub fn close<T>(&mut self, tree: &Tree<T>, yielded: NodeId) {
        let mut node = &tree.nodes[yielded.get()];

        if let Some(prev) = node.prev_sibling {
            self.next = prev.get();
        } else {
            while let Some(parent) = node.parent {
                let parent = parent.get();

                if parent == self.end {
                    self.next = self.end;
                    break;
                }

                node = &tree.nodes[parent];

                if let Some(prev) = node.prev_sibling {
                    self.next = prev.get();
                    break;
                }
            }
        }
    }

    pub fn skip_to<T>(&mut self, tree: &Tree<T>, node: NodeId) {
        let node = node.get();
        if node > self.end {
            let root = &tree.nodes[self.end];
            if node >= root.descendants_end as usize {
                self.next = self.end;
            } else {
                self.next = node;
                self.started = true;
            }
        }
    }

    pub fn empty() -> Self {
        Self {
            next: 0,
            end: 0,
            started: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iter_tree() -> Tree<&'static str> {
        let mut tree = Tree::new("r");
        let mut r = tree.root_mut();
        let mut a = r.push_child("a");
        a.push_child("a.a");
        let mut ab = a.push_child("a.b");
        ab.push_child("a.b.a");
        ab.push_child("a.b.b");
        a.push_child("a.c");
        a.close();
        r.push_child("b");
        r.close();
        tree
    }

    #[test]
    fn iter_next() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants();

        let mut r = vec![];
        while let Some(id) = iter.next() {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["r", "a", "a.a", "a.b", "a.b.a", "a.b.b", "a.c", "b"]);
    }

    #[test]
    fn iter_rev() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants().rev(&tree);

        let mut r = vec![];
        while let Some(id) = iter.next(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["r", "b", "a", "a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
    }

    #[test]
    fn iter_not_root() {
        let tree = iter_tree();
        let mut iter = tree.root().first_child().unwrap().self_and_descendants();

        let mut r = vec![];
        while let Some(id) = iter.next() {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a", "a.a", "a.b", "a.b.a", "a.b.b", "a.c"]);
    }

    #[test]
    fn iter_rev_not_root() {
        let tree = iter_tree();
        let mut iter = tree.root().first_child().unwrap().self_and_descendants().rev(&tree);

        let mut r = vec![];
        while let Some(id) = iter.next(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a", "a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
    }

    #[test]
    fn iter_descendants() {
        let tree = iter_tree();
        let mut iter = tree.root().first_child().unwrap().self_and_descendants();
        iter.next();

        let mut r = vec![];
        while let Some(id) = iter.next() {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a.a", "a.b", "a.b.a", "a.b.b", "a.c"]);
    }

    #[test]
    fn iter_rev_descendants() {
        let tree = iter_tree();
        let mut iter = tree.root().first_child().unwrap().self_and_descendants().rev(&tree);
        iter.next(&tree);

        let mut r = vec![];
        while let Some(id) = iter.next(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
    }

    #[test]
    fn iter_close() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants();

        iter.next().unwrap(); // r
        let a = iter.next().unwrap();

        iter.close(&tree, a);

        let mut r = vec![];
        while let Some(id) = iter.next() {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["b"]);
    }

    #[test]
    fn iter_rev_close() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants().rev(&tree);

        iter.next(&tree).unwrap(); // r
        let b = iter.next(&tree).unwrap();

        iter.close(&tree, b);

        let mut r = vec![];
        while let Some(id) = iter.next(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a", "a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
    }

    #[test]
    fn iter_skip_to() {
        let tree = iter_tree();

        let mut iter = tree.root().self_and_descendants();
        let mut all = vec![];
        while let Some(id) = iter.next() {
            all.push(id);
        }

        for (i, id) in all.iter().enumerate() {
            let mut iter = tree.root().self_and_descendants();
            iter.skip_to(*id);

            let mut result = vec![];
            while let Some(id) = iter.next() {
                result.push(tree.nodes[id.get()].value);
            }

            let expected: Vec<_> = all[i..].iter().map(|id| tree.nodes[id.get()].value).collect();

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn iter_rev_skip_to() {
        let tree = iter_tree();

        let mut iter = tree.root().self_and_descendants().rev(&tree);
        let mut all = vec![];
        while let Some(id) = iter.next(&tree) {
            all.push(id);
        }

        for (i, id) in all.iter().enumerate() {
            let mut iter = tree.root().self_and_descendants().rev(&tree);
            iter.skip_to(&tree, *id);

            let mut result = vec![];
            while let Some(id) = iter.next(&tree) {
                result.push(tree.nodes[id.get()].value);
            }

            let expected: Vec<_> = all[i..].iter().map(|id| tree.nodes[id.get()].value).collect();
            assert_eq!(expected, result);
        }
    }
}
