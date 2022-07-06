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
    pub(super) fn new(root: T) -> Self {
        Self::with_capacity(root, 1)
    }

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

    pub fn index(&self, index: NodeId) -> NodeRef<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[index.get()];
        NodeRef { tree: self, id: index }
    }

    pub fn index_mut(&mut self, index: NodeId) -> NodeMut<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[index.get()];
        NodeMut { tree: self, id: index }
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
impl<'a, T> NodeRef<'a, T> {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn tree(&self) -> &'a Tree<T> {
        self.tree
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

    pub fn append(self, value: T) -> NodeMut<'a, T> {
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

    pub fn value(&mut self) -> &mut T {
        &mut self.tree.nodes[self.id.get()].value
    }
}
