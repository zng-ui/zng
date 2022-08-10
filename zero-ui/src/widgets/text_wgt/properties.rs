//! Properties and context variables that configure the appearance of text widgets.

use crate::core::text::{font_features::*, *};
use crate::prelude::new_property::*;

context_var! {
    /// Font family of [`text`](crate::widgets::text) spans.
    pub struct FontFamilyVar: FontNames = FontNames::default();

    /// Font weight of [`text`](crate::widgets::text) spans.
    pub struct FontWeightVar: FontWeight = FontWeight::NORMAL;

    /// Font style of [`text`](crate::widgets::text) spans.
    pub struct FontStyleVar: FontStyle = FontStyle::Normal;

    /// Font stretch of [`text`](crate::widgets::text) spans.
    pub struct FontStretchVar: FontStretch = FontStretch::NORMAL;

    /// Font synthesis of [`text`](crate::widgets::text) spans.
    pub struct FontSynthesisVar: FontSynthesis = FontSynthesis::ENABLED;

    /// Font anti-aliasing of [`text`](crate::widgets::text) spans.
    pub struct FontAaVar: FontAntiAliasing = FontAntiAliasing::Default;

    /// Font size of [`text`](crate::widgets::text) spans.
    pub struct FontSizeVar: FontSize = FontSize::Pt(11.0);

    /// Text color of [`text`](crate::widgets::text) spans.
    pub struct TextColorVar: Rgba = colors::WHITE;

    /// Text transformation function applied to [`text`](crate::widgets::text) spans.
    pub struct TextTransformVar: TextTransformFn = TextTransformFn::None;

    /// Text line height of [`text`](crate::widgets::text) spans.
    pub struct LineHeightVar: LineHeight = LineHeight::Default;

    /// Extra spacing in between lines of [`text`](crate::widgets::text) spans.
    pub struct LineSpacingVar: Length = Length::Px(Px(0));

    /// Extra letter spacing of [`text`](crate::widgets::text) spans.
    pub struct LetterSpacingVar: LetterSpacing = LetterSpacing::Default;

    /// Extra word spacing of [`text`](crate::widgets::text) spans.
    pub struct WordSpacingVar: WordSpacing = WordSpacing::Default;

    /// Extra paragraph spacing of text blocks.
    pub struct ParagraphSpacingVar: ParagraphSpacing = Length::Px(Px(0));

    /// Configuration of line breaks inside words during text wrap.
    pub struct WordBreakVar: WordBreak = WordBreak::Normal;

    /// Configuration of line breaks in Chinese, Japanese, or Korean text.
    pub struct LineBreakVar: LineBreak = LineBreak::Auto;

    /// Text line alignment in a text block.
    pub struct TextAlignVar: TextAlign = TextAlign::START;

    /// Length of the `TAB` space.
    pub struct TabLengthVar: TabLength = 400.pct().into();

    /// Text white space transform of [`text`](crate::widgets::text) spans.
    pub struct WhiteSpaceVar: WhiteSpace = WhiteSpace::Preserve;

    /// Font features of [`text`](crate::widgets::text) spans.
    pub struct FontFeaturesVar: FontFeatures = FontFeatures::new();

    /// Font variations of [`text`](crate::widgets::text) spans.
    pub struct FontVariationsVar: FontVariations = FontVariations::new();

    /// Language of [`text`](crate::widgets::text) spans.
    pub struct LangVar: Lang = Lang::default();

    /// Underline thickness.
    pub struct UnderlineThicknessVar: UnderlineThickness = 0.into();
    /// Underline style.
    pub struct UnderlineStyleVar: LineStyle = LineStyle::Hidden;
    /// Underline color.
    pub struct UnderlineColorVar: TextLineColor = TextLineColor::Text;
    /// Parts of text skipped by underline.
    pub struct UnderlineSkipVar: UnderlineSkip = UnderlineSkip::DEFAULT;
    /// Position of the underline.
    pub struct UnderlinePositionVar: UnderlinePosition = UnderlinePosition::Font;

    /// Overline thickness.
    pub struct OverlineThicknessVar: TextLineThickness = 0.into();
    /// Overline style.
    pub struct OverlineStyleVar: LineStyle = LineStyle::Hidden;
    /// Overline color.
    pub struct OverlineColorVar: TextLineColor = TextLineColor::Text;

    /// Strikethrough thickness.
    pub struct StrikethroughThicknessVar: TextLineThickness = 0.into();
    /// Strikethrough style.
    pub struct  StrikethroughStyleVar: LineStyle = LineStyle::Hidden;
    /// Strikethrough color.
    pub struct StrikethroughColorVar: TextLineColor = TextLineColor::Text;

    /// Text is editable.
    pub struct TextEditableVar: bool = false;
}

/// Sets the [`FontFamilyVar`] context var.
#[property(context, default(FontFamilyVar))]
pub fn font_family(child: impl UiNode, names: impl IntoVar<FontNames>) -> impl UiNode {
    with_context_var(child, FontFamilyVar, names)
}

/// Sets the [`FontStyleVar`] context var.
#[property(context, default(FontStyleVar))]
pub fn font_style(child: impl UiNode, style: impl IntoVar<FontStyle>) -> impl UiNode {
    with_context_var(child, FontStyleVar, style)
}

/// Sets the [`FontWeightVar`] context var.
#[property(context, default(FontWeightVar))]
pub fn font_weight(child: impl UiNode, weight: impl IntoVar<FontWeight>) -> impl UiNode {
    with_context_var(child, FontWeightVar, weight)
}

/// Sets the [`FontStretchVar`] context var.
#[property(context, default(FontStretchVar))]
pub fn font_stretch(child: impl UiNode, stretch: impl IntoVar<FontStretch>) -> impl UiNode {
    with_context_var(child, FontStretchVar, stretch)
}

/// Sets the [`FontSynthesisVar`] context var.
#[property(context, default(FontSynthesisVar))]
pub fn font_synthesis(child: impl UiNode, enabled: impl IntoVar<FontSynthesis>) -> impl UiNode {
    with_context_var(child, FontSynthesisVar, enabled)
}

/// Sets the [`FontAaVar`] context var.
#[property(context, default(FontAaVar))]
pub fn font_aa(child: impl UiNode, aa: impl IntoVar<FontAntiAliasing>) -> impl UiNode {
    with_context_var(child, FontAaVar, aa)
}

/// Sets the [`FontSizeVar`] context var and the [`LayoutMetrics::font_size`].
#[property(context, default(FontSizeVar))]
pub fn font_size(child: impl UiNode, size: impl IntoVar<FontSize>) -> impl UiNode {
    struct FontSizeNode<C> {
        child: C,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for FontSizeNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &FontSizeVar::new());
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if FontSizeVar::is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let font_size = FontSizeVar::get(ctx.vars).layout(ctx.for_y(), |ctx| ctx.metrics.root_font_size());
            ctx.with_font_size(font_size, |ctx| self.child.measure(ctx))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let font_size = FontSizeVar::get(ctx.vars).layout(ctx.for_y(), |ctx| ctx.metrics.root_font_size());
            ctx.with_font_size(font_size, |ctx| self.child.layout(ctx, wl))
        }
    }
    let child = FontSizeNode { child };
    with_context_var(child, FontSizeVar, size)
}

/// Sets the [`TextColorVar`] context var.
#[property(context, default(TextColorVar))]
pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, TextColorVar, color)
}

/// Sets the [`TextTransformVar`] context var.
#[property(context, default(TextTransformVar))]
pub fn text_transform(child: impl UiNode, transform: impl IntoVar<TextTransformFn>) -> impl UiNode {
    with_context_var(child, TextTransformVar, transform)
}

/// Sets the [`LineHeightVar`] context var.
#[property(context, default(LineHeightVar))]
pub fn line_height(child: impl UiNode, height: impl IntoVar<LineHeight>) -> impl UiNode {
    with_context_var(child, LineHeightVar, height)
}

/// Sets the [`LetterSpacingVar`] context var.
#[property(context, default(LetterSpacingVar))]
pub fn letter_spacing(child: impl UiNode, extra: impl IntoVar<LetterSpacing>) -> impl UiNode {
    with_context_var(child, LetterSpacingVar, extra)
}

/// Sets the [`LineSpacingVar`] context var.
#[property(context, default(LineSpacingVar))]
pub fn line_spacing(child: impl UiNode, extra: impl IntoVar<LineSpacing>) -> impl UiNode {
    with_context_var(child, LineSpacingVar, extra)
}

/// Sets the [`WordSpacingVar`] context var.
#[property(context, default(WordSpacingVar))]
pub fn word_spacing(child: impl UiNode, extra: impl IntoVar<WordSpacing>) -> impl UiNode {
    with_context_var(child, WordSpacingVar, extra)
}

/// Sets the [`ParagraphSpacingVar`] context var.
#[property(context, default(ParagraphSpacingVar))]
pub fn paragraph_spacing(child: impl UiNode, extra: impl IntoVar<ParagraphSpacing>) -> impl UiNode {
    with_context_var(child, ParagraphSpacingVar, extra)
}

/// Sets the [`WordBreakVar`] context var.
#[property(context, default(WordBreakVar))]
pub fn word_break(child: impl UiNode, mode: impl IntoVar<WordBreak>) -> impl UiNode {
    with_context_var(child, WordBreakVar, mode)
}

/// Sets the [`LineBreakVar`] context var.
#[property(context, default(LineBreakVar))]
pub fn line_break(child: impl UiNode, mode: impl IntoVar<LineBreak>) -> impl UiNode {
    with_context_var(child, LineBreakVar, mode)
}

/// Sets the [`TextAlignVar`] context var.
#[property(context, default(TextAlignVar))]
pub fn text_align(child: impl UiNode, mode: impl IntoVar<TextAlign>) -> impl UiNode {
    with_context_var(child, TextAlignVar, mode)
}

/// Sets the [`TabLengthVar`] context var.
#[property(context, default(TabLengthVar))]
pub fn tab_length(child: impl UiNode, length: impl IntoVar<TabLength>) -> impl UiNode {
    with_context_var(child, TabLengthVar, length)
}

/// Sets the [`WhiteSpaceVar`] context var.
#[property(context, default(WhiteSpaceVar))]
pub fn white_space(child: impl UiNode, transform: impl IntoVar<WhiteSpace>) -> impl UiNode {
    with_context_var(child, WhiteSpaceVar, transform)
}

/// Includes the font variation config in the widget context.
///
/// The variation `name` is set for the [`FontVariationsVar`] in this context, variations already set in the parent
/// context that are not the same `name` are also included.
pub fn with_font_variation(child: impl UiNode, name: FontVariationName, value: impl IntoVar<f32>) -> impl UiNode {
    with_context_var(
        child,
        FontVariationsVar,
        merge_var!(FontVariationsVar::new(), value.into_var(), move |variations, value| {
            let mut variations = variations.clone();
            variations.insert(name, *value);
            variations
        }),
    )
}

/// Include the font feature config in the widget context.
///
/// The modifications done in `set_feature` are visible only in the [`FontFeaturesVar`] in this context, and features
/// already set in a parent context are included.
pub fn with_font_feature<C, S, V, D>(child: C, state: V, set_feature: D) -> impl UiNode
where
    C: UiNode,
    S: VarValue,
    V: IntoVar<S>,
    D: FnMut(&mut FontFeatures, S) -> S + 'static,
{
    let mut set_feature = set_feature;
    with_context_var(
        child,
        FontFeaturesVar,
        merge_var!(FontFeaturesVar::new(), state.into_var(), move |features, state| {
            let mut features = features.clone();
            set_feature(&mut features, state.clone());
            features
        }),
    )
}

/// Sets font variations.
///
/// **Note:** This property fully replaces the font variations for the widget and descendants, use [`with_font_variation`]
/// to create a property that sets a variation but retains others from the context.
#[property(context)]
pub fn font_variations(child: impl UiNode, variations: impl IntoVar<FontVariations>) -> impl UiNode {
    with_context_var(child, FontVariationsVar, variations)
}

/// Sets font features.
///
/// **Note:** This property fully replaces the font variations for the widget and descendants, use [`with_font_variation`]
/// to create a property that sets a variation but retains others from the context.
#[property(context)]
pub fn font_features(child: impl UiNode, features: impl IntoVar<FontFeatures>) -> impl UiNode {
    with_context_var(child, FontFeaturesVar, features)
}

/// Sets the font kerning feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_kerning(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.kerning().set(s))
}

/// Sets the font common ligatures features.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_common_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.common_lig().set(s))
}

/// Sets the font discretionary ligatures feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_discretionary_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.discretionary_lig().set(s))
}

/// Sets the font historical ligatures feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_historical_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.historical_lig().set(s))
}

/// Sets the font contextual alternatives feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_contextual_alt(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.contextual_alt().set(s))
}

/// Sets the font capital variant features.
#[property(context, default(CapsVariant::Auto))]
pub fn font_caps(child: impl UiNode, state: impl IntoVar<CapsVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.caps().set(s))
}

/// Sets the font numeric variant features.
#[property(context, default(NumVariant::Auto))]
pub fn font_numeric(child: impl UiNode, state: impl IntoVar<NumVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.numeric().set(s))
}

/// Sets the font numeric spacing features.
#[property(context, default(NumSpacing::Auto))]
pub fn font_num_spacing(child: impl UiNode, state: impl IntoVar<NumSpacing>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.num_spacing().set(s))
}

/// Sets the font numeric fraction features.
#[property(context, default(NumFraction::Auto))]
pub fn font_num_fraction(child: impl UiNode, state: impl IntoVar<NumFraction>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.num_fraction().set(s))
}

/// Sets the font swash features.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_swash(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.swash().set(s))
}

/// Sets the font stylistic alternative feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_stylistic(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.stylistic().set(s))
}

/// Sets the font historical forms alternative feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_historical_forms(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.historical_forms().set(s))
}

/// Sets the font ornaments alternative feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_ornaments(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.ornaments().set(s))
}

/// Sets the font annotation alternative feature.
#[property(context, default(FontFeatureState::auto()))]
pub fn font_annotation(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.annotation().set(s))
}

/// Sets the font stylistic set alternative feature.
#[property(context, default(FontStyleSet::auto()))]
pub fn font_style_set(child: impl UiNode, state: impl IntoVar<FontStyleSet>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.style_set().set(s))
}

/// Sets the font character variant alternative feature.
#[property(context, default(CharVariant::auto()))]
pub fn font_char_variant(child: impl UiNode, state: impl IntoVar<CharVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.char_variant().set(s))
}

/// Sets the font sub/super script position alternative feature.
#[property(context, default(FontPosition::Auto))]
pub fn font_position(child: impl UiNode, state: impl IntoVar<FontPosition>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.position().set(s))
}

/// Sets the Japanese logographic set.
#[property(context, default(JpVariant::Auto))]
pub fn font_jp_variant(child: impl UiNode, state: impl IntoVar<JpVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.jp_variant().set(s))
}

/// Sets the Chinese logographic set.
#[property(context, default(CnVariant::Auto))]
pub fn font_cn_variant(child: impl UiNode, state: impl IntoVar<CnVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.cn_variant().set(s))
}

/// Sets the East Asian figure width.
#[property(context, default(EastAsianWidth::Auto))]
pub fn font_ea_width(child: impl UiNode, state: impl IntoVar<EastAsianWidth>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.ea_width().set(s))
}

/// Sets the [`LangVar`] context var.
#[property(context, default(LangVar))]
pub fn lang(child: impl UiNode, lang: impl IntoVar<Lang>) -> impl UiNode {
    with_context_var(child, LangVar, lang)
}

/// Sets the [`UnderlineThicknessVar`] and [`UnderlineStyleVar`].
#[property(context, default(UnderlineThicknessVar, UnderlineStyleVar))]
pub fn underline(child: impl UiNode, thickness: impl IntoVar<UnderlineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, UnderlineThicknessVar, thickness);
    with_context_var(child, UnderlineStyleVar, style)
}
/// Sets the [`UnderlineColorVar`].
#[property(context, default(UnderlineColorVar))]
pub fn underline_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, UnderlineColorVar, color)
}
/// Sets the [`UnderlineSkipVar`].
#[property(context, default(UnderlineSkipVar))]
pub fn underline_skip(child: impl UiNode, skip: impl IntoVar<UnderlineSkip>) -> impl UiNode {
    with_context_var(child, UnderlineSkipVar, skip)
}
/// Sets the [`UnderlinePosition`].
#[property(context, default(UnderlinePositionVar))]
pub fn underline_position(child: impl UiNode, position: impl IntoVar<UnderlinePosition>) -> impl UiNode {
    with_context_var(child, UnderlinePositionVar, position)
}

/// Sets the [`OverlineThicknessVar`] and [`OverlineStyleVar`].
#[property(context, default(OverlineThicknessVar, OverlineStyleVar))]
pub fn overline(child: impl UiNode, thickness: impl IntoVar<TextLineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, OverlineThicknessVar, thickness);
    with_context_var(child, OverlineStyleVar, style)
}
/// Sets the [`OverlineColorVar`].
#[property(context, default(OverlineColorVar))]
pub fn overline_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, OverlineColorVar, color)
}

/// Sets the [`StrikethroughThicknessVar`] and [`StrikethroughStyleVar`].
#[property(context, default(StrikethroughThicknessVar, StrikethroughStyleVar))]
pub fn strikethrough(child: impl UiNode, thickness: impl IntoVar<TextLineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, StrikethroughThicknessVar, thickness);
    with_context_var(child, StrikethroughStyleVar, style)
}
/// Sets the [`StrikethroughColorVar`].
#[property(context, default(StrikethroughColorVar))]
pub fn strikethrough_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, StrikethroughColorVar, color)
}

/// Sets the [`TextEditableVar`].
#[property(context, default(TextEditableVar))]
pub fn text_editable(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, TextEditableVar, enabled)
}

/// All the text contextual values.
#[derive(Debug)]
pub struct TextContext<'a> {
    /* Affects font */
    /// The [`lang`](fn@lang) value.
    pub lang: &'a Lang,
    /// The [`font_family`](fn@font_family) value.
    pub font_family: &'a [FontName],
    /// The [`font_style`](fn@font_style) value.
    pub font_style: FontStyle,
    /// The [`font_weight`](fn@font_weight) value.
    pub font_weight: FontWeight,
    /// The [`font_stretch`](fn@font_stretch) value.
    pub font_stretch: FontStretch,

    /* Affects text characters */
    /// The [`text_transform`](fn@text_transform) value.
    pub text_transform: TextTransformFn,
    /// The [`white_space`](fn@white_space) value.
    pub white_space: WhiteSpace,

    /* Affects font instance */
    /// The [`font_size`](fn@font_size) value.
    pub font_size: &'a FontSize,
    /// The [`font_variations`](fn@font_variations) value.    
    pub font_variations: &'a FontVariations,

    /* Affects measure */
    /// The [`line_height`](fn@line_height) value.
    pub line_height: &'a LineHeight,
    /// The [`letter_spacing`](fn@letter_spacing) value.
    pub letter_spacing: &'a LetterSpacing,
    /// The [`word_spacing`](fn@word_spacing) value.
    pub word_spacing: &'a WordSpacing,
    /// The [`line_spacing`](fn@line_spacing) value.
    pub line_spacing: &'a LineSpacing,
    /// The [`tab_length`](fn@tab_length) value.
    pub tab_length: &'a TabLength,

    /// The [`font_features`](fn@font_features) value.
    pub font_features: &'a FontFeatures,

    /// The [`word_break`](fn@word_break) value.
    pub word_break: WordBreak,
    /// The [`line_break`](fn@line_break) value.
    pub line_break: LineBreak,

    /* Affects arrange */
    /// The [`text_align`](fn@text_align) value.
    pub text_align: TextAlign,

    /* Affects render only */
    /// The [`text_color`](fn@text_color) value.
    pub text_color: Rgba,

    /* Maybe affects render only */
    /// The [`font_synthesis`](fn@font_synthesis) value.
    pub font_synthesis: FontSynthesis,
    /// The [`font_aa`](fn@font_aa) value.
    pub font_aa: FontAntiAliasing,

    /// The [`overline`](fn@overline) values.
    pub overline: (&'a Length, LineStyle),
    /// The [`overline_color`](fn@overline_color) value.
    pub overline_color: TextLineColor,

    /// The [`strikethrough`](fn@strikethrough) values.
    pub strikethrough: (&'a Length, LineStyle),
    /// The [`strikethrough_color`](fn@strikethrough_color) value.
    pub strikethrough_color: TextLineColor,

    /// The [`underline`](fn@underline) values.
    pub underline: (&'a Length, LineStyle),
    /// The [`underline_color`](fn@underline_color) value.
    pub underline_color: TextLineColor,
    /// The [`underline_skip`](fn@underline_skip) value.
    pub underline_skip: UnderlineSkip,
    /// The [`underline_position`](fn@underline_position) value.
    pub underline_position: UnderlinePosition,
}
impl<'a> TextContext<'a> {
    /// Register all text context variables in the widget.
    pub fn subscribe(vars: &VarsRead, widget: &mut WidgetSubscriptions) {
        widget
            .vars(vars)
            .var(&LangVar::new())
            .var(&FontFamilyVar::new())
            .var(&FontStyleVar::new())
            .var(&FontWeightVar::new())
            .var(&FontStretchVar::new())
            .var(&TextTransformVar::new())
            .var(&WhiteSpaceVar::new())
            .var(&FontSizeVar::new())
            .var(&FontVariationsVar::new())
            .var(&LineHeightVar::new())
            .var(&LetterSpacingVar::new())
            .var(&WordSpacingVar::new())
            .var(&LineSpacingVar::new())
            .var(&WordBreakVar::new())
            .var(&LineBreakVar::new())
            .var(&TabLengthVar::new())
            .var(&FontFeaturesVar::new())
            .var(&TextAlignVar::new())
            .var(&TextColorVar::new())
            .var(&FontSynthesisVar::new())
            .var(&FontAaVar::new())
            .var(&OverlineThicknessVar::new())
            .var(&OverlineStyleVar::new())
            .var(&OverlineColorVar::new())
            .var(&StrikethroughThicknessVar::new())
            .var(&StrikethroughStyleVar::new())
            .var(&StrikethroughColorVar::new())
            .var(&UnderlineThicknessVar::new())
            .var(&UnderlineColorVar::new())
            .var(&UnderlineSkipVar::new())
            .var(&UnderlinePositionVar::new());
    }

    /// Borrow or copy all the text contextual values.
    pub fn get<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> Self {
        let vars = vars.as_ref();

        TextContext {
            lang: LangVar::get(vars),
            font_family: FontFamilyVar::get(vars),
            font_style: *FontStyleVar::get(vars),
            font_weight: *FontWeightVar::get(vars),
            font_stretch: *FontStretchVar::get(vars),

            text_transform: TextTransformVar::get(vars).clone(),
            white_space: *WhiteSpaceVar::get(vars),

            font_size: FontSizeVar::get(vars),
            font_variations: FontVariationsVar::get(vars),

            line_height: LineHeightVar::get(vars),
            letter_spacing: LetterSpacingVar::get(vars),
            word_spacing: WordSpacingVar::get(vars),
            line_spacing: LineSpacingVar::get(vars),
            word_break: *WordBreakVar::get(vars),
            line_break: *LineBreakVar::get(vars),
            tab_length: TabLengthVar::get(vars),
            font_features: FontFeaturesVar::get(vars),

            text_align: *TextAlignVar::get(vars),

            text_color: *TextColorVar::get(vars),

            font_synthesis: *FontSynthesisVar::get(vars),
            font_aa: *FontAaVar::get(vars),

            overline: (OverlineThicknessVar::get(vars), *OverlineStyleVar::get(vars)),
            overline_color: *OverlineColorVar::get(vars),

            strikethrough: (StrikethroughThicknessVar::get(vars), *StrikethroughStyleVar::get(vars)),
            strikethrough_color: *StrikethroughColorVar::get(vars),

            underline: (UnderlineThicknessVar::get(vars), *UnderlineStyleVar::get(vars)),
            underline_color: *UnderlineColorVar::get(vars),
            underline_skip: *UnderlineSkipVar::get(vars),
            underline_position: *UnderlinePositionVar::get(vars),
        }
    }

    /// Gets the properties that affect the font face.
    pub fn font_face<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (&'a Lang, &'a [FontName], FontStyle, FontWeight, FontStretch) {
        let vars = vars.as_ref();
        (
            LangVar::get(vars),
            FontFamilyVar::get(vars),
            *FontStyleVar::get(vars),
            *FontWeightVar::get(vars),
            *FontStretchVar::get(vars),
        )
    }
    /// Gets [`font_face`](Self::font_face) if any of the properties updated.
    pub fn font_face_update<Vw: AsRef<Vars>>(vars: &'a Vw) -> Option<(&'a Lang, &'a [FontName], FontStyle, FontWeight, FontStretch)> {
        let vars = vars.as_ref();
        if LangVar::is_new(vars)
            || FontFamilyVar::is_new(vars)
            || FontStyleVar::is_new(vars)
            || FontWeightVar::is_new(vars)
            || FontStretchVar::is_new(vars)
        {
            Some(Self::font_face(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect the text characters.
    pub fn text<Vr: WithVarsRead>(vars: &Vr) -> (TextTransformFn, WhiteSpace) {
        vars.with_vars_read(|vars| (TextTransformVar::get(vars).clone(), *WhiteSpaceVar::get(vars)))
    }
    /// Gets [`text`](Self::text) if any of the properties updated.
    pub fn text_update<Vw: WithVars>(vars: &Vw) -> Option<(TextTransformFn, WhiteSpace)> {
        vars.with_vars(|vars| {
            if TextTransformVar::is_new(vars) || WhiteSpaceVar::is_new(vars) {
                Some(Self::text(vars))
            } else {
                None
            }
        })
    }

    /// Gets the properties that affect the sized font.
    pub fn font<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (&'a FontSize, &'a FontVariations) {
        let vars = vars.as_ref();
        (FontSizeVar::get(vars), FontVariationsVar::get(vars))
    }
    /// Gets [`font`](Self::font) if any of the properties updated.
    pub fn font_update<Vw: AsRef<Vars>>(vars: &'a Vw) -> Option<(&'a FontSize, &'a FontVariations)> {
        let vars = vars.as_ref();
        if FontSizeVar::is_new(vars) || FontVariationsVar::is_new(vars) {
            Some(Self::font(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect text shaping.
    pub fn shaping<Vr: AsRef<VarsRead>>(
        vars: &'a Vr,
    ) -> (
        &'a LetterSpacing,
        &'a WordSpacing,
        &'a LineSpacing,
        &'a LineHeight,
        &'a TabLength,
        &'a Lang,
    ) {
        let vars = vars.as_ref();
        (
            LetterSpacingVar::get(vars),
            WordSpacingVar::get(vars),
            LineSpacingVar::get(vars),
            LineHeightVar::get(vars),
            TabLengthVar::get(vars),
            LangVar::get(vars),
        )
    }

    /// Gets [`shaping`](Self::shaping) if any of the properties is new.
    pub fn shaping_update<Vw: AsRef<Vars>>(
        vars: &'a Vw,
    ) -> Option<(
        &'a LetterSpacing,
        &'a WordSpacing,
        &'a LineSpacing,
        &'a LineHeight,
        &'a TabLength,
        &'a Lang,
    )> {
        let vars = vars.as_ref();
        if LetterSpacingVar::is_new(vars)
            || WordSpacingVar::is_new(vars)
            || LineSpacingVar::is_new(vars)
            || LineHeightVar::is_new(vars)
            || TabLengthVar::is_new(vars)
            || LangVar::is_new(vars)
        {
            Some(Self::shaping(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect text wrapping only.
    pub fn wrapping<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (WordBreak, LineBreak) {
        (*WordBreakVar::get(vars), *LineBreakVar::get(vars))
    }

    /// Gets [`wrapping`](Self::wrapping) if any of the properties updated.
    pub fn wrapping_update<Vw: WithVars>(vars: &'a Vw) -> Option<(WordBreak, LineBreak)> {
        vars.with_vars(|vars| {
            if WordBreakVar::is_new(vars) || LineBreakVar::is_new(vars) {
                Some((*WordBreakVar::get(vars), *LineBreakVar::get(vars)))
            } else {
                None
            }
        })
    }

    /// Gets the property that affect color.
    pub fn color<Vr: AsRef<VarsRead>>(vars: &Vr) -> Rgba {
        *TextColorVar::get(vars)
    }
    /// Gets [`color`](Self::color) if the property updated.
    pub fn color_update<Vw: WithVars>(vars: &Vw) -> Option<Rgba> {
        vars.with_vars(|vars| TextColorVar::get_new(vars).copied())
    }

    /// Gets the properties that affects what font synthesis is used.
    pub fn font_synthesis<Vr: WithVarsRead>(vars: &Vr) -> (FontSynthesis, FontStyle, FontWeight) {
        vars.with_vars_read(|vars| (*FontSynthesisVar::get(vars), *FontStyleVar::get(vars), *FontWeightVar::get(vars)))
    }

    /// Gets [`font_synthesis`](Self::font_synthesis) if any of the properties changed.
    pub fn font_synthesis_update<Vw: WithVars>(vars: &Vw) -> Option<(FontSynthesis, FontStyle, FontWeight)> {
        vars.with_vars(|vars| {
            if FontSynthesisVar::is_new(vars) || FontStyleVar::is_new(vars) || FontWeightVar::is_new(vars) {
                Some(Self::font_synthesis(vars))
            } else {
                None
            }
        })
    }
}
impl<'a> Clone for TextContext<'a> {
    fn clone(&self) -> Self {
        TextContext {
            text_transform: self.text_transform.clone(),
            font_features: self.font_features,
            lang: self.lang,
            font_family: self.font_family,
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_style: self.font_style,
            font_variations: self.font_variations,
            font_stretch: self.font_stretch,
            font_synthesis: self.font_synthesis,
            font_aa: self.font_aa,
            text_color: self.text_color,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            word_spacing: self.word_spacing,
            line_spacing: self.line_spacing,
            word_break: self.word_break,
            line_break: self.line_break,
            text_align: self.text_align,
            tab_length: self.tab_length,
            white_space: self.white_space,
            overline: self.overline,
            overline_color: self.overline_color,
            strikethrough: self.strikethrough,
            strikethrough_color: self.strikethrough_color,
            underline: self.underline,
            underline_color: self.underline_color,
            underline_skip: self.underline_skip,
            underline_position: self.underline_position,
        }
    }
}
