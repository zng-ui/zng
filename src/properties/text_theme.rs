//! Context properties for theming the [`text!`](module@crate::widgets::text) widget.

use crate::core::color::Rgba;
use crate::core::property;
use crate::core::types::*;
use crate::core::var::{context_var, IntoVar};
use crate::core::UiNode;
use crate::core::{
    color::web_colors,
    units::{Length, LetterSpacing, LineHeight, WordSpacing},
};
use crate::properties::with_context_var;
use std::{borrow::Cow, fmt, rc::Rc};

/// Text transform function.
#[derive(Clone)]
pub enum TextTransformFn {
    /// No transform.
    None,
    /// To UPPERCASE.
    Uppercase,
    /// to lowercase.
    Lowercase,
    /// Custom transform function.
    Custom(Rc<dyn Fn(&str) -> Cow<str>>),
}
impl TextTransformFn {
    pub fn transform<'a, 'b>(&'a self, text: &'b str) -> Cow<'b, str> {
        match self {
            TextTransformFn::None => Cow::Borrowed(text),
            TextTransformFn::Uppercase => Cow::Owned(text.to_uppercase()),
            TextTransformFn::Lowercase => Cow::Owned(text.to_lowercase()),
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    pub fn custom(fn_: impl Fn(&str) -> Cow<str> + 'static) -> Self {
        TextTransformFn::Custom(Rc::new(fn_))
    }
}
impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}

context_var! {
    /// Font family of [`text`](crate::widgets::text) spans.
    pub struct FontFamilyVar: Box<[FontName]> = once Box::new([
        FontName::sans_serif(),
        FontName::serif(),
        FontName::monospace(),
        FontName::cursive(),
        FontName::fantasy()
    ]);

    /// Font weight of [`text`](crate::widgets::text) spans.
    pub struct FontWeightVar: FontWeight = const FontWeight::NORMAL;

    /// Font style of [`text`](crate::widgets::text) spans.
    pub struct FontStyleVar: FontStyle = const FontStyle::Normal;

    /// Font stretch of [`text`](crate::widgets::text) spans.
    pub struct FontStretchVar: FontStretch = const FontStretch::NORMAL;

    /// Font size of [`text`](crate::widgets::text) spans.
    pub struct FontSizeVar: Length = once Length::pt(14.0);

    /// Text color of [`text`](crate::widgets::text) spans.
    pub struct TextColorVar: Rgba = const web_colors::WHITE;

    /// Text transformation function applied to [`text`](crate::widgets::text) spans.
    pub struct TextTransformVar: TextTransformFn = return &TextTransformFn::None;

    /// Text line height of [`text`](crate::widgets::text) spans.
    pub struct LineHeightVar: LineHeight = return &LineHeight::Font;

    /// Extra letter spacing of [`text`](crate::widgets::text) spans.
    pub struct LetterSpacingVar: LetterSpacing = return &LetterSpacing::Auto;

    /// Extra word spacing of [`text`](crate::widgets::text) spans.
    pub struct WordSpacingVar: WordSpacing = return &WordSpacing::Font;
}

/// Sets the [`FontFamilyVar`] context var.
#[property(context)]
pub fn font_family(child: impl UiNode, names: impl IntoVar<Box<[FontName]>>) -> impl UiNode {
    with_context_var(child, FontFamilyVar, names)
}

/// Sets the [`FontStyleVar`] context var.
#[property(context)]
pub fn font_style(child: impl UiNode, style: impl IntoVar<FontStyle>) -> impl UiNode {
    with_context_var(child, FontStyleVar, style)
}

/// Sets the [`FontWeightVar`] context var.
#[property(context)]
pub fn font_weight(child: impl UiNode, weight: impl IntoVar<FontWeight>) -> impl UiNode {
    with_context_var(child, FontWeightVar, weight)
}

/// Sets the [`FontStretchVar`] context var.
#[property(context)]
pub fn font_stretch(child: impl UiNode, stretch: impl IntoVar<FontStretch>) -> impl UiNode {
    with_context_var(child, FontStretchVar, stretch)
}

/// Sets the [`FontSizeVar`] context var.
#[property(context)]
pub fn font_size(child: impl UiNode, size: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, FontSizeVar, size)
}

/// Sets the [`TextColorVar`] context var.
#[property(context)]
pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, TextColorVar, color)
}

/// Sets the [`TextTransformVar`] context var.
#[property(context)]
pub fn text_transform(child: impl UiNode, transform: impl IntoVar<TextTransformFn>) -> impl UiNode {
    with_context_var(child, TextTransformVar, transform)
}

/// Sets the [`LineHeightVar`] context var.
#[property(context)]
pub fn line_height(child: impl UiNode, height: impl IntoVar<LineHeight>) -> impl UiNode {
    with_context_var(child, LineHeightVar, height)
}

/// Sets the [`LetterSpacingVar`] context var.
#[property(context)]
pub fn letter_spacing(child: impl UiNode, extra: impl IntoVar<LetterSpacing>) -> impl UiNode {
    with_context_var(child, LetterSpacingVar, extra)
}

/// Sets the [`WordSpacingVar`] context var.
#[property(context)]
pub fn word_spacing(child: impl UiNode, extra: impl IntoVar<WordSpacing>) -> impl UiNode {
    with_context_var(child, WordSpacingVar, extra)
}
