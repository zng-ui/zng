//! Text widgets and properties.

use crate::prelude::new_widget::*;

pub mod commands;
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
#[rustfmt::skip]
pub struct Text(
    FontMix<
    TextFillMix<
    TextAlignMix<
    TextWrapMix<
    TextDecorationMix<
    TextSpacingMix<
    TextTransformMix<
    LangMix<
    FontFeaturesMix<
    TextEditMix<
    WidgetBase
    >>>>>>>>>>
);

/// The text string.
///
/// Set to an empty string (`""`) by default.
#[property(CHILD, capture, default(""), widget_impl(Text))]
pub fn txt(txt: impl IntoVar<Txt>) {}

/// Value that is parsed from the text and displayed as the text.
///
/// This is an alternative to [`txt`] that converts to and from `T`. If `T: VarValue + Display + FromStr where FromStr::Err: Display`
/// the type is compatible with this property.
///
/// If the parse operation fails the value variable is not updated and the error display text is set in [`DATA.invalidate`], you
/// can use [`has_data_error`] and [`get_data_error_txt`] to display the error.
///
/// See also [`txt_parse_live`] for ways to control when the parse attempt happens.
///
/// [`txt`]: fn@txt
/// [`txt_parse_live`]: fn@txt_parse_live
/// [`DATA.invalidate`]: crate::properties::data_context::DATA::invalidate
/// [`has_data_error`]: fn@crate::properties::data_context::has_data_error
/// [`get_data_error_txt`]: fn@crate::properties::data_context::get_data_error_txt
#[property(CHILD, widget_impl(Text))]
pub fn txt_parse<T>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode
where
    T: TxtParseValue,
{
    nodes::parse_text(child, value)
}

/// A type that can be a var value, parse and display.
///
/// This trait is used by [`txt_parse`]. It is implemented for all types that are
/// `VarValue + FromStr + Display where FromStr::Err: Display`.
///
/// [`txt_parse`]: fn@txt_parse
pub trait TxtParseValue: VarValue {
    /// Try parse `Self` from `txt`, formats the error for display.
    ///
    /// Note that the widget context is not available here as this method is called in the app context.
    fn from_txt(txt: &Txt) -> Result<Self, Txt>;
    /// Display the value, the returned text can be parsed back to an equal value.
    ///
    /// Note that the widget context is not available here as this method is called in the app context.
    fn to_txt(&self) -> Txt;
}
impl<T> TxtParseValue for T
where
    T: VarValue + std::str::FromStr + std::fmt::Display,
    <Self as std::str::FromStr>::Err: std::fmt::Display,
{
    fn from_txt(txt: &Txt) -> Result<Self, Txt> {
        T::from_str(txt).map_err(|e| e.to_text())
    }

    fn to_txt(&self) -> Txt {
        self.to_text()
    }
}

impl Text {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = nodes::render_text();
            let child = nodes::render_caret(child);
            let child = nodes::touch_carets(child);
            let child = nodes::render_overlines(child);
            let child = nodes::render_strikethroughs(child);
            let child = nodes::render_underlines(child);
            let child = nodes::render_selection(child);
            wgt.set_child(child.boxed());

            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", nodes::layout_text);

            if let Some(txt_parse) = wgt.capture_property(property_id!(Self::txt_parse)) {
                let txt_parse = txt_parse.args.clone_boxed();
                let text = wgt
                    .capture_var(property_id!(Self::txt))
                    .unwrap_or_else(|| var(Txt::from_str("")).boxed());

                wgt.push_intrinsic(NestGroup::EVENT, "resolve_text+parse", move |child| {
                    let child = txt_parse.instantiate(child);
                    nodes::resolve_text(child, text)
                });
            } else {
                let text = wgt.capture_var_or_default(property_id!(Self::txt));
                wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", |child| nodes::resolve_text(child, text));
            }
        });
    }
}

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
