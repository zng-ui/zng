use crate::core2::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

struct Margin<T: UiNode, M: Var<LayoutSideOffsets>> {
    child: T,
    margin: M,
    render_margin: LayoutSideOffsets,
    child_rect: LayoutRect,
}

#[impl_ui_node_crate(child)]
impl<T: UiNode, M: Var<LayoutSideOffsets>> UiNode for Margin<T, M> {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut child_sz = self.child.measure(available_size);
        child_sz.width += self.render_margin.left + self.render_margin.right;
        child_sz.height += self.render_margin.top + self.render_margin.bottom;
        child_sz
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width -= self.render_margin.left + self.render_margin.right;
        final_size.height -= self.render_margin.top + self.render_margin.bottom;
        self.child.arrange(final_size);

        self.child_rect = LayoutRect::new(
            LayoutPoint::new(self.render_margin.left, self.render_margin.top),
            final_size,
        );
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
        render_margin: LayoutSideOffsets::zero(),
        child_rect: LayoutRect::zero(),
    }
}
