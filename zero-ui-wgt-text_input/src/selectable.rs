//! Selectable text.

use zero_ui_ext_clipboard::COPY_CMD;
use zero_ui_wgt::prelude::*;
use zero_ui_wgt_button::Button;
use zero_ui_wgt_input::focus::FocusableMix;
use zero_ui_wgt_menu::{
    self as menu,
    context::{context_menu_fn, ContextMenu},
};
use zero_ui_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};
use zero_ui_wgt_text::{self as text, *};

/// Styleable read-only text widget that can be selected and copied to clipboard.
#[widget($crate::selectable::SelectableText)]
pub struct SelectableText(FocusableMix<StyleMix<zero_ui_wgt_text::Text>>);
impl SelectableText {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;
            txt_selectable = true;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(SelectableText);

/// Default selectable text style.
#[widget($crate::selectable::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        use zero_ui_wgt_input::*;
        use zero_ui_wgt_layer::*;

        widget_set! {
            self;
            replace = true;
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
    ContextMenu!(ui_vec![Button!(COPY_CMD.scoped(id)), Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),])
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
