use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::*,
    var::{IntoVar, LocalVar, Var},
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

/// Nomalized `x, y` alignment.
///
/// The numbers indicate how much to the right and bottom the content is moved within
/// a larger available space.
///
/// This is the value of the [`align`](align) property.
#[derive(Debug, Clone, Copy, Default)]
pub struct Alignment(pub f32, pub f32);

macro_rules! named_aligns {
    ( $($NAME:ident = ($x:expr, $y:expr);)+ ) => {named_aligns!{$(
        [stringify!(($x, $y))] $NAME = ($x, $y);
    )+}};

    ( $([$doc:expr] $NAME:ident = ($x:expr, $y:expr);)+ ) => {$(
        #[doc=$doc]
        pub const $NAME: Alignment = Alignment($x, $y);

    )+};
}

impl Alignment {
    named_aligns! {
        TOP_LEFT = (0.0, 0.0);
        TOP_CENTER = (0.0, 0.5);
        TOP_RIGHT = (0.0, 1.0);

        CENTER_LEFT = (0.0, 0.5);
        CENTER = (0.5, 0.5);
        CENTER_RIGHT = (1.0, 0.5);

        BOTTOM_LEFT = (0.0, 1.0);
        BOTTOM_CENTER = (0.5, 1.0);
        BOTTOM_RIGHT = (1.0, 1.0);
    }
}

struct Align<T: UiNode, A: LocalVar<Alignment>> {
    child: T,
    alignment: A,

    final_size: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node(child)]
impl<T: UiNode, A: LocalVar<Alignment>> UiNode for Align<T, A> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.alignment.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(alignment) = self.alignment.update_local(ctx.vars) {
            self.child_rect.origin = LayoutPoint::new(
                (self.final_size.width - self.child_rect.size.width) * alignment.0,
                (self.final_size.height - self.child_rect.size.height) * alignment.1,
            );

            ctx.updates.push_render();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        self.child_rect.size = self.child.measure(available_size);

        if available_size.width.is_infinite() {
            available_size.width = self.child_rect.size.width;
        }

        if available_size.height.is_infinite() {
            available_size.height = self.child_rect.size.height;
        }

        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.final_size = final_size;
        self.child_rect.size = final_size.min(self.child_rect.size);
        self.child.arrange(self.child_rect.size);

        let alignment = self.alignment.get_local();

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) * alignment.0,
            (final_size.height - self.child_rect.size.height) * alignment.1,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_node(&self.child, &self.child_rect);
    }
}

/// Aligns the widget within the available space.
///
/// The property argument is an [`Alignment`](Alignment) value.
#[property(outer)]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Alignment>) -> impl UiNode {
    Align {
        child,
        alignment: alignment.into_local(),
        final_size: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}

struct Margin<T: UiNode, M: Var<LayoutSideOffsets>> {
    child: T,
    margin: M,
    size_increment: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node(child)]
impl<T: UiNode, M: Var<LayoutSideOffsets>> UiNode for Margin<T, M> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let margin = self.margin.get(ctx.vars);
        self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
        self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);

        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(margin) = self.margin.update(ctx.vars) {
            self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
            self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(available_size - self.size_increment) + self.size_increment
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size = final_size - self.size_increment;
        self.child_rect.size = final_size;
        self.child.arrange(final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_node(&self.child, &self.child_rect);
    }
}

#[property(outer)]
pub fn margin(child: impl UiNode, margin: impl IntoVar<LayoutSideOffsets>) -> impl UiNode {
    Margin {
        child,
        margin: margin.into_var(),
        size_increment: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}
