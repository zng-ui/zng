//! Context properties for theming the [`text!`](module@crate::widgets::text) widget.

use std::marker::PhantomData;

use crate::core::{color::web_colors, units::*};
use crate::core::{
    color::Rgba,
    text::{font_features::*, *},
    units::TabLength,
};
use crate::core::{
    context::WidgetContext,
    var::{context_var, IntoVar},
};
use crate::core::{impl_ui_node, UiNode};
use crate::core::{property, var::Var};
use crate::core::{types::*, var::VarValue};
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

struct WithFontFeatureNode<C: UiNode, S: VarValue, V: Var<S>, D: FnMut(&mut FontFeatures, S) -> S + 'static> {
    child: C,
    _s: PhantomData<S>,
    var: V,
    delegate: D,
}
#[impl_ui_node(child)]
impl<C: UiNode, S: VarValue, V: Var<S>, D: FnMut(&mut FontFeatures, S) -> S + 'static> UiNode for WithFontFeatureNode<C, S, V, D> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let state = self.var.get(ctx.vars).clone();
        let prev = (self.delegate)(&mut FontFeatures::new(), state);
        println!("TODO {:?}", prev);
        self.child.init(ctx);
    }
}

/// Include the font feature config in the widget context.
pub fn with_font_feature<C: UiNode, S: VarValue, V: IntoVar<S>, D: FnMut(&mut FontFeatures, S) -> S + 'static>(
    child: C,
    state: V,
    set_feature: D,
) -> impl UiNode {
    WithFontFeatureNode {
        child,
        _s: PhantomData,
        var: state.into_var(),
        delegate: set_feature,
    }
}

struct FontFeaturesNode<C: UiNode, V: Var<FontFeatures>> {
    child: C,
    features: V,
}
#[impl_ui_node(child)]
impl<C: UiNode, V: Var<FontFeatures>> UiNode for FontFeaturesNode<C, V> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let features = self.features.get(ctx.vars);
        println!("TODO {:?}", features);
        self.child.init(ctx);
    }
}

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
pub fn font_kerning(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.kerning().set(s))
}

/// Sets the font common ligatures features.
#[property(context)]
pub fn font_common_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.common_lig().set(s))
}

/// Sets the font discretionary ligatures feature.
#[property(context)]
pub fn font_discretionary_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.discretionary_lig().set(s))
}

/// Sets the font historical ligatures feature.
#[property(context)]
pub fn font_historical_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.historical_lig().set(s))
}

/// Sets the font contextual alternatives feature.
#[property(context)]
pub fn font_contextual_alt(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.contextual_alt().set(s))
}

/// Sets the font capital variant features.
#[property(context)]
pub fn font_caps(child: impl UiNode, state: impl IntoVar<CapsVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.caps().set(s))
}

/// Sets the font numeric variant features.
#[property(context)]
pub fn font_numeric(child: impl UiNode, state: impl IntoVar<NumVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.numeric().set(s))
}

/// Sets the font numeric spacing features.
#[property(context)]
pub fn font_num_spacing(child: impl UiNode, state: impl IntoVar<NumSpacing>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.num_spacing().set(s))
}

/// Sets the font numeric fraction features.
#[property(context)]
pub fn font_num_fraction(child: impl UiNode, state: impl IntoVar<NumFraction>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.num_fraction().set(s))
}

/// Sets the font swash features.
#[property(context)]
pub fn font_swash(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.swash().set(s))
}

/// Sets the font stylistic alternative feature.
#[property(context)]
pub fn font_stylistic(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.stylistic().set(s))
}

/// Sets the font historical forms alternative feature.
#[property(context)]
pub fn font_historical_forms(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.historical_forms().set(s))
}

/// Sets the font ornaments alternative feature.
#[property(context)]
pub fn font_ornaments(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.ornaments().set(s))
}

/// Sets the font annotation alternative feature.
#[property(context)]
pub fn font_annotation(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.annotation().set(s))
}

/// Sets the font stylistic set alternative feature.
#[property(context)]
pub fn font_style_set(child: impl UiNode, state: impl IntoVar<StyleSet>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.style_set().set(s))
}

/// Sets the font character variant alternative feature.
#[property(context)]
pub fn font_char_variant(child: impl UiNode, state: impl IntoVar<CharVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.char_variant().set(s))
}

/// Sets the font sub/super script position alternative feature.
#[property(context)]
pub fn font_position(child: impl UiNode, state: impl IntoVar<FontPosition>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.position().set(s))
}

/// Sets the East Asian logographic set.
#[property(context)]
pub fn font_ea_variant(child: impl UiNode, state: impl IntoVar<EastAsianVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.east_asian_variant().set(s))
}

/// Sets the East Asian figure width.
#[property(context)]
pub fn font_ea_width(child: impl UiNode, state: impl IntoVar<EastAsianWidth>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.east_asian_width().set(s))
}
