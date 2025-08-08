use std::sync::Arc;

use parking_lot::Mutex;

use crate::widget::WidgetUpdateMode;

use super::*;

/// Create a widget node that wraps the `widget` with any number of other non-widget nodes and
/// still delegates [`with_context`] to the `widget`.
///
/// Note that the [`with_context`] is called in the context of `widget`, not in the context of the `build_extension` nodes.
/// Other node operations are delegated to the `build_extension` nodes, and they in turn must delegate to the input child
/// node that is `widget`.
///
/// [`with_context`]: WidgetUiNode::with_context
pub fn extend_widget(widget: impl IntoUiNode, build_extension: impl FnOnce(UiNode) -> UiNode) -> UiNode {
    let widget = Arc::new(Mutex::new(widget.into_node()));
    let child = build_extension(UiNode::new(ExtendWidgetChildNode { widget: widget.clone() }));
    UiNode::new(ExtendWidgetNode { widget, child })
}

struct ExtendWidgetChildNode {
    widget: Arc<Mutex<UiNode>>,
}
impl UiNodeImpl for ExtendWidgetChildNode {
    fn children_len(&self) -> usize {
        1
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        if index == 0 {
            visitor(&mut *self.widget.lock())
        }
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        self.widget.lock().0.as_widget()?;
        Some(self)
    }
}
impl WidgetUiNodeImpl for ExtendWidgetChildNode {
    fn with_context(&mut self, update_mode: WidgetUpdateMode, visitor: &mut dyn FnMut()) {
        if let Some(wgt) = self.widget.lock().0.as_widget() {
            wgt.with_context(update_mode, visitor);
        } else {
            // this could be intentional, like nodes that only become widgets on init
            tracing::debug!("extend_widget child is not a widget");
        }
    }
}

struct ExtendWidgetNode {
    widget: Arc<Mutex<UiNode>>,
    child: UiNode,
}
impl UiNodeImpl for ExtendWidgetNode {
    fn children_len(&self) -> usize {
        1
    }

    fn with_child(&mut self, index: usize, visitor: &mut dyn FnMut(&mut UiNode)) {
        if index == 0 {
            visitor(&mut self.child)
        }
    }

    fn as_widget(&mut self) -> Option<&mut dyn WidgetUiNodeImpl> {
        self.widget.lock().0.as_widget()?;
        Some(self)
    }
}
impl WidgetUiNodeImpl for ExtendWidgetNode {
    fn with_context(&mut self, update_mode: WidgetUpdateMode, visitor: &mut dyn FnMut()) {
        if let Some(wgt) = self.widget.lock().0.as_widget() {
            wgt.with_context(update_mode, visitor);
        } else {
            // this could be intentional, like nodes that only become widgets on init
            tracing::debug!("extend_widget child is not a widget");
        }
    }
}
