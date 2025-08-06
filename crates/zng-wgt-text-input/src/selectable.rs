//! Selectable text.

use zng_ext_clipboard::COPY_CMD;
use zng_wgt::prelude::*;
use zng_wgt_button::Button;
use zng_wgt_input::focus::FocusableMix;
use zng_wgt_menu::{
    self as menu,
    context::{ContextMenu, context_menu_fn},
};
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};
use zng_wgt_text::{self as text, *};

#[doc(hidden)]
pub use zng_wgt::prelude::formatx as __formatx;

/// Styleable read-only text widget that can be selected and copied to clipboard.
///
/// # Shorthand
///
/// The same [`Text!`](struct@zng_wgt_text::Text#shorthand) shorthand can be used in this macro.
#[widget($crate::selectable::SelectableText {
    ($txt:literal) => {
        txt = $crate::selectable::__formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::selectable::__formatx!($txt, $($format)*);
    };
})]
pub struct SelectableText(FocusableMix<StyleMix<zng_wgt_text::Text>>);
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
        use zng_wgt_input::*;
        use zng_wgt_layer::*;

        widget_set! {
            self;
            replace = true;
            cursor = CursorIcon::Text;

            popup::context_capture = crate::default_popup_context_capture();
            context_menu_fn = WidgetFn::new(default_context_menu);
            selection_toolbar_fn = WidgetFn::new(default_selection_toolbar);
            selection_color = colors::ACCENT_COLOR_VAR.rgba_map(|c| c.with_alpha(30.pct()));
        }
    }
}

/// Context menu set by the [`DefaultStyle!`].
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_context_menu(args: menu::context::ContextMenuArgs) -> UiNode {
    let id = args.anchor_id;
    ContextMenu!(ui_vec![Button!(COPY_CMD.scoped(id)), Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),])
}

/// Selection toolbar set by the [`DefaultStyle!`].
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_selection_toolbar(args: text::SelectionToolbarArgs) -> UiNode {
    if args.is_touch {
        let id = args.anchor_id;
        ContextMenu! {
            style_fn = menu::context::TouchStyle!();
            children = ui_vec![Button!(COPY_CMD.scoped(id)), Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),]
        }
        .boxed()
    } else {
        NilUiNode.boxed()
    }
}
