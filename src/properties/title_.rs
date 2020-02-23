use crate::core::{
    context::WidgetContext,
    types::Text,
    var::{IntoVar, Var},
    UiNode,
};
use crate::{impl_ui_node, property};

struct Title<C: UiNode, T: Var<Text>> {
    child: C,
    title: T,
}

#[impl_ui_node(child)]
impl<C: UiNode, T: Var<Text>> UiNode for Title<C, T> {
    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        todo!("tooltip title not implemented {}", self.title.get(ctx.vars));
    }
}

/// Short informational text about the widget.
///
/// By default this property shows a tool-tip with the text, the [`window`](zero_ui::widgets::window) widget
/// captures this value and uses it for the window title, some other custom widgets may also override the default behavior.
#[property(context)]
pub fn title(child: impl UiNode, title: impl IntoVar<Text>) -> impl UiNode {
    Title {
        child,
        title: title.into_var(),
    }
}
