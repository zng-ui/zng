use std::sync::Arc;

use parking_lot::Mutex;
use zero_ui_app_proc_macros::ui_node;

use crate::widget::WidgetUpdateMode;

use super::*;

/// Create an widget node that wraps the `widget` with any number of other non-widget nodes and
/// still delegates [`with_context`] to the `widget`.
///
/// Note that the [`with_context`] is not called in the context of `widget`, but not in the context of `build_extension` nodes.
/// Other node operation methods are delegated to the `build_extension` nodes, and they in turn must delegate to the input child
/// node that is also the `widget`.
///
/// [`with_context`]: UiNode::with_context
pub fn extend_widget(widget: impl UiNode, build_extension: impl FnOnce(BoxedUiNode) -> BoxedUiNode) -> impl UiNode {
    let widget = Arc::new(Mutex::new(widget.boxed()));
    let child = build_extension(ExtendWidgetChildNode { widget: widget.clone() }.boxed());
    ExtendWidgetNode { widget, child }
}

struct ExtendWidgetChildNode {
    widget: Arc<Mutex<BoxedUiNode>>,
}
#[ui_node(delegate = self.widget.lock())]
impl UiNode for ExtendWidgetChildNode {
    fn is_widget(&self) -> bool {
        self.widget.lock().is_widget()
    }

    fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        self.widget.lock().with_context(update_mode, f)
    }
}

struct ExtendWidgetNode {
    widget: Arc<Mutex<BoxedUiNode>>,
    child: BoxedUiNode,
}
#[ui_node(child)]
impl UiNode for ExtendWidgetNode {
    fn is_widget(&self) -> bool {
        self.widget.lock().is_widget()
    }

    fn with_context<R, F>(&mut self, update_mode: WidgetUpdateMode, f: F) -> Option<R>
    where
        F: FnOnce() -> R,
    {
        self.widget.lock().with_context(update_mode, f)
    }
}
