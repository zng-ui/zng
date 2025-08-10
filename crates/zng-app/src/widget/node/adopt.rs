use parking_lot::Mutex;

use super::*;
use std::{mem, sync::Arc};

/// Represents a node setup to dynamically swap child.
///
/// Any property node can be made adoptive by wrapping it with this node.
pub struct AdoptiveNode {
    child: Arc<Mutex<UiNode>>,
    node: UiNode,
    is_inited: bool,
}
impl AdoptiveNode {
    /// Create the adoptive node, the [`AdoptiveChildNode`] must be used as the child of the created node.
    ///
    /// The created node is assumed to not be inited.
    pub fn new(create: impl FnOnce(AdoptiveChildNode) -> UiNode) -> Self {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child);
        Self {
            child,
            node,
            is_inited: false,
        }
    }

    /// Create the adoptive node using a closure that can fail.
    ///
    /// The created node is assumed to not be inited.
    pub fn try_new<E>(create: impl FnOnce(AdoptiveChildNode) -> Result<UiNode, E>) -> Result<Self, E> {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child)?;
        Ok(Self {
            child,
            node,
            is_inited: false,
        })
    }

    /// Replaces the child node.
    ///
    /// Returns the previous child, the initial child is a [`UiNode::nil`].
    ///
    /// # Panics
    ///
    /// Panics if [`is_inited`](Self::is_inited).
    pub fn replace_child(&mut self, new_child: UiNode) -> UiNode {
        assert!(!self.is_inited);
        mem::replace(&mut *self.child.lock(), new_child)
    }

    /// Returns `true` if this node is inited.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    /// Into child reference and node.
    ///
    /// # Panics
    ///
    /// Panics if [`is_inited`](Self::is_inited).
    pub fn into_parts(self) -> (Arc<Mutex<UiNode>>, UiNode) {
        assert!(!self.is_inited);
        (self.child, self.node)
    }

    /// From parts, assumes the nodes are not inited and that `child` is the actual child of `node`.
    pub fn from_parts(child: Arc<Mutex<UiNode>>, node: UiNode) -> Self {
        Self {
            child,
            node,
            is_inited: false,
        }
    }
}

impl UiNodeImpl for AdoptiveNode {
    fn init(&mut self) {
        self.is_inited = true;
        self.node.init();
    }
    fn deinit(&mut self) {
        self.is_inited = false;
        self.node.deinit();
    }

    fn children_len(&self) -> usize {
        1
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        if index == 0 {
            visitor(&mut self.node);
        }
    }
}

/// Placeholder for the dynamic child of an adoptive node.
///
/// This node must be used as the child of the adoptive node, see [`AdoptiveNode::new`] for more details.
pub struct AdoptiveChildNode {
    child: Arc<Mutex<UiNode>>,
}
impl AdoptiveChildNode {
    fn nil() -> Self {
        Self {
            child: Arc::new(Mutex::new(UiNode::nil())),
        }
    }
}

impl UiNodeImpl for AdoptiveChildNode {
    fn children_len(&self) -> usize {
        1
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        if index == 0 {
            visitor(&mut self.child.lock())
        }
    }
}
