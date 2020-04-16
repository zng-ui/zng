use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::{impl_ui_node, property};

struct Cursor<T: UiNode, C: LocalVar<CursorIcon>> {
    cursor: C,
    child: T,
}

#[impl_ui_node(child)]
impl<T: UiNode, C: LocalVar<CursorIcon>> UiNode for Cursor<T, C> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.cursor.init_local(ctx.vars);
        self.child.init(ctx);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.cursor.update_local(&ctx.vars).is_some() {
            ctx.updates.push_render();
        }
        self.child.update(ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_cursor(*self.cursor.get_local(), |frame| self.child.render(frame));
    }
}

/// Widget property that sets the [`CursorIcon`](zero_ui::core::types::CursorIcon) displayed when hovering the widget.
///
/// # Example
/// ```
/// # #[macro_use] extern crate zero_ui;
/// # fn main() {
/// use zero_ui::{
///     core::types::CursorIcon,
///     properties::cursor,
///     widgets::{container, text}
/// };
///
/// container! {
///     cursor: CursorIcon::Hand;
///     => text("Mouse over this text shows the hand cursor")
/// }
/// # ; }
/// ```
#[property(context)]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    Cursor {
        cursor: cursor.into_local(),
        child,
    }
}

pub trait TestGen<A>: Clone {
    type Assoc;
}

#[property(context)]
pub fn test_property(child: impl UiNode, cursor: impl TestGen<u32, Assoc = u8>) -> impl UiNode {
    Cursor {
        cursor: CursorIcon::Default.into_local(),
        child,
    }
}
