//! Manually sizing properties, [`min_size`](fn@min_size), [`max_size`](fn@max_size) and more.

use crate::prelude::new_property::*;

struct MinSizeNode<T: UiNode, S: VarLocal<Size>> {
    child: T,
    min_size: S,
}
#[impl_ui_node(child)]
impl<T: UiNode, S: VarLocal<Size>> UiNode for MinSizeNode<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.min_size.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.min_size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let min_size = self.min_size.get_local().to_layout(available_size, ctx);
        let desired_size = self
            .child
            .measure(replace_layout_any_size(min_size, available_size).max(available_size), ctx);
        desired_size.max(replace_layout_any_size(min_size, desired_size))
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        let min_size = replace_layout_any_size(self.min_size.get_local().to_layout(final_size, ctx), final_size);
        self.child.arrange(min_size.max(final_size), ctx);
    }
}

/// Minimum size of the widget.
///
/// The widget size can be larger then this but not smaller.
#[property(size)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<Size>) -> impl UiNode {
    MinSizeNode {
        child,
        min_size: min_size.into_local(),
    }
}

struct MinWidthNode<T: UiNode, W: VarLocal<Length>> {
    child: T,
    min_width: W,
}
#[impl_ui_node(child)]
impl<T: UiNode, W: VarLocal<Length>> UiNode for MinWidthNode<T, W> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.min_width.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.min_width.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let min_width = self
            .min_width
            .get_local()
            .to_layout(LayoutLength::new(available_size.width), ctx)
            .get();

        if !is_layout_any_size(min_width) {
            available_size.width = min_width.max(available_size.width);
            let mut desired_size = self.child.measure(available_size, ctx);
            desired_size.width = desired_size.width.max(min_width);
            desired_size
        } else {
            self.child.measure(available_size, ctx)
        }
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        let min_width = self.min_width.get_local().to_layout(LayoutLength::new(final_size.width), ctx).get();
        if !is_layout_any_size(min_width) {
            final_size.width = min_width.max(final_size.width);
        }
        self.child.arrange(final_size, ctx);
    }
}

/// Minimum width of the widget.
///
/// The widget width can be larger then this but not smaller.
#[property(size)]
pub fn min_width(child: impl UiNode, min_width: impl IntoVar<Length>) -> impl UiNode {
    MinWidthNode {
        child,
        min_width: min_width.into_local(),
    }
}

struct MinHeightNode<T: UiNode, H: VarLocal<Length>> {
    child: T,
    min_height: H,
}
#[impl_ui_node(child)]
impl<T: UiNode, H: VarLocal<Length>> UiNode for MinHeightNode<T, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.min_height.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.min_height.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let min_height = self
            .min_height
            .get_local()
            .to_layout(LayoutLength::new(available_size.height), ctx)
            .get();
        if !is_layout_any_size(min_height) {
            available_size.height = min_height.max(available_size.height);
            let mut desired_size = self.child.measure(available_size, ctx);
            desired_size.height = desired_size.height.max(min_height);
            desired_size
        } else {
            self.child.measure(available_size, ctx)
        }
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        let min_height = self
            .min_height
            .get_local()
            .to_layout(LayoutLength::new(final_size.height), ctx)
            .get();
        if !is_layout_any_size(min_height) {
            final_size.height = min_height.max(final_size.height);
        }
        self.child.arrange(final_size, ctx);
    }
}

/// Minimum height of the widget.
///
/// The widget height can be larger then this but not smaller.
#[property(size)]
pub fn min_height(child: impl UiNode, min_height: impl IntoVar<Length>) -> impl UiNode {
    MinHeightNode {
        child,
        min_height: min_height.into_local(),
    }
}

struct MaxSizeNode<T: UiNode, S: VarLocal<Size>> {
    child: T,
    max_size: S,
}
#[impl_ui_node(child)]
impl<T: UiNode, S: VarLocal<Size>> UiNode for MaxSizeNode<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.max_size.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.max_size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let max_size = self.max_size.get_local().to_layout(available_size, ctx);
        self.child.measure(max_size.min(available_size), ctx).min(max_size)
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.child
            .arrange(self.max_size.get_local().to_layout(final_size, ctx).min(final_size), ctx);
    }
}

/// Maximum size of the widget.
///
/// The widget size can be smaller then this but not larger.
#[property(size)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<Size>) -> impl UiNode {
    MaxSizeNode {
        child,
        max_size: max_size.into_local(),
    }
}

struct MaxWidthNode<T: UiNode, W: VarLocal<Length>> {
    child: T,
    max_width: W,
}
#[impl_ui_node(child)]
impl<T: UiNode, W: VarLocal<Length>> UiNode for MaxWidthNode<T, W> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.max_width.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.max_width.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let max_width = self
            .max_width
            .get_local()
            .to_layout(LayoutLength::new(available_size.width), ctx)
            .get();

        // if max_width is LAYOUT_ANY_SIZE this still works because every other value
        // is smaller the positive infinity.
        available_size.width = available_size.width.min(max_width);

        let mut desired_size = self.child.measure(available_size, ctx);
        desired_size.width = desired_size.width.min(max_width);
        desired_size
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        final_size.width = self
            .max_width
            .get_local()
            .to_layout(LayoutLength::new(final_size.width), ctx)
            .get()
            .min(final_size.width);
        self.child.arrange(final_size, ctx);
    }
}

/// Maximum width of the widget.
///
/// The widget width can be smaller then this but not larger.
#[property(size)]
pub fn max_width(child: impl UiNode, max_width: impl IntoVar<Length>) -> impl UiNode {
    MaxWidthNode {
        child,
        max_width: max_width.into_local(),
    }
}

struct MaxHeightNode<T: UiNode, H: VarLocal<Length>> {
    child: T,
    max_height: H,
}
#[impl_ui_node(child)]
impl<T: UiNode, H: VarLocal<Length>> UiNode for MaxHeightNode<T, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.max_height.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.max_height.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let max_height = self
            .max_height
            .get_local()
            .to_layout(LayoutLength::new(available_size.height), ctx)
            .get();

        // if max_height is LAYOUT_ANY_SIZE this still works because every other value
        // is smaller the positive infinity.
        available_size.height = available_size.height.min(max_height);

        let mut desired_size = self.child.measure(available_size, ctx);
        desired_size.height = desired_size.height.min(max_height);
        desired_size
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        final_size.height = self
            .max_height
            .get_local()
            .to_layout(LayoutLength::new(final_size.height), ctx)
            .get()
            .min(final_size.height);
        self.child.arrange(final_size, ctx);
    }
}

/// Maximum height of the widget.
///
/// The widget height can be smaller then this but not larger.
#[property(size)]
pub fn max_height(child: impl UiNode, max_height: impl IntoVar<Length>) -> impl UiNode {
    MaxHeightNode {
        child,
        max_height: max_height.into_local(),
    }
}

struct SizeNode<T: UiNode, S: VarLocal<Size>> {
    child: T,
    size: S,
}
#[impl_ui_node(child)]
impl<T: UiNode, S: VarLocal<Size>> UiNode for SizeNode<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.size.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.size.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let size = self.size.get_local().to_layout(available_size, ctx);
        let desired_size = self.child.measure(replace_layout_any_size(size, available_size), ctx);
        replace_layout_any_size(size, desired_size)
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        let size = replace_layout_any_size(self.size.get_local().to_layout(final_size, ctx), final_size);
        self.child.arrange(size, ctx);
    }
}

/// Manually sets the size of the widget.
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
pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
    SizeNode {
        child,
        size: size.into_local(),
    }
}

struct WidthNode<T: UiNode, W: VarLocal<Length>> {
    child: T,
    width: W,
}
#[impl_ui_node(child)]
impl<T: UiNode, W: VarLocal<Length>> UiNode for WidthNode<T, W> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.width.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.width.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let width = self.width.get_local().to_layout(LayoutLength::new(available_size.width), ctx).get();
        if !is_layout_any_size(width) {
            available_size.width = width;
            let mut desired_size = self.child.measure(available_size, ctx);
            desired_size.width = width;
            desired_size
        } else {
            self.child.measure(available_size, ctx)
        }
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        let width = self.width.get_local().to_layout(LayoutLength::new(final_size.width), ctx).get();
        if !is_layout_any_size(width) {
            final_size.width = width;
        }
        self.child.arrange(final_size, ctx)
    }
}

/// Exact width of the widget.
#[property(size)]
pub fn width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    WidthNode {
        child,
        width: width.into_local(),
    }
}

struct HeightNode<T: UiNode, H: VarLocal<Length>> {
    child: T,
    height: H,
}
#[impl_ui_node(child)]
impl<T: UiNode, H: VarLocal<Length>> UiNode for HeightNode<T, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.height.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.height.update_local(ctx.vars).is_some() {
            ctx.updates.layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        let height = self
            .height
            .get_local()
            .to_layout(LayoutLength::new(available_size.height), ctx)
            .get();
        if !is_layout_any_size(height) {
            available_size.height = height;
            let mut desired_size = self.child.measure(available_size, ctx);
            desired_size.height = height;
            desired_size
        } else {
            self.child.measure(available_size, ctx)
        }
    }

    fn arrange(&mut self, mut final_size: LayoutSize, ctx: &mut LayoutContext) {
        let height = self.height.get_local().to_layout(LayoutLength::new(final_size.height), ctx).get();
        if !is_layout_any_size(height) {
            final_size.height = height;
        }
        self.child.arrange(final_size, ctx)
    }
}

/// Exact height of the widget.
#[property(size)]
pub fn height(child: impl UiNode, height: impl IntoVar<Length>) -> impl UiNode {
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
