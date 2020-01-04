use super::*;
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
}

#[impl_ui_node_crate]
impl<T: UiNode, C: Var<CursorIcon>> UiNode for Cursor<T, C> {
    fn render(&self, frame: &mut FrameBuilder) {
        //frame.push_cursor(self.cursor, &self.child);
    }
}

//#[property]
pub fn cursor(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
    Cursor {
        cursor: cursor.into_var(),
        child,
    }
}

// #[property(outer)] expands to:
pub mod my_layout_property {
    use super::*;

    pub fn set(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> impl UiNode {
        Cursor {
            cursor: cursor.into_var(),
            child,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn set_context_var(
        child: impl UiNode,
        cursor: impl IntoVar<CursorIcon>,
    ) -> (impl UiNode, impl IntoVar<CursorIcon>) {
        // not #[property(context_var)], pass through, return types copied from `set` argument types.
        (child, cursor)
    }

    #[doc(hidden)]
    #[inline]
    pub fn set_event(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> (impl UiNode, impl IntoVar<CursorIcon>) {
        // not #[property(event)], pass through
        (child, cursor)
    }

    #[doc(hidden)]
    #[inline]
    pub fn set_outer(child: impl UiNode, cursor: impl IntoVar<CursorIcon>) -> (impl UiNode, ()) {
        // is #[property(outer)], pass through, consume arguments, return type is (impl UiNone, <repeat () for `set`.args.count - 1>)
        (set(child, cursor), ())
    }

    #[doc(hidden)]
    #[inline]
    pub fn set_inner(child: impl UiNode, _: ()) -> (impl UiNode, ()) {
        // not #[property(inner)], after argument consumption,  pass through child, input and outputs are blanks
        (child, ())
    }
}

// widget code expands to
pub const NOTE: &str = stringify! {
    let property1_arg = #expr;
    let property2_arg = #expr;

    let (child, property1_arg) = property1::set_context_var(child, property1_arg);
    let (child, property2_arg) = property1::set_context_var(child, property2_arg);

    let (child, property1_arg) = property1::set_event(child, property1_arg);
    let (child, property2_arg) = property1::set_event(child, property2_arg);

    let (child, property1_arg) = property1::set_outer(child, property1_arg);
    let (child, property2_arg) = property1::set_outer(child, property2_arg);

    let child = crate::core2::widget_area(child);

    let (child, property1_arg) = property1::set_inner(child, property1_arg);
    let (child, property2_arg) = property1::set_inner(child, property2_arg);
};
