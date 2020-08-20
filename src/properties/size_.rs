use crate::core::{
    context::WidgetContext,
    is_layout_any_size,
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::core::{impl_ui_node, property};

struct MinSizeNode<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    min_size: S,
}
#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<LayoutSize>> UiNode for MinSizeNode<T, S> {
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

    fn measure(&mut self, available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let min_size = replace_layout_any_size(*self.min_size.get_local(), available_size).snap_to(pixels);
        self.child.measure(min_size.max(available_size), pixels)
    }

    fn arrange(&mut self, final_size: LayoutSize, pixels: PixelGrid) {
        let min_size = replace_layout_any_size(*self.min_size.get_local(), final_size).snap_to(pixels);
        self.child.arrange(min_size.max(final_size), pixels);
    }
}

#[property(size)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MinSizeNode {
        child,
        min_size: min_size.into_local(),
    }
}

struct MinWidthNode<T: UiNode, W: LocalVar<f32>> {
    child: T,
    min_width: W,
}
#[impl_ui_node(child)]
impl<T: UiNode, W: LocalVar<f32>> UiNode for MinWidthNode<T, W> {
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

    fn measure(&mut self, mut available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let min_width = *self.min_width.get_local();
        if !is_layout_any_size(min_width) {
            available_size.width = pixels.snap(min_width.max(available_size.width));
        }
        self.child.measure(available_size, pixels)
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        let min_width = *self.min_width.get_local();
        if !is_layout_any_size(min_width) {
            final_size.width = pixels.snap(min_width.max(final_size.width));
        }
        self.child.arrange(final_size, pixels);
    }
}

#[property(size)]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<f32>) -> impl UiNode {
    MinWidthNode {
        child,
        min_width: min_width.into_local(),
    }
}

struct MinHeightNode<T: UiNode, H: LocalVar<f32>> {
    child: T,
    min_height: H,
}
#[impl_ui_node(child)]
impl<T: UiNode, H: LocalVar<f32>> UiNode for MinHeightNode<T, H> {
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

    fn measure(&mut self, mut available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let min_height = *self.min_height.get_local();
        if !is_layout_any_size(min_height) {
            available_size.height = pixels.snap(min_height.max(available_size.height));
        }
        self.child.measure(available_size, pixels)
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        let min_height = *self.min_height.get_local();
        if !is_layout_any_size(min_height) {
            final_size.height = pixels.snap(min_height.max(final_size.height));
        }
        self.child.arrange(final_size, pixels);
    }
}

#[property(size)]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<f32>) -> impl UiNode {
    MinHeightNode {
        child,
        min_height: min_height.into_local(),
    }
}

struct MaxSizeNode<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    max_size: S,
}
#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<LayoutSize>> UiNode for MaxSizeNode<T, S> {
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

    fn measure(&mut self, available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        self.child
            .measure(self.max_size.get_local().snap_to(pixels).min(available_size), pixels)
    }

    fn arrange(&mut self, final_size: LayoutSize, pixels: PixelGrid) {
        self.child
            .arrange(self.max_size.get_local().snap_to(pixels).min(final_size), pixels);
    }
}

#[property(size)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MaxSizeNode {
        child,
        max_size: max_size.into_local(),
    }
}

struct MaxWidthNode<T: UiNode, W: LocalVar<f32>> {
    child: T,
    max_width: W,
}
#[impl_ui_node(child)]
impl<T: UiNode, W: LocalVar<f32>> UiNode for MaxWidthNode<T, W> {
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

    fn measure(&mut self, mut available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        available_size.width = pixels.snap(self.max_width.get_local().min(available_size.width));
        self.child.measure(available_size, pixels)
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        final_size.width = pixels.snap(self.max_width.get_local().min(final_size.width));
        self.child.arrange(final_size, pixels);
    }
}

#[property(size)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<f32>) -> impl UiNode {
    MaxWidthNode {
        child,
        max_width: max_width.into_local(),
    }
}

struct MaxHeightNode<T: UiNode, H: LocalVar<f32>> {
    child: T,
    max_height: H,
}
#[impl_ui_node(child)]
impl<T: UiNode, H: LocalVar<f32>> UiNode for MaxHeightNode<T, H> {
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

    fn measure(&mut self, mut available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        available_size.height = pixels.snap(self.max_height.get_local().min(available_size.height));
        self.child.measure(available_size, pixels)
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        final_size.height = pixels.snap(self.max_height.get_local().min(final_size.height));
        self.child.arrange(final_size, pixels);
    }
}

#[property(size)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<f32>) -> impl UiNode {
    MaxHeightNode {
        child,
        max_height: max_height.into_local(),
    }
}

struct SizeNode<T: UiNode, S: LocalVar<LayoutSize>> {
    child: T,
    size: S,
}
#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<LayoutSize>> UiNode for SizeNode<T, S> {
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

    fn measure(&mut self, available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let size = replace_layout_any_size(*self.size.get_local(), available_size).snap_to(pixels);
        self.child.measure(size, pixels)
    }

    fn arrange(&mut self, final_size: LayoutSize, pixels: PixelGrid) {
        let size = replace_layout_any_size(*self.size.get_local(), final_size).snap_to(pixels);
        self.child.arrange(size, pixels);
    }
}

/// Size of the widget.
///
/// When set the widget is sized with the given value, independent of the parent available size.
///
/// If the width or height is set to [positive infinity](is_layout_any_size) then the normal layout measuring happens.
///
/// # Example
/// ```
/// use zero_ui::prelude::*;
/// container! {
///     background_color: rgb(255, 0, 0);
///     size: (200.0, 300.0);
///     content: text("200x300 red");
/// }
/// # ;
/// ```
#[property(size)]
pub fn size(child: impl UiNode, size: impl IntoVar<LayoutSize>) -> impl UiNode {
    SizeNode {
        child,
        size: size.into_local(),
    }
}

struct WidthNode<T: UiNode, W: LocalVar<f32>> {
    child: T,
    width: W,
}
#[impl_ui_node(child)]
impl<T: UiNode, W: LocalVar<f32>> UiNode for WidthNode<T, W> {
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

    fn measure(&mut self, mut available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let width = *self.width.get_local();
        if !is_layout_any_size(width) {
            available_size.width = pixels.snap(width);
        }
        self.child.measure(available_size, pixels)
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        let width = *self.width.get_local();
        if !is_layout_any_size(width) {
            final_size.width = pixels.snap(width);
        }
        self.child.arrange(final_size, pixels)
    }
}

#[property(size)]
pub fn width(child: impl UiNode, width: impl IntoVar<f32>) -> impl UiNode {
    WidthNode {
        child,
        width: width.into_local(),
    }
}

struct HeightNode<T: UiNode, H: LocalVar<f32>> {
    child: T,
    height: H,
}
#[impl_ui_node(child)]
impl<T: UiNode, H: LocalVar<f32>> UiNode for HeightNode<T, H> {
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

    fn measure(&mut self, mut available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let height = *self.height.get_local();
        if !is_layout_any_size(height) {
            available_size.height = pixels.snap(height);
        }
        self.child.measure(available_size, pixels)
    }

    fn arrange(&mut self, mut final_size: LayoutSize, pixels: PixelGrid) {
        let height = *self.height.get_local();
        if !is_layout_any_size(height) {
            final_size.height = pixels.snap(height);
        }
        self.child.arrange(final_size, pixels)
    }
}

#[property(size)]
pub fn height(child: impl UiNode, height: impl IntoVar<f32>) -> impl UiNode {
    HeightNode {
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
