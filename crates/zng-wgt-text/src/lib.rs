#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Text widgets and properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::prelude::*;

#[macro_use]
extern crate bitflags;

pub mod cmd;
pub mod node;
mod text_properties;
pub use text_properties::*;

#[doc(hidden)]
pub use zng_wgt::prelude::formatx as __formatx;

pub mod icon;

/// A configured text run.
///
/// # Examples
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt_text::*;
/// # fn main() {
/// let hello_txt = Text! {
///     font_family = "Arial";
///     font_size = 18;
///     txt = "Hello!";
/// };
/// # }
/// ```
/// # Shorthand
///
/// The `Text!` macro provides shorthand syntax that matches the [`formatx!`] input, but outputs a text widget:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt_text::*;
/// # fn main() {
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
/// # }
/// ```
///
/// The code abode is equivalent to:
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt_text::*;
/// # fn main() {
/// # use zng_wgt::prelude::*;
/// let txt = Text! {
///     txt = formatx!("Hello!");
/// };
///
/// let name = "World";
/// let fmt = Text! {
///     txt = formatx!("Hello {}!", name);
/// };
///
/// let expr = Text! {
///     txt = {
///         let mut s = String::new();
///         s.push('a');
///         s
///     };
/// };
/// # }
/// ```
///
/// [`formatx!`]: zng_wgt::prelude::formatx
#[widget($crate::Text {
    ($txt:literal) => {
        txt = $crate::__formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::__formatx!($txt, $($format)*);
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
    SelectionToolbarMix<
    TextInspectMix<
    WidgetBase
    >>>>>>>>>>>>
);

impl Text {
    /// Context variables used by properties in text.
    pub fn context_vars_set(set: &mut ContextValueSet) {
        FontMix::<()>::context_vars_set(set);
        TextFillMix::<()>::context_vars_set(set);
        TextAlignMix::<()>::context_vars_set(set);
        TextWrapMix::<()>::context_vars_set(set);
        TextDecorationMix::<()>::context_vars_set(set);
        TextSpacingMix::<()>::context_vars_set(set);
        TextTransformMix::<()>::context_vars_set(set);
        FontFeaturesMix::<()>::context_vars_set(set);
        TextEditMix::<()>::context_vars_set(set);
        SelectionToolbarMix::<()>::context_vars_set(set);
        TextInspectMix::<()>::context_vars_set(set);

        LangMix::<()>::context_vars_set(set);
    }
}

/// The text string.
///
/// Set to an empty string (`""`) by default.
#[property(CHILD, capture, default(""), widget_impl(Text))]
pub fn txt(txt: impl IntoVar<Txt>) {}

/// Value that is parsed from the text and displayed as the text.
///
/// This is an alternative to [`txt`] that converts to and from `T` if it can be formatted to display text and can parse, with
/// parse error that can display.
///
/// If the parse operation fails the value variable is not updated and the error display text is set in [`DATA.invalidate`], you
/// can use [`has_data_error`] and [`get_data_error_txt`] to display the error.
///
/// See also [`txt_parse_live`] for ways to control when the parse attempt happens.
///
/// [`txt`]: fn@txt
/// [`txt_parse_live`]: fn@txt_parse_live
/// [`DATA.invalidate`]: zng_wgt_data::DATA::invalidate
/// [`has_data_error`]: fn@zng_wgt_data::has_data_error
/// [`get_data_error_txt`]: fn@zng_wgt_data::get_data_error_txt
#[property(CHILD, widget_impl(Text))]
pub fn txt_parse<T>(child: impl IntoUiNode, value: impl IntoVar<T>) -> UiNode
where
    T: TxtParseValue,
{
    node::parse_text(child, value)
}

/// Represents a type that can be a var value, parse and display.
///
/// This trait is used by [`txt_parse`]. It is implemented for all types that are
/// `VarValue + FromStr + Display where FromStr::Err: Display`.
///
/// [`txt_parse`]: fn@txt_parse
#[diagnostic::on_unimplemented(note = "`TxtParseValue` is implemented for all `T: VarValue + Display + FromStr<Error: Display>")]
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
        T::from_str(txt).map_err(|e| e.to_txt())
    }

    fn to_txt(&self) -> Txt {
        ToTxt::to_txt(self)
    }
}

impl Text {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = node::render_text();
            let child = node::non_interactive_caret(child);
            let child = node::interactive_carets(child);
            let child = node::render_overlines(child);
            let child = node::render_strikethroughs(child);
            let child = node::render_underlines(child);
            let child = node::render_ime_preview_underlines(child);
            let child = node::render_selection(child);
            wgt.set_child(child);

            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", |child| {
                let child = node::selection_toolbar_node(child);
                node::layout_text(child)
            });

            let text = if wgt.property(property_id!(Self::txt_parse)).is_some() {
                wgt.capture_var(property_id!(Self::txt)).unwrap_or_else(|| var(Txt::from_str("")))
            } else {
                wgt.capture_var_or_default(property_id!(Self::txt))
            };
            wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", |child| {
                let child = node::rich_text_component(child, "text");
                node::resolve_text(child, text)
            });
        });
    }
}

#[doc(hidden)]
pub use zng_ext_font::{FontStyle as __FontStyle, FontWeight as __FontWeight};

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
        $crate::Text! {
            txt = $txt;
            font_weight = $crate::__FontWeight::BOLD;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::Text! {
            txt = $crate::__formatx!($txt, $($format)*);
            font_weight = $crate::__FontWeight::BOLD;
        }
    };
}

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
        $crate::Text! {
            txt = $txt;
            font_style = $crate::__FontStyle::Italic;
        }
    };
    ($txt:tt, $($format:tt)*) => {
        $crate::Text! {
            txt = $crate::__formatx!($txt, $($format)*);
            font_style = $crate::__FontStyle::Italic;
        }
    };
}
