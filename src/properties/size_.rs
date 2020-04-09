use crate::core::{
    context::WidgetContext,
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::{impl_ui_node, property};

struct MinSize<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    min_size: S,
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
        let final_size = self.min_size.get_local().max(final_size);
        self.child.arrange(final_size);
    }
}

#[property(size)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MinSize {
        child,
        min_size: min_size.into_local(),
    }
}

struct MaxSize<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    max_size: S,
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
        let final_size = self.max_size.get_local().min(final_size);
        self.child.arrange(final_size);
    }
}

#[property(size)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MaxSize {
        child,
        max_size: max_size.into_local(),
    }
}

struct ExactSize<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    size: S,
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

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut size = *self.size.get_local();
        if size.width.is_infinite() {
            size.width = available_size.width;
        }
        if size.height.is_infinite() {
            size.height = available_size.height;
        }
        self.child.measure(size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        let mut size = *self.size.get_local();
        if size.width.is_infinite() {
            size.width = final_size.width;
        }
        if size.height.is_infinite() {
            size.height = final_size.height;
        }
        self.child.arrange(size);
    }
}

#[property(size)]
pub fn size(child: impl UiNode, size: impl IntoVar<LayoutSize>) -> impl UiNode {
    ExactSize {
        child,
        size: size.into_local(),
    }
}

struct ExactWidth<T: UiNode, W: LocalVar<f32>> {
    child: T,
    width: W,
}

#[impl_ui_node(child)]
impl<T: UiNode, W: LocalVar<f32>> UiNode for ExactWidth<T, W> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.width.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.width.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width = *self.width.get_local();
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width = *self.width.get_local();
        self.child.arrange(final_size)
    }
}

#[property(size)]
pub fn width(child: impl UiNode, width: impl IntoVar<f32>) -> impl UiNode {
    ExactWidth {
        child,
        width: width.into_local(),
    }
}

struct ExactHeight<T: UiNode, H: LocalVar<f32>> {
    child: T,
    height: H,
}

#[impl_ui_node(child)]
impl<T: UiNode, H: LocalVar<f32>> UiNode for ExactHeight<T, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.height.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.height.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.height = *self.height.get_local();
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.height = *self.height.get_local();
        self.child.arrange(final_size)
    }
}

#[property(size)]
pub fn height(child: impl UiNode, height: impl IntoVar<f32>) -> impl UiNode {
    ExactHeight {
        child,
        height: height.into_local(),
    }
}
