//! Selectable text.

use zero_ui_ext_clipboard::COPY_CMD;
use zero_ui_wgt::prelude::*;
use zero_ui_wgt_input::focus::FocusableMix;
use zero_ui_wgt_menu::{
    self as menu,
    context::{context_menu_fn, ContextMenu},
};
use zero_ui_wgt_style::{Style, StyleFn, StyleMix};
use zero_ui_wgt_text::{self as text, *};
use zero_ui_wgt_button::Button;

/// Styleable read-only text widget that can be selected and copied to clipboard.
#[widget($crate::selectable::SelectableText)]
pub struct SelectableText(FocusableMix<StyleMix<zero_ui_wgt_text::Text>>);
impl SelectableText {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            txt_selectable = true;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Selectable text style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Sets the selectable text style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the selectable text in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    zero_ui_wgt_style::with_style_extension(child, STYLE_VAR, style)
}

/// Default selectable text style.
#[widget($crate::selectable::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        use zero_ui_wgt_input::*;
        use zero_ui_wgt_layer::*;

        widget_set! {
            self;
            cursor = CursorIcon::Text;

            popup::context_capture = crate::default_popup_context_capture();
            context_menu_fn = WidgetFn::new(default_context_menu);
            selection_toolbar_fn = WidgetFn::new(default_selection_toolbar);
        }
    }
}

/// Context menu set by the [`DefaultStyle!`].
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_context_menu(args: menu::context::ContextMenuArgs) -> impl UiNode {
    let id = args.anchor_id;
    ContextMenu!(ui_vec![
        Button!(COPY_CMD.scoped(id)),
        Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),
    ])
}

/// Selection toolbar set by the [`DefaultStyle!`].
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_selection_toolbar(args: text::SelectionToolbarArgs) -> impl UiNode {
    if args.is_touch {
        let id = args.anchor_id;
        ContextMenu! {
            style_fn = menu::context::TouchStyle!();
            children = ui_vec![
                Button!(COPY_CMD.scoped(id)),
                Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),
            ]
        }
        .boxed()
    } else {
        NilUiNode.boxed()
    }
}
