use std::num::NonZeroU32;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct NodeId(NonZeroU32);
assert_non_null!(NodeId);
impl NodeId {
    fn new(i: usize) -> Self {
        debug_assert!(i < u32::MAX as usize);
        // SAFETY: +1
        Self(unsafe { NonZeroU32::new_unchecked((i + 1) as u32) })
    }

    fn get(self) -> usize {
        (self.0.get() - 1) as usize
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
            children: None,
            value: root,
        });

        Tree { nodes }
    }

    pub fn index(&self, id: NodeId) -> NodeRef<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[id.get()];
        NodeRef { tree: self, id }
    }

    pub fn get(&self, id: NodeId) -> Option<NodeRef<T>> {
        if self.nodes.len() < id.get() {
            Some(NodeRef { tree: self, id })
        } else {
            None
        }
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

struct Node<T> {
    parent: Option<NodeId>,
    prev_sibling: Option<NodeId>,
    next_sibling: Option<NodeId>,
    children: Option<(NodeId, NodeId)>,
    value: T,
}
assert_size_of!(Node<()>, 5 * 4); // non-null works for the `children` field too.

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
        self.tree.nodes[self.id.get()]
            .children
            .map(|(p, _)| NodeRef { tree: self.tree, id: p })
    }

    pub fn last_child(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.id.get()]
            .children
            .map(|(_, p)| NodeRef { tree: self.tree, id: p })
    }

    pub fn has_children(&self) -> bool {
        self.tree.nodes[self.id.get()].children.is_some()
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
        let index = NodeId::new(self.tree.nodes.len());

        let self_node = &mut self.tree.nodes[self.id.get()];
        let mut new_node = Node {
            parent: Some(self.id),
            prev_sibling: None,
            next_sibling: None,
            children: None,
            value,
        };

        if let Some((_, last)) = &mut self_node.children {
            let prev_last = *last;
            new_node.prev_sibling = Some(prev_last);
            *last = index;
            self.tree.nodes[prev_last.get()].next_sibling = Some(index);
        } else {
            self_node.children = Some((index, index));
        }

        self.tree.nodes.push(new_node);

        NodeMut {
            tree: self.tree,
            id: index,
        }
    }

    pub fn push_reuse(&mut self, child: NodeRef<T>, reuse: &mut impl FnMut(&T) -> T) -> NodeMut<T> {
        let mut clone = self.push_child(reuse(child.value()));
        if let Some(child) = child.first_child() {
            clone.push_reuse(child, reuse);

            while let Some(child) = child.next_sibling() {
                clone.push_reuse(child, reuse);
            }
        }
        clone
    }

    pub fn value(&mut self) -> &mut T {
        &mut self.tree.nodes[self.id.get()].value
    }
}
