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
    value: T,
}
assert_size_of!(Node<()>, 4 * 4);
impl<T> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("prev_sibling", &self.prev_sibling)
            .field("next_sibling", &self.next_sibling)
            .field("last_child", &self.last_child)
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
            id: self.id.next(),// if we have a last child, we have a first one, just after `self`
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
        let start = self.id.get() + 1;
        if let Some(next) = self.next_sibling() {
            // descendants are only pushed directly after ancestors.
            let end = next.id.get();
            start..end
        } else {
            let mut p = self;
            while let Some(n) = p.parent() {
                if let Some(next) = n.next_sibling() {
                    let end = next.id.get();
                    return start..end;
                }
                p = n;
            }

            start..self.tree.len()
        }
    }

    pub fn is_descendent(self, maybe_ancestor: NodeRef<'a, T>) -> bool {
        let start = maybe_ancestor.id.get() + 1;

        let search_id = self.id.get();

        if search_id <= start {
            return false;
        }

        if let Some(next) = maybe_ancestor.next_sibling() {
            let end = next.id.get();

            return search_id < end;
        }

        let mut p = maybe_ancestor;
        while let Some(n) = p.parent() {
            if let Some(next) = n.next_sibling() {
                let end = next.id.get();

                return search_id < end;
            }
            p = n;
        }

        // tree.len
        true
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
        let new_id = NodeId::new(self.tree.nodes.len());

        let self_node = &mut self.tree.nodes[self.id.get()];
        let mut new_node = Node {
            parent: Some(self.id),
            prev_sibling: None,
            next_sibling: None,
            last_child: None,
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

    pub fn push_reuse(&mut self, child: NodeRef<T>, reuse: &mut impl FnMut(&T) -> T, inspect: &mut impl FnMut(NodeRef<T>)) -> NodeMut<T> {
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
        clone
    }

    pub fn value(&mut self) -> &mut T {
        &mut self.tree.nodes[self.id.get()].value
    }
}
