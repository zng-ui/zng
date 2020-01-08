use super::*;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

struct Widget<T: UiNode> {
    id: WidgetId,
    child: T,
}

#[impl_ui_node_crate]
impl<T: UiNode> UiNode for Widget<T> {
    fn update(&mut self, ctx: &mut AppContext) {
        ctx.widget_update(self.id, |ctx| self.child.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut AppContext) {
        ctx.widget_update(self.id, |ctx| self.child.update_hp(ctx));
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_widget(self.id, &self.child);
    }
}

/// Creates a widget bondary.
pub fn widget(id: WidgetId, child: impl UiNode) -> impl UiNode {
    Widget { id, child }
}

struct Cursor<T: UiNode, C: Var<CursorIcon>> {
    cursor: C,
    child: T,
    render_cursor: CursorIcon,
}

#[impl_ui_node_crate]
impl<T: UiNode, C: Var<CursorIcon>> UiNode for Cursor<T, C> {
    fn init(&mut self, ctx: &mut AppContext) {
        self.render_cursor = *self.cursor.get(&ctx);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(cursor) = self.cursor.update(&ctx) {
            self.render_cursor = *cursor;
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_cursor(self.render_cursor, &self.child);
    }
}

#[property(context_var)]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    Cursor {
        cursor: cursor.into_var(),
        child,
        render_cursor: CursorIcon::Default,
    }
}

// widget code expands to
pub const NOTE: &str = stringify! {
    let property1_arg0 = #expr;
    let property1_arg1 = #expr;
    let property2_arg = #expr;

    let (child, property1_arg, pa1) = property1::set_context_var(child, property1_arg0, property1_arg1);
    let (child, property2_arg) = property1::set_context_var(child, property2_arg);

    let (child, property1_arg) = property1::set_event(child, property1_arg);
    let (child, property2_arg) = property1::set_event(child, property2_arg);

    let (child, property1_arg) = property1::set_outer(child, property1_arg);
    let (child, property2_arg) = property1::set_outer(child, property2_arg);

    let child = crate::core2::widget_area(child);

    let (child, property1_arg) = property1::set_inner(child, property1_arg);
    let (child, property2_arg) = property1::set_inner(child, property2_arg);
};

use crate::widget;

widget! {
    //! Button widget.
    //! # Arguments
    //! * `on_click`: Required button click event handler.
    //! * `padding`: Margin around the button content.
    //! * `background_color`:
    //! * `border`:
    //!
    //! # Examples
    //! ```
    //! use crate::widgets::text;
    //!
    //! button! {
    //!     on_click: |_, _| { println!("Button clicked!") };
    //!     => text("Click Me!")
    //! }
    //! ```

    use crate::properties::*;

    // Properties applied to child before calling widget fn.
    child_properties {
        // Property declaration without default value, if not set does not apply.
        // If set applies margin to child.
        padding -> margin;
        // Same with default value.
        content_align -> align: CENTER;
        // Default value of background_color property that is applied to child.
        background_color: rgb(255, 255, 255);
    }


    // Properties applied to return of widget fn. Same sintax as
    // child_properties.
    self_properties {
        border: 4., (rgba(0, 0, 0, 0.0), BorderStyle::Dashed);
    }

    // widget signature, must name the parameters after child,
    // they behave like required properties in the declared button! macro.

    pub fn new_button(child: impl UiNode, on_click: u32) -> impl UiNode {
        println!("does things");
        child
    }
}
