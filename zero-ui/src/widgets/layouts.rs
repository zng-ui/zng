//! Layout widgets.

mod align;
mod grid_;
mod stacks;
mod uniform_grid_;

pub use align::*;
//pub use grid::*;
pub use grid_::grid;
pub use stacks::*;
pub use uniform_grid_::*;

use crate::core::{impl_ui_node, UiNode, UiNodeList};

/// Creates a node that processes the `nodes` in the logical order they appear in the list, layouts for the largest node
/// and renders then on on top of the other from back to front.
///
/// This is the most simple *z-stack* implementation possible, it is a building block useful for quickly declaring
/// overlaying effects composed of multiple nodes, it does not do any alignment layout or z-sorting render,
/// for a complete z-stack panel widget see [`z_stack`].
///
/// [`z_stack`]: mod@z_stack
pub fn stack_nodes(nodes: impl UiNodeList) -> impl UiNode {
    struct NodesStackNode<C> {
        children: C,
    }
    #[impl_ui_node(children)]
    impl<C: UiNodeList> NodesStackNode<C> {}

    NodesStackNode { children: nodes }
}
