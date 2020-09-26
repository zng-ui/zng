use crate::core::{
    color::{self, Filter},
    context::WidgetContext,
    render::FrameBuilder,
    var::{IntoVar, LocalVar, Var},
};
use crate::core::{impl_ui_node, property, units::FactorNormal, UiNode};

struct FilterNode<C: UiNode, F: LocalVar<Filter>> {
    child: C,
    filter: F,
}
#[impl_ui_node(child)]
impl<C: UiNode, F: LocalVar<Filter>> UiNode for FilterNode<C, F> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.filter.init_local(ctx.vars);
        self.child.init(ctx)
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.filter.update_local(ctx.vars).is_some() {
            ctx.updates.push_render()
        }
        self.child.update(ctx)
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame
            .widget_filters()
            .expect("filter property is `context`, expected `widget_filters` access")
            .push_filter(self.filter.get_local().clone());
        self.child.render(frame)
    }
}

#[property(context)]
pub fn filter(child: impl UiNode, filter: impl IntoVar<Filter>) -> impl UiNode {
    FilterNode {
        child,
        filter: filter.into_local(),
    }
}

/// Inverts the colors of the widget.
///
/// This property is a shorthand way of setting [`filter`] to [`color::invert(amount)`](color::invert) using variable merging.
#[property(context)]
pub fn invert_color(child: impl UiNode, amount: impl IntoVar<FactorNormal>) -> impl UiNode {
    filter::set(child, amount.into_var().map(|&a| color::invert(a)))
}
