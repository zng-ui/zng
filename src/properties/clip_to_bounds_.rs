use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::FrameBuilder,
    units::*,
    var::{IntoVar, LocalVar},
    UiNode,
};
use crate::core::{impl_ui_node, property};

struct ClipToBoundsNode<T: UiNode, S: LocalVar<bool>> {
    child: T,
    clip: S,
    bounds: LayoutSize,
}

#[impl_ui_node(child)]
impl<T: UiNode, S: LocalVar<bool>> UiNode for ClipToBoundsNode<T, S> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.clip.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.clip.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }

        self.child.update(ctx);
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.bounds = final_size;
        self.child.arrange(final_size, ctx)
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if *self.clip.get_local() {
            frame.push_simple_clip(self.bounds, |frame| self.child.render(frame));
        } else {
            self.child.render(frame);
        }
    }
}

/// Clips the widget child to the area of the widget when set to `true`.
///
/// The clip is a simple rectangular area that matches the widget size. Any content rendered
/// outsize the widget size bounds is clipped.
///
/// # Example
/// ```
/// use zero_ui::prelude::*;
///
/// container! {
///     background_color: rgb(255, 0, 0);
///     size: (200.0, 300.0);
///     clip_to_bounds: true;
///     content: container! {
///         background_color: rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size: (1000.0, 1000.0);
///         content: text("1000x1000 green clipped to 200x300");
///     };
/// }
/// # ;
/// ```
#[property(inner)]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    ClipToBoundsNode {
        child,
        clip: clip.into_local(),
        bounds: LayoutSize::zero(),
    }
}
