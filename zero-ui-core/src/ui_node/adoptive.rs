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
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        let node = create(ad_child);
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
impl AdoptiveChildNode {
    fn nil() -> Self {
        Self {
            child: Rc::new(RefCell::new(NilUiNode.boxed())),
        }
    }
}
#[impl_ui_node(
    delegate = self.child.borrow(),
    delegate_mut = self.child.borrow_mut(),
)]
impl UiNode for AdoptiveChildNode {}

/// Represents an widget property use in dynamic initialization.
///
/// See the [`#[widget]`] documentation for more details.
///
/// [`#[widget]`]: macro@crate::widget
pub struct PropertyInstance {
    /// The property node, setup as an adoptive node that allows swapping the child node.
    pub node: AdoptiveNode<BoxedUiNode>,

    /// Name of the property that was set.
    ///
    /// All of these assigns have the same name `foo`:
    ///
    /// ```
    /// path::to::foo = true;
    /// bar as foo = true;
    /// foo = true;
    /// ```
    pub name: &'static str,

    /// Who assigned the property.
    pub source: PropertyInstanceSource,
    /*
    /// Unique ID of the property *type*.
    pub id: PropertyId,
    */
}
impl PropertyInstance {
    #[doc(hidden)]
    pub fn start_v1() -> (AdoptiveChildNode, PropertyInstanceBuilderV1) {
        let ad_child = AdoptiveChildNode::nil();
        let child = ad_child.child.clone();
        (ad_child, PropertyInstanceBuilderV1 { child })
    }
}

#[doc(hidden)]
pub struct PropertyInstanceBuilderV1 {
    child: Rc<RefCell<BoxedUiNode>>,
}
impl PropertyInstanceBuilderV1 {
    #[doc(hidden)]
    pub fn build(self, property: impl UiNode, name: &'static str, source: PropertyInstanceSourceV1) -> PropertyInstance {
        let node = AdoptiveNode {
            child: self.child,
            node: property.boxed(),
            is_inited: false,
        };

        PropertyInstance { node, name, source }
    }
}

#[doc(hidden)]
pub type PropertyInstanceSourceV1 = PropertyInstanceSource;

/// Represents who assigned the property that caused the [`PropertyInstance`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropertyInstanceSource {
    /// Property assigned in the widget declaration and not overwritten in the instance.
    ///
    /// ```
    /// # macro_rules! _demo { () => {
    /// #[widget($crate::foo)]
    /// pub mod foo {
    ///     properties! {
    ///         bar = true;
    ///     }
    /// }
    ///
    /// foo!() // `bar` assigned in the widget.
    /// # }}
    /// ```
    Widget,
    /// Property assigned in the widget instance.
    ///
    /// ```
    /// # foo! { ($($tt:tt)*) => { } }
    /// foo! {
    ///     bar = true;// assign in the instance.
    /// }
    /// ```
    Instance,
}

/*
unique_id_64! {
    /// Unique ID of a `#[property]` declaration.
    pub struct PropertyId;
}
*/
