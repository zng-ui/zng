use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::{impl_ui_node, property};

struct MinSize<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    min_size: S,
    final_size: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<LayoutSize>> UiNode for MinSize<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.min_size.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.min_size.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(self.min_size.get_local().max(available_size))
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.final_size = self.min_size.get_local().max(final_size);
        self.child.arrange(self.final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_node(&self.child, &LayoutRect::from_size(self.final_size));
    }
}

#[property(outer)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MinSize {
        child,
        min_size: min_size.into_local(),
        final_size: LayoutSize::zero(),
    }
}

struct MaxSize<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    max_size: S,
    final_size: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<LayoutSize>> UiNode for MaxSize<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.max_size.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.max_size.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(self.max_size.get_local().min(available_size))
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.final_size = self.max_size.get_local().min(final_size);
        self.child.arrange(self.final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_node(&self.child, &LayoutRect::from_size(self.final_size));
    }
}

#[property(outer)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MaxSize {
        child,
        max_size: max_size.into_local(),
        final_size: LayoutSize::zero(),
    }
}

struct ExactSize<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    size: S,
    final_size: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<LayoutSize>> UiNode for ExactSize<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.size.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.size.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.child.measure(*self.size.get_local())
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        let size = *self.size.get_local();
        self.child.arrange(size);
        self.final_size = final_size.min(size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_node(&self.child, &LayoutRect::from_size(self.final_size));
    }
}

#[property(outer)]
pub fn size(child: impl UiNode, size: impl IntoVar<LayoutSize>) -> impl UiNode {
    ExactSize {
        child,
        size: size.into_local(),
        final_size: LayoutSize::zero(),
    }
}
