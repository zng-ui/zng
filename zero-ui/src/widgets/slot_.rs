use crate::prelude::new_widget::*;
use crate::core::{RcNode, RcNodeTakeSignal};

/// An [`RcNode`] slot widget.
///
/// ## `slot()`
///
/// If you only want to create a slot as an widget there is a [`slot`](fn@slot) shortcut function.
#[widget($crate::widgets::slot)]
pub mod slot {

    use super::*;

    properties! {
        /// The [`RcNode`] reference.
        #[allowed_in_when = false]
        node(RcNode<impl UiNode>);

        /// A closure that returns `true` when this slot should **take** the `node`.
        ///
        /// This property accepts any `bool` variable, you can also use [`take_on_init`] to
        /// be the first slot to take the widget, [`take_on`] to take when an event listener updates or [`take_if`]
        /// to use a custom delegate to signal.
        ///
        /// See [`RcNode::slot`] for more details.
        #[allowed_in_when = false]
        take_signal(impl RcNodeTakeSignal);
    }

    fn new_child(node: RcNode<impl UiNode>, take_signal: impl RcNodeTakeSignal) -> impl UiNode {
        let node = node.slot(take_signal);
        implicit_base::nodes::leaf_transform(node)
    }
}

/// An [`RcNode`] slot widget.
///
/// # `slot!`
///
/// This function is just a shortcut for [`slot!`](mod@slot).
pub fn slot(node: RcNode<impl UiNode>, take_signal: impl RcNodeTakeSignal) -> impl Widget {
    slot!(node; take_signal)
}
