//! Context properties for theming the [`text!`](module@crate::widgets::text) widget.

use crate::core::types::*;
use crate::core::var::{context_var, IntoVar};
use crate::core::{color::web_colors, units::*};
use crate::core::{color::Rgba, text::*, units::TabLength};
use crate::core::{impl_ui_node, UiNode};
use crate::core::{property, var::Var};
use crate::properties::with_context_var;

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

    /// Font features of [`text`](crate::widgets::text) spans.
    pub struct FontFeaturesVar: FontFeatures = once FontFeatures::default();

    /// Text color of [`text`](crate::widgets::text) spans.
    pub struct TextColorVar: Rgba = const web_colors::WHITE;

    /// Text transformation function applied to [`text`](crate::widgets::text) spans.
    pub struct TextTransformVar: TextTransformFn = return &TextTransformFn::None;

    /// Text line height of [`text`](crate::widgets::text) spans.
    pub struct LineHeightVar: LineHeight = return &LineHeight::Font;

    /// Extra letter spacing of [`text`](crate::widgets::text) spans.
    pub struct LetterSpacingVar: LetterSpacing = return &LetterSpacing::Auto;

    /// Extra word spacing of [`text`](crate::widgets::text) spans.
    pub struct WordSpacingVar: WordSpacing = return &WordSpacing::Auto;

    /// Configuration of line breaks inside words during text wrap.
    pub struct WordBreakVar: WordBreak = return &WordBreak::Normal;

    /// Configuration of line breaks in Chinese, Japanese, or Korean text.
    pub struct LineBreakVar: LineBreak = return &LineBreak::Auto;

    /// Text line alignment in a text block.
    pub struct TextAlignVar: TextAlign = return &TextAlign::Start;

    /// Length of the `TAB` space.
    pub struct TabLengthVar: TabLength = once 400.pct().into();

    /// Text white space transform of [`text`](crate::widgets::text) spans.
    pub struct WhiteSpaceVar: WhiteSpace = return &WhiteSpace::Preserve;
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

/// Sets the [`WordBreakVar`] context var.
#[property(context)]
pub fn word_break(child: impl UiNode, mode: impl IntoVar<WordBreak>) -> impl UiNode {
    with_context_var(child, WordBreakVar, mode)
}

/// Sets the [`LineBreakVar`] context var.
#[property(context)]
pub fn line_break(child: impl UiNode, mode: impl IntoVar<LineBreak>) -> impl UiNode {
    with_context_var(child, LineBreakVar, mode)
}

/// Sets the [`TextAlignVar`] context var.
#[property(context)]
pub fn text_align(child: impl UiNode, mode: impl IntoVar<TextAlign>) -> impl UiNode {
    with_context_var(child, TextAlignVar, mode)
}

/// Sets the [`TabLengthVar`] context var.
#[property(context)]
pub fn tab_length(child: impl UiNode, length: impl IntoVar<TabLength>) -> impl UiNode {
    with_context_var(child, TabLengthVar, length)
}

/// Sets the [`WhiteSpaceVar`] context var.
#[property(context)]
pub fn white_space(child: impl UiNode, transform: impl IntoVar<WhiteSpace>) -> impl UiNode {
    with_context_var(child, WhiteSpaceVar, transform)
}

struct FontFeaturesNode<C: UiNode, F: Var<FontFeatures>> {
    child: C,
    features: F,
}

#[impl_ui_node(child)]
impl<C: UiNode, F: Var<FontFeatures>> UiNode for FontFeaturesNode<C, F> {}

/// Sets/overrides font features.
#[property(context)]
pub fn font_features(child: impl UiNode, features: impl IntoVar<FontFeatures>) -> impl UiNode {
    FontFeaturesNode {
        child,
        features: features.into_var(),
    }
}

/// Sets the font kerning feature.
#[property(context)]
pub fn font_kerning(child: impl UiNode, kerning: impl IntoVar<KerningState>) -> impl UiNode {
    font_features::set(child, kerning.into_var().map(|&k| FontFeatures::new().set_kerning(k)))
}

/// Sets the font common ligatures features.
#[property(context)]
pub fn font_common_lig(child: impl UiNode, state: impl IntoVar<LigatureState>) -> impl UiNode {
    font_features::set(child, state.into_var().map(|&k| FontFeatures::new().set_common_lig(k)))
}

/// Sets the font discretionary ligatures feature.
#[property(context)]
pub fn font_discretionary_lig(child: impl UiNode, state: impl IntoVar<LigatureState>) -> impl UiNode {
    font_features::set(child, state.into_var().map(|&k| FontFeatures::new().set_discretionary_lig(k)))
}

/// Sets the font historical ligatures feature.
#[property(context)]
pub fn font_historical_lig(child: impl UiNode, state: impl IntoVar<LigatureState>) -> impl UiNode {
    font_features::set(child, state.into_var().map(|&k| FontFeatures::new().set_historical_lig(k)))
}

/// Sets the font contextual alternatives feature.
#[property(context)]
pub fn font_contextual_alt(child: impl UiNode, state: impl IntoVar<LigatureState>) -> impl UiNode {
    font_features::set(child, state.into_var().map(|&k| FontFeatures::new().set_contextual_alt(k)))
}

/// Sets the font capital variant features.
#[property(context)]
pub fn font_caps_alt(child: impl UiNode, variant: impl IntoVar<CapsVariant>) -> impl UiNode {
    font_features::set(child, variant.into_var().map(|&k| FontFeatures::new().caps(k)))
}
