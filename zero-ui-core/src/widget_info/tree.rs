use std::{fmt, num::NonZeroU32};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct NodeId(NonZeroU32);
assert_non_null!(NodeId);
impl NodeId {
    fn new(i: usize) -> Self {
        debug_assert!(i < u32::MAX as usize);
        // SAFETY: +1
        Self(unsafe { NonZeroU32::new_unchecked((i + 1) as u32) })
    }

    pub fn get(self) -> usize {
        (self.0.get() - 1) as usize
    }

    pub fn next(self) -> Self {
        let mut id = self.0.get();
        id = id.saturating_add(1);
        // SAFETY: sat +1
        Self(unsafe { NonZeroU32::new_unchecked(id) })
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
    pub(super) fn with_capacity(root: T, capacity: usize) -> Self {
        let mut nodes = Vec::with_capacity(capacity);
        nodes.push(Node {
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            last_child: None,
            descendants_end: 1,
            value: root,
        });

        Tree { nodes }
    }

    pub fn index(&self, id: NodeId) -> NodeRef<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[id.get()];
        NodeRef { tree: self, id }
    }

    pub fn index_mut(&mut self, id: NodeId) -> NodeMut<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[id.get()];
        NodeMut { tree: self, id }
    }

    pub fn root(&self) -> NodeRef<T> {
        self.index(NodeId::new(0))
    }

    pub fn root_mut(&mut self) -> NodeMut<T> {
        self.index_mut(NodeId::new(0))
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes(&self) -> impl std::iter::Iterator<Item = NodeRef<T>> + '_ {
        (0..self.len()).map(|i| NodeRef {
            tree: self,
            id: NodeId::new(i),
        })
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
assert_size_of!(Node<()>, 5 * 4);

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
impl<'a, T> Clone for NodeRef<'a, T> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree,
            id: self.id,
        }
    }
}
impl<'a, T> Copy for NodeRef<'a, T> {}
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
            next: node,
            next_back: node,
            end: self.tree.nodes[self.id.get()].descendants_end as usize,
        }
    }

    pub fn value(&self) -> &'a T {
        &self.tree.nodes[self.id.get()].value
    }
}
impl<'a, T> PartialEq for NodeRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub(super) struct NodeMut<'a, T> {
    tree: &'a mut Tree<T>,
    id: NodeId,
}
impl<'a, T> NodeMut<'a, T> {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn push_child(&mut self, value: T) -> NodeMut<T> {
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

    pub fn push_reuse(&mut self, child: NodeRef<T>, reuse: &mut impl FnMut(&T) -> T, inspect: &mut impl FnMut(NodeRef<T>)) {
        let mut clone = self.push_child(reuse(child.value()));
        inspect(NodeRef {
            tree: clone.tree,
            id: clone.id,
        });

        if let Some(mut child) = child.first_child() {
            clone.push_reuse(child, reuse, inspect);

            while let Some(c) = child.next_sibling() {
                child = c;
                clone.push_reuse(c, reuse, inspect);
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
    next: usize,
    end: usize,

    next_back: usize,
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

    /// for a tree (a(a.a, a.b, a.c), b)
    /// yield [b, a, a.c, a.b, a.a]
    pub fn next_back<T>(&mut self, tree: &Tree<T>) -> Option<NodeId> {
        // * Can we make this happen without adding branching in `next`?
        // * Update `end` After each `prev_sibling`?

        if self.next < self.end {
            let next = NodeId::new(self.next_back);

            let node = &tree.nodes[self.next_back];
            if let Some(last_child) = node.last_child {
                self.next_back = last_child.get();
            } else if let Some(prev_sibling) = node.prev_sibling {
                self.next_back = prev_sibling.get();
                self.end -= 1;
            } else {
                let mut node = node;
                loop {
                    if let Some(parent) = node.parent {
                        node = &tree.nodes[parent.get()];
                        if let Some(prev_sibling) = node.prev_sibling {
                            self.next_back = prev_sibling.get();
                            self.end -= 1;
                            break;
                        }
                    } else {
                        self.end = self.next;
                        break;
                    }
                }
            }

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

    /// Skip to the prev sibling of the node last yielded by `next_back`.
    pub fn close_back<T>(&mut self, tree: &Tree<T>, yielded: NodeId) {
        let node = &tree.nodes[yielded.get()];
        if let Some(prev_sibling) = node.prev_sibling {
            self.next_back = prev_sibling.get();
            self.end = tree.nodes[self.next_back].descendants_end as usize;
        } else {
            let mut node = node;
            while let Some(parent) = node.parent {
                node = &tree.nodes[parent.get()];
                if let Some(prev_sibling) = node.prev_sibling {
                    self.next_back = self.next.max(prev_sibling.get() as usize);
                    todo!("update front to match");
                    return;
                }
            }

            // else
            self.end = self.next;
            self.next_back = self.next;
        }
    }

    pub fn skip_to(&mut self, node: NodeId) {
        let node = node.get() as usize;
        if node > self.next {
            if node > self.end {
                self.next = self.end;
            } else {
                self.next = node;
            }
        }
    }

    pub fn skip_back_to(&mut self, node: NodeId) {
        let node = node.get() as usize;
        if node > self.next {
            if node > self.end {
                self.next = self.end;
            } else {
                self.end = node;
                self.next_back = node;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn iter_tree() -> Tree<&'static str> {
        let mut tree = Tree::with_capacity("r", 6);
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
    fn iter_prev() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants();

        let mut r = vec![];
        while let Some(id) = iter.next_back(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["r", "b", "a", "a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
    }

    #[test]
    fn iter_next_not_root() {
        let tree = iter_tree();
        let mut iter = tree.root().first_child().unwrap().self_and_descendants();

        let mut r = vec![];
        while let Some(id) = iter.next() {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a", "a.a", "a.b", "a.b.a", "a.b.b", "a.c"]);
    }

    #[test]
    fn iter_prev_not_root() {
        let tree = iter_tree();
        let mut iter = tree.root().first_child().unwrap().self_and_descendants();

        let mut r = vec![];
        while let Some(id) = iter.next_back(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a", "a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
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
    fn iter_close_back() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants();

        iter.next_back(&tree).unwrap(); // r
        let b = iter.next_back(&tree).unwrap();

        iter.close_back(&tree, b);

        let mut r = vec![];
        while let Some(id) = iter.next_back(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["a", "a.c", "a.b", "a.b.b", "a.b.a", "a.a"]);
    }

    #[test]
    fn iter_both_ends() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants();
        let r_start = iter.next().unwrap();
        let r_end = iter.next_back(&tree).unwrap();

        assert_eq!(tree.index(r_start).value(), tree.index(r_end).value());
    }

    #[test]
    fn iter_both_ends_closed_back() {
        let tree = iter_tree();
        let mut iter = tree.root().self_and_descendants();
        iter.next_back(&tree).unwrap(); // r
        iter.next_back(&tree).unwrap(); // b

        let mut r = vec![];
        while let Some(id) = iter.next() {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["r", "a", "a.a", "a.b", "a.b.a", "a.b.b", "a.c"]);
    }

    #[test]
    fn iter_both_ends_closed_front() {
        let tree = iter_tree();

        let mut iter = tree.root().self_and_descendants();
        for _ in 0..["r", "a", "a.a", "a.b", "a.b.a", "a.b.b", "a.c"].len() {
            iter.next().unwrap();
        }

        let mut r = vec![];
        while let Some(id) = iter.next_back(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["r", "b"]);
    }

    #[test]
    fn iter_both_ends_closed_front2() {
        let tree = iter_tree();

        let mut iter = tree.root().self_and_descendants();
        for _ in 0..["r", "a", "a.a", "a.b", "a.b.a"].len() {
            iter.next().unwrap();
        }

        let mut r = vec![];
        while let Some(id) = iter.next_back(&tree) {
            r.push(*tree.index(id).value());
        }

        assert_eq!(r, vec!["r", "b", "a", "a.c", "a.b", "a.b.b"]);
    }
}
