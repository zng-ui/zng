//! Text widgets and properties.

use crate::prelude::new_widget::*;

pub mod nodes;
mod text_properties;
pub use text_properties::*;

/// A configured text run.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
///
/// let hello_txt = Text! {
///     font_family = "Arial";
///     font_size = 18;
///     txt = "Hello!";
/// };
/// ```
/// # Shorthand
///
/// The `Text!` macro provides shorthand syntax that matches the [`formatx!`] input, but outputs a text widget:
///
/// ```
/// # use zero_ui::prelude::*;
/// let txt = Text!("Hello!");
///
/// let name = "World";
/// let fmt = Text!("Hello {}!", name);
///
/// let expr = Text!({
///     let mut s = String::new();
///     s.push('a');
///     s
/// });
/// ```
///
/// The code abode is equivalent to:
///
/// ```
/// # use zero_ui::prelude::*;
/// let txt = Text! {
///     txt = zero_ui::core::text::formatx!("Hello!");
/// };
///
/// let name = "World";
/// let fmt = Text! {
///     txt = zero_ui::core::text::formatx!("Hello {}!", name);
/// };
///
/// let expr = Text! {
///     txt = {
///         let mut s = String::new();
///         s.push('a');
///         s
///     };
/// };
/// ```
///
/// [`formatx!`]: crate::core::text::formatx!
#[widget($crate::widgets::Text {
    ($txt:literal) => {
        txt = $crate::core::text::formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::core::text::formatx!($txt, $($format)*);
    };
})]
pub struct Text(WidgetBase);

impl Text {
    fn on_start(&mut self) {
        self.builder().push_build_action(|wgt| {
            let child = nodes::render_text();
            let child = nodes::render_caret(child);
            let child = nodes::render_overlines(child);
            let child = nodes::render_strikethroughs(child);
            let child = nodes::render_underlines(child);
            wgt.set_child(child.boxed());

            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", nodes::layout_text);

            let text = wgt.capture_var_or_default(property_id!(Self::txt));
            wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", |child| nodes::resolve_text(child, text));
        });
    }

    widget_impl! {
        /// Spacing in-between the text and borders.
        pub fn crate::properties::padding(padding: impl IntoVar<SideOffsets>);
    }
}

/// The text string.
///
/// Set to an empty string (`""`) by default.
#[property(CHILD, capture, default(""), impl(Text))]
pub fn txt(child: impl UiNode, txt: impl IntoVar<Txt>) -> impl UiNode {}

///<span data-del-macro-root></span> A simple text run with **bold** font weight.
///
/// The input syntax is the same as the shorthand [`Text!`].
///
/// # Configure
///
/// Apart from the font weight this widget can be configured with contextual properties like [`Text!`].
///
/// [`Text!`]: struct@Text
#[macro_export]
macro_rules! Strong {
    ($txt:expr) => {
        $crate::widgets::Text! {
            txt = $txt;
            font_weight = $crate::core::text::FontWeight::BOLD;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::widgets::Text! {
            txt = $crate::core::text::formatx!($txt, $($format)*);
            font_weight = $crate::core::text::FontWeight::BOLD;
        }
    };
}
#[doc(inline)]
pub use Strong;

///<span data-del-macro-root></span> A simple text run with *italic* font style.
///
/// The input syntax is the same as the shorthand [`Text!`].
///
/// # Configure
///
/// Apart from the font style this widget can be configured with contextual properties like [`Text!`].
///
/// [`Text!`]: struct@Text
#[macro_export]
macro_rules! Em {
    ($txt:expr) => {
        $crate::widgets::Text! {
            txt = $txt;
            font_style = FontStyle::Italic;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::widgets::Text! {
            txt = $crate::core::text::formatx!($txt, $($format)*);
            font_style = FontStyle::Italic;
        }
    };
}
#[doc(inline)]
pub use Em;
