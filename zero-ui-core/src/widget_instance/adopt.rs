use std::{rc::Rc, cell::RefCell, mem};
use super::*;

/// Represents a node setup to dynamically swap child.
///
/// Any property node can be made adoptive by wrapping it with this node.
pub struct AdoptiveNode<U> {
    child: Rc<RefCell<BoxedUiNode>>,
    node: U,
    is_inited: bool,
}
impl<U: UiNode> AdoptiveNode<U> {
    /// Create the adoptive node, the [`AdoptiveChildNode`] must be used as the *property child*.
    pub fn new(create: impl FnOnce(AdoptiveChildNode) -> U) -> Self {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child);
        Self {
            child,
            node,
            is_inited: false,
        }
    }

    /// Create the adoptive node with a constructor that can fail.
    pub fn try_new<E>(create: impl FnOnce(AdoptiveChildNode) -> Result<U, E>) -> Result<Self, E> {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child)?;
        Ok(Self {
            child,
            node,
            is_inited: false,
        })
    }

    /// Replaces the child node, panics if the node is inited.
    ///
    /// Returns the previous child, the initial child is a [`NilUiNode`].
    pub fn replace_child(&mut self, new_child: impl UiNode) -> BoxedUiNode {
        assert!(!self.is_inited);
        mem::replace(&mut *self.child.borrow_mut(), new_child.boxed())
    }

    /// Returns `true` if this node is initialized in a UI tree.
    pub fn is_inited(&self) -> bool {
        self.is_inited
    }

    /// Into child reference, node and if it is inited.
    pub fn into_parts(self) -> (Rc<RefCell<BoxedUiNode>>, U) {
        assert!(!self.is_inited);
        (self.child, self.node)
    }

    /// From parts, assumes not inited.
    pub fn from_parts(child: Rc<RefCell<BoxedUiNode>>, node: U) -> Self {
        Self {
            child,
            node,
            is_inited: false,
        }
    }
}
#[ui_node(
    delegate = &self.node,
    delegate_mut = &mut self.node,
)]
impl<U: UiNode> UiNode for AdoptiveNode<U> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.is_inited = true;
        self.node.init(ctx);
    }
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.is_inited = false;
        self.node.deinit(ctx);
    }
}

/// Placeholder for the dynamic child of an [`AdoptiveNode`].
///
/// This node must be used as the property child of the adoptive node.
pub struct AdoptiveChildNode {
    child: Rc<RefCell<BoxedUiNode>>,
}
impl AdoptiveChildNode {
    fn nil() -> Self {
        Self {
            child: Rc::new(RefCell::new(NilUiNode.boxed())),
        }
    }
}
#[ui_node(
    delegate = self.child.borrow(),
    delegate_mut = self.child.borrow_mut(),
)]
impl UiNode for AdoptiveChildNode {}