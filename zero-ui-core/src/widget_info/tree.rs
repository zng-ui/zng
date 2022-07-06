use std::num::NonZeroU32;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct NodeIndex(NonZeroU32);

assert_non_null!(NodeIndex);

impl NodeIndex {
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

struct Node<T> {
    parent: Option<NodeIndex>,
    prev_sibling: Option<NodeIndex>,
    next_sibling: Option<NodeIndex>,
    children: Option<(NodeIndex, NodeIndex)>,
    value: T,
}
assert_size_of!(Node<()>, 5 * 4); // non-null works for the `children` field too.

impl<T> Tree<T> {
    pub(super) fn new(root: T) -> Self {
        Tree {
            nodes: vec![Node {
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                children: None,
                value: root,
            }],
        }
    }

    pub fn index(&self, index: NodeIndex) -> NodeRef<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[index.get()];
        NodeRef { tree: self, index }
    }

    pub fn index_mut(&mut self, index: NodeIndex) -> NodeMut<T> {
        #[cfg(debug_assertions)]
        let _ = self.nodes[index.get()];
        NodeMut { tree: self, index }
    }

    pub fn root(&self) -> NodeRef<T> {
        self.index(NodeIndex::new(0))
    }

    pub fn root_mut(&mut self) -> NodeMut<T> {
        self.index_mut(NodeIndex::new(0))
    }
}

pub(super) struct NodeRef<'a, T> {
    tree: &'a Tree<T>,
    index: NodeIndex,
}
pub(super) struct NodeMut<'a, T> {
    tree: &'a mut Tree<T>,
    index: NodeIndex,
}

impl<'a, T> NodeRef<'a, T> {
    pub fn index(&self) -> NodeIndex {
        self.index
    }

    pub fn tree(&self) -> &'a Tree<T> {
        self.tree
    }

    pub fn parent(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.index.get()]
            .parent
            .map(|p| NodeRef { tree: self.tree, index: p })
    }

    pub fn prev_sibling(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.index.get()]
            .prev_sibling
            .map(|p| NodeRef { tree: self.tree, index: p })
    }

    pub fn next_sibling(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.index.get()]
            .next_sibling
            .map(|p| NodeRef { tree: self.tree, index: p })
    }

    pub fn first_child(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.index.get()]
            .children
            .map(|(p, _)| NodeRef { tree: self.tree, index: p })
    }

    pub fn last_child(&self) -> Option<NodeRef<'a, T>> {
        self.tree.nodes[self.index.get()]
            .children
            .map(|(_, p)| NodeRef { tree: self.tree, index: p })
    }

    pub fn value(&self) -> &'a T {
        &self.tree.nodes[self.index.get()].value
    }
}
impl<'a, T> NodeMut<'a, T> {
    pub fn index(&self) -> NodeIndex {
        self.index
    }

    pub fn append(self, value: T) -> NodeMut<'a, T> {
        let index = NodeIndex::new(self.tree.nodes.len());

        let self_node = &mut self.tree.nodes[self.index.get()];
        let mut new_node = Node {
            parent: Some(self.index),
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

        NodeMut { tree: self.tree, index }
    }

    pub fn value(&mut self) -> &mut T {
        &mut self.tree.nodes[self.index.get()].value
    }
}
