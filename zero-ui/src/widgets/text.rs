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
/// use zero_ui::prelude::text;
///
/// let hello_txt = text! {
///     font_family = "Arial";
///     font_size = 18;
///     txt = "Hello!";
/// };
/// ```
/// # Shorthand
///
/// The `text!` macro provides shorthand syntax that matches the [`formatx!`] input, but outputs a text widget:
///
/// ```
/// # use zero_ui::prelude::text;
/// let txt = text!("Hello!");
///
/// let name = "World";
/// let fmt = text!("Hello {}!", name);
///
/// let expr = text!({
///     let mut s = String::new();
///     s.push('a');
///     s
/// });
/// ```
///
/// The code abode is equivalent to:
///
/// ```
/// # use zero_ui::prelude::text;
/// let txt = text! {
///     txt = zero_ui::core::text::formatx!("Hello!");
/// };
///
/// let name = "World";
/// let fmt = text! {
///     txt = zero_ui::core::text::formatx!("Hello {}!", name);
/// };
///
/// let expr = text! {
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
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.builder().push_build_action(|wgt| {
            let child = nodes::render_text();
            let child = nodes::render_caret(child);
            let child = nodes::render_overlines(child);
            let child = nodes::render_strikethroughs(child);
            let child = nodes::render_underlines(child);
            wgt.set_child(child.boxed());

            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", nodes::layout_text);

            let text = wgt.capture_var_or_default(property_id!(self.txt));
            wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", |child| nodes::resolve_text(child, text));
        });
    }

    impl_properties! {
        /// Spacing in-between the text and borders.
        pub fn crate::properties::padding(padding: impl IntoVar<SideOffsets>);
    }
}

/// The text string.
///
/// Set to an empty string (`""`) by default.
#[property(CHILD, capture, default(""))]
pub fn txt(child: impl UiNode, txt: impl IntoVar<Txt>) -> impl UiNode {}

///<span data-del-macro-root></span> A simple text run with **bold** font weight.
///
/// The input syntax is the same as the shorthand [`text!`].
///
/// # Configure
///
/// Apart from the font weight this widget can be configured with contextual properties like [`text!`].
///
/// [`text`]: mod@text
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
/// The input syntax is the same as the shorthand [`text!`].
///
/// # Configure
///
/// Apart from the font style this widget can be configured with contextual properties like [`text!`].
///
/// [`text`]: mod@text
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
