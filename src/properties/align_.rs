use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::core::{impl_ui_node, property};

/// Normalized `x, y` alignment.
///
/// The numbers indicate how much to the right and bottom the content is moved within
/// a larger available space.
///
/// This is the value of the [`align`](align) property.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
        if self.alignment.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }

        self.child.update(ctx);
    }

    fn measure(&mut self, available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        self.child_rect.size = self.child.measure(available_size, pixels);
        self.child_rect.size
    }

    fn arrange(&mut self, final_size: LayoutSize, pixels: PixelGrid) {
        self.final_size = final_size;
        self.child_rect.size = final_size.min(self.child_rect.size);
        self.child.arrange(self.child_rect.size, pixels);

        let alignment = self.alignment.get_local();

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) * alignment.0,
            (final_size.height - self.child_rect.size.height) * alignment.1,
        )
        .snap_to(pixels);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
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
