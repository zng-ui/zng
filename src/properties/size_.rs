use crate::core::{
    context::WidgetContext,
    is_layout_any_size,
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
        let min_size = replace_layout_any_size(*self.min_size.get_local(), available_size);
        self.child.measure(min_size.max(available_size))
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        let min_size = replace_layout_any_size(*self.min_size.get_local(), final_size);
        self.child.arrange(min_size.max(final_size));
    }
}

#[property(size)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MinSize {
        child,
        min_size: min_size.into_local(),
    }
}

struct MinWidth<T: UiNode, W: LocalVar<f32>> {
    child: T,
    min_width: W,
}

#[impl_ui_node(child)]
impl<T: UiNode, W: LocalVar<f32>> UiNode for MinWidth<T, W> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.min_width.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.min_width.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        let min_width = *self.min_width.get_local();
        if !is_layout_any_size(min_width) {
            available_size.width = min_width.max(available_size.width);
        }
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        let min_width = *self.min_width.get_local();
        if !is_layout_any_size(min_width) {
            final_size.width = min_width.max(final_size.width);
        }
        self.child.arrange(final_size);
    }
}

#[property(size)]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<f32>) -> impl UiNode {
    MinWidth {
        child,
        min_width: min_width.into_local(),
    }
}

struct MinHeight<T: UiNode, H: LocalVar<f32>> {
    child: T,
    min_height: H,
}

#[impl_ui_node(child)]
impl<T: UiNode, H: LocalVar<f32>> UiNode for MinHeight<T, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.min_height.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.min_height.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        let min_height = *self.min_height.get_local();
        if !is_layout_any_size(min_height) {
            available_size.height = min_height.max(available_size.height);
        }
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        let min_height = *self.min_height.get_local();
        if !is_layout_any_size(min_height) {
            final_size.height = min_height.max(final_size.height);
        }
        self.child.arrange(final_size);
    }
}

#[property(size)]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<f32>) -> impl UiNode {
    MinHeight {
        child,
        min_height: min_height.into_local(),
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
        self.child.arrange(self.max_size.get_local().min(final_size));
    }
}

#[property(size)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MaxSize {
        child,
        max_size: max_size.into_local(),
    }
}

struct MaxWidth<T: UiNode, W: LocalVar<f32>> {
    child: T,
    max_width: W,
}

#[impl_ui_node(child)]
impl<T: UiNode, W: LocalVar<f32>> UiNode for MaxWidth<T, W> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.max_width.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.max_width.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width = self.max_width.get_local().min(available_size.width);
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width = self.max_width.get_local().min(final_size.width);
        self.child.arrange(final_size);
    }
}

#[property(size)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<f32>) -> impl UiNode {
    MaxWidth {
        child,
        max_width: max_width.into_local(),
    }
}

struct MaxHeight<T: UiNode, H: LocalVar<f32>> {
    child: T,
    max_height: H,
}

#[impl_ui_node(child)]
impl<T: UiNode, H: LocalVar<f32>> UiNode for MaxHeight<T, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.max_height.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.max_height.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.height = self.max_height.get_local().min(available_size.height);
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.height = self.max_height.get_local().min(final_size.height);
        self.child.arrange(final_size);
    }
}

#[property(size)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<f32>) -> impl UiNode {
    MaxHeight {
        child,
        max_height: max_height.into_local(),
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
        let size = replace_layout_any_size(*self.size.get_local(), available_size);
        self.child.measure(size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        let size = replace_layout_any_size(*self.size.get_local(), final_size);
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
        let width = *self.width.get_local();
        if !is_layout_any_size(width) {
            available_size.width = width;
        }
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        let width = *self.width.get_local();
        if !is_layout_any_size(width) {
            final_size.width = width;
        }
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
        let height = *self.height.get_local();
        if !is_layout_any_size(height) {
            available_size.height = height;
        }
        self.child.measure(available_size)
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        let height = *self.height.get_local();
        if !is_layout_any_size(height) {
            final_size.height = height;
        }
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

fn replace_layout_any_size(mut size: LayoutSize, replacement_size: LayoutSize) -> LayoutSize {
    if is_layout_any_size(size.width) {
        size.width = replacement_size.width;
    }
    if is_layout_any_size(size.height) {
        size.height = replacement_size.height;
    }

    size
}
