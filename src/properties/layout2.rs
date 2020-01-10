use crate::core2::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

struct MinSize<T: UiNode, S: Var<LayoutSize>> {
    child: T,
    min_size: S,
    curr_min_size: LayoutSize,
    final_size: LayoutSize,
}

#[impl_ui_node_crate(child)]
impl<T: UiNode, S: Var<LayoutSize>> UiNode for MinSize<T, S> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.curr_min_size = *self.min_size.get(ctx);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(min_size) = self.min_size.update(ctx) {
            self.curr_min_size = *min_size;
            ctx.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(self.curr_min_size.max(available_size))
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.final_size = self.curr_min_size.max(final_size);
        self.child.arrange(self.final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_ui_node(&self.child, &LayoutRect::from_size(self.final_size));
    }
}

#[property(outer)]
pub fn min_size(child: impl UiNode, min_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MinSize {
        child,
        min_size: min_size.into_var(),
        curr_min_size: LayoutSize::zero(),
        final_size: LayoutSize::zero(),
    }
}

struct MaxSize<T: UiNode, S: Var<LayoutSize>> {
    child: T,
    max_size: S,
    curr_max_size: LayoutSize,
    final_size: LayoutSize,
}

#[impl_ui_node_crate(child)]
impl<T: UiNode, S: Var<LayoutSize>> UiNode for MaxSize<T, S> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.curr_max_size = *self.max_size.get(ctx);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(max_size) = self.max_size.update(ctx) {
            self.curr_max_size = *max_size;
            ctx.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(self.curr_max_size.min(available_size))
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.final_size = self.curr_max_size.min(final_size);
        self.child.arrange(self.final_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_ui_node(&self.child, &LayoutRect::from_size(self.final_size));
    }
}

#[property(outer)]
pub fn max_size(child: impl UiNode, max_size: impl IntoVar<LayoutSize>) -> impl UiNode {
    MaxSize {
        child,
        max_size: max_size.into_var(),
        curr_max_size: LayoutSize::zero(),
        final_size: LayoutSize::zero(),
    }
}

struct ExactSize<T: UiNode, S: Var<LayoutSize>> {
    child: T,
    size: S,
    curr_size: LayoutSize,
    final_size: LayoutSize,
}

#[impl_ui_node_crate(child)]
impl<T: UiNode, S: Var<LayoutSize>> UiNode for ExactSize<T, S> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.curr_size = *self.size.get(ctx);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(size) = self.size.update(ctx) {
            self.curr_size = *size;
            ctx.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.child.measure(self.curr_size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(self.curr_size);
        self.final_size = final_size.min(self.curr_size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_ui_node(&self.child, &LayoutRect::from_size(self.final_size));
    }
}

#[property(outer)]
pub fn size(child: impl UiNode, size: impl IntoVar<LayoutSize>) -> impl UiNode {
    ExactSize {
        child,
        size: size.into_var(),
        curr_size: LayoutSize::zero(),
        final_size: LayoutSize::zero(),
    }
}

#[property(outer)]
pub fn width(child: impl UiNode, width: impl IntoVar<LayoutSize>) -> impl UiNode {
    let width = width.into_var();
    //size::set(child, width.map(|w|LayoutSize::new(w, std::f32::INFINITY)))
    todo!();
    child
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Alignment(pub f32, pub f32);

impl Alignment {
    pub const TOP_LEFT: Alignment = Alignment(0.0, 0.0);
    pub const TOP_CENTER: Alignment = Alignment(0.0, 0.5);
    pub const TOP_RIGHT: Alignment = Alignment(0.0, 1.0);

    pub const CENTER_LEFT: Alignment = Alignment(0.0, 0.5);
    pub const CENTER: Alignment = Alignment(0.5, 0.5);
    pub const CENTER_RIGHT: Alignment = Alignment(1.0, 0.5);

    pub const BOTTOM_LEFT: Alignment = Alignment(0.0, 1.0);
    pub const BOTTOM_CENTER: Alignment = Alignment(0.5, 1.0);
    pub const BOTTOM_RIGHT: Alignment = Alignment(1.0, 1.0);
}

struct Align<T: UiNode, A: Var<Alignment>> {
    child: T,
    alignment: A,

    curr_alignment: Alignment,
    final_size: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node_crate(child)]
impl<T: UiNode, A: Var<Alignment>> UiNode for Align<T, A> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.curr_alignment = *self.alignment.get(ctx);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(alignment) = self.alignment.update(ctx) {
            self.curr_alignment = *alignment;

            self.child_rect.origin = LayoutPoint::new(
                (self.final_size.width - self.child_rect.size.width) * self.curr_alignment.0,
                (self.final_size.height - self.child_rect.size.height) * self.curr_alignment.1,
            );

            ctx.push_frame();
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

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) * self.curr_alignment.0,
            (final_size.height - self.child_rect.size.height) * self.curr_alignment.1,
        );
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_ui_node(&self.child, &self.child_rect);
    }
}

#[property(outer)]
pub fn align(child: impl UiNode, alignment: impl IntoVar<Alignment>) -> impl UiNode {
    Align {
        child,
        alignment: alignment.into_var(),
        curr_alignment: Alignment::TOP_LEFT,
        final_size: LayoutSize::zero(),
        child_rect: LayoutRect::zero(),
    }
}

fn center(child: impl UiNode) -> impl UiNode {
    align::set(child, Alignment::CENTER)
}

struct Margin<T: UiNode, M: Var<LayoutSideOffsets>> {
    child: T,
    margin: M,
    size_increment: LayoutSize,
    child_rect: LayoutRect,
}

#[impl_ui_node_crate(child)]
impl<T: UiNode, M: Var<LayoutSideOffsets>> UiNode for Margin<T, M> {
    fn init(&mut self, ctx: &mut AppContext) {
        let margin = self.margin.get(ctx);
        self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
        self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);

        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(margin) = self.margin.update(ctx) {
            self.child_rect.origin = LayoutPoint::new(margin.left, margin.top);
            self.size_increment = LayoutSize::new(margin.left + margin.right, margin.top + margin.bottom);
            ctx.push_layout();
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
        frame.push_ui_node(&self.child, &self.child_rect);
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
