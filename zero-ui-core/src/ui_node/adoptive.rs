use std::{cell::RefCell, mem, rc::Rc};

use crate::NilUiNode;
use crate::{context::WidgetContext, impl_ui_node, BoxedUiNode, UiNode};

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
        let child = Rc::new(RefCell::new(NilUiNode.boxed()));
        let node = create(AdoptiveChildNode { child: child.clone() });
        Self {
            child,
            node,
            is_inited: false,
        }
    }

    /// Replaces the child node, if the adoptive node is inited, the previous child will deinit and the new child will be inited.
    ///
    /// Returns the previous child, the initial child is a [`NilUiNode`].
    pub fn replace_child(&mut self, ctx: &mut WidgetContext, new_child: impl UiNode) -> BoxedUiNode {
        let mut new_child = new_child.boxed();

        if mem::take(&mut self.is_inited) {
            self.child.borrow_mut().deinit(ctx);
            new_child.init(ctx);
        }
        mem::replace(&mut *self.child.borrow_mut(), new_child)
    }
}
#[impl_ui_node(
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
#[impl_ui_node(
    delegate = self.child.borrow(),
    delegate_mut = self.child.borrow_mut(),
)]
impl UiNode for AdoptiveChildNode {}
