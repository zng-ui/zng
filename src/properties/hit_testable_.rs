use crate::core::context::*;
use crate::core::render::FrameBuilder;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct HitTestable<U: UiNode, H: LocalVar<bool>> {
    child: U,
    hit_testable: H,
}
#[impl_ui_node(child)]
impl<U: UiNode, H: LocalVar<bool>> UiNode for HitTestable<U, H> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.hit_testable.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        if self.hit_testable.update_local(ctx.vars).is_some() {
            ctx.updates.push_render();
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_hit_testable(&self.child, *self.hit_testable.get_local());
    }
}

/// If the widget is visible during hit-testing.
///
/// When `false` the widget and is descendents do not
/// participate in pointer events and do not set the cursor.
///
/// Widgets inherit their hit-testability from their parents and by default
/// the window widget is hit-testable, so all widgets are hit-testable by default.
#[property(context)]
pub fn hit_testable(child: impl UiNode, hit_testable: impl IntoVar<bool>) -> impl UiNode {
    HitTestable {
        child,
        hit_testable: hit_testable.into_local(),
    }
}
