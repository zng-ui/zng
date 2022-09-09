//! Properties and context variables that configure the appearance of text widgets.

use crate::core::text::{font_features::*, *};
use crate::prelude::new_property::*;

context_var! {
    /// Font family of [`text`](crate::widgets::text) spans.
    pub static FONT_FAMILY_VAR: FontNames = FontNames::default();

    /// Font weight of [`text`](crate::widgets::text) spans.
    pub static FONT_WEIGHT_VAR: FontWeight = FontWeight::NORMAL;

    /// Font style of [`text`](crate::widgets::text) spans.
    pub static FONT_STYLE_VAR: FontStyle = FontStyle::Normal;

    /// Font stretch of [`text`](crate::widgets::text) spans.
    pub static FONT_STRETCH_VAR: FontStretch = FontStretch::NORMAL;

    /// Font synthesis of [`text`](crate::widgets::text) spans.
    pub static FONT_SYNTHESIS_VAR: FontSynthesis = FontSynthesis::ENABLED;

    /// Font anti-aliasing of [`text`](crate::widgets::text) spans.
    pub static FONT_AA_VAR: FontAntiAliasing = FontAntiAliasing::Default;

    /// Font size of [`text`](crate::widgets::text) spans.
    pub static FONT_SIZE_VAR: FontSize = FontSize::Pt(11.0);

    /// Text color of [`text`](crate::widgets::text) spans.
    pub static TEXT_COLOR_VAR: Rgba = colors::WHITE;

    /// Text transformation function applied to [`text`](crate::widgets::text) spans.
    pub static TEXT_TRANSFORM_VAR: TextTransformFn = TextTransformFn::None;

    /// Text line height of [`text`](crate::widgets::text) spans.
    pub static LINE_HEIGHT_VAR: LineHeight = LineHeight::Default;

    /// Extra spacing in between lines of [`text`](crate::widgets::text) spans.
    pub static LINE_SPACING_VAR: Length = Px(0);

    /// Extra letter spacing of [`text`](crate::widgets::text) spans.
    pub static LETTER_SPACING_VAR: LetterSpacing = LetterSpacing::Default;

    /// Extra word spacing of [`text`](crate::widgets::text) spans.
    pub static WORD_SPACING_VAR: WordSpacing = WordSpacing::Default;

    /// Extra paragraph spacing of text blocks.
    pub static PARAGRAPH_SPACING_VAR: ParagraphSpacing = Length::Px(Px(0));

    /// Configuration of line breaks inside words during text wrap.
    pub static WORD_BREAK_VAR: WordBreak = WordBreak::Normal;

    /// Configuration of line breaks in Chinese, Japanese, or Korean text.
    pub static LINE_BREAK_VAR: LineBreak = LineBreak::Auto;

    /// Text line alignment in a text block.
    pub static TEXT_ALIGN_VAR: TextAlign = TextAlign::START;

    /// Length of the `TAB` space.
    pub static TAB_LENGTH_VAR: TabLength = 400.pct();

    /// Text white space transform of [`text`](crate::widgets::text) spans.
    pub static WHITE_SPACE_VAR: WhiteSpace = WhiteSpace::Preserve;

    /// Font features of [`text`](crate::widgets::text) spans.
    pub static FONT_FEATURES_VAR: FontFeatures = FontFeatures::new();

    /// Font variations of [`text`](crate::widgets::text) spans.
    pub static FONT_VARIATIONS_VAR: FontVariations = FontVariations::new();

    /// Language of [`text`](crate::widgets::text) spans.
    pub static LANG_VAR: Lang = Lang::default();

    /// Underline thickness.
    pub static UNDERLINE_THICKNESS_VAR: UnderlineThickness = 0;
    /// Underline style.
    pub static UNDERLINE_STYLE_VAR: LineStyle = LineStyle::Hidden;
    /// Underline color.
    pub static UNDERLINE_COLOR_VAR: TextLineColor = TextLineColor::Text;
    /// Parts of text skipped by underline.
    pub static UNDERLINE_SKIP_VAR: UnderlineSkip = UnderlineSkip::DEFAULT;
    /// Position of the underline.
    pub static UNDERLINE_POSITION_VAR: UnderlinePosition = UnderlinePosition::Font;

    /// Overline thickness.
    pub static OVERLINE_THICKNESS_VAR: TextLineThickness = 0;
    /// Overline style.
    pub static OVERLINE_STYLE_VAR: LineStyle = LineStyle::Hidden;
    /// Overline color.
    pub static OVERLINE_COLOR_VAR: TextLineColor = TextLineColor::Text;

    /// Strikethrough thickness.
    pub static STRIKETHROUGH_THICKNESS_VAR: TextLineThickness = 0;
    /// Strikethrough style.
    pub static  STRIKETHROUGH_STYLE_VAR: LineStyle = LineStyle::Hidden;
    /// Strikethrough color.
    pub static STRIKETHROUGH_COLOR_VAR: TextLineColor = TextLineColor::Text;

    /// Caret color.
    pub static CARET_COLOR_VAR: TextLineColor = TextLineColor::Text;

    /// Text is editable.
    pub static TEXT_EDITABLE_VAR: bool = false;

    /// Text padding.
    pub static TEXT_PADDING_VAR: SideOffsets = 0;
}

/// Sets the [`FONT_FAMILY_VAR`] context var.
#[property(context, default(FONT_FAMILY_VAR))]
pub fn font_family(child: impl UiNode, names: impl IntoVar<FontNames>) -> impl UiNode {
    with_context_var(child, FONT_FAMILY_VAR, names)
}

/// Sets the [`FONT_STYLE_VAR`] context var.
#[property(context, default(FONT_STYLE_VAR))]
pub fn font_style(child: impl UiNode, style: impl IntoVar<FontStyle>) -> impl UiNode {
    with_context_var(child, FONT_STYLE_VAR, style)
}

/// Sets the [`FONT_WEIGHT_VAR`] context var.
#[property(context, default(FONT_WEIGHT_VAR))]
pub fn font_weight(child: impl UiNode, weight: impl IntoVar<FontWeight>) -> impl UiNode {
    with_context_var(child, FONT_WEIGHT_VAR, weight)
}

/// Sets the [`FONT_STRETCH_VAR`] context var.
#[property(context, default(FONT_STRETCH_VAR))]
pub fn font_stretch(child: impl UiNode, stretch: impl IntoVar<FontStretch>) -> impl UiNode {
    with_context_var(child, FONT_STRETCH_VAR, stretch)
}

/// Sets the [`FONT_SYNTHESIS_VAR`] context var.
#[property(context, default(FONT_SYNTHESIS_VAR))]
pub fn font_synthesis(child: impl UiNode, enabled: impl IntoVar<FontSynthesis>) -> impl UiNode {
    with_context_var(child, FONT_SYNTHESIS_VAR, enabled)
}

/// Sets the [`FONT_AA_VAR`] context var.
#[property(context, default(FONT_AA_VAR))]
pub fn font_aa(child: impl UiNode, aa: impl IntoVar<FontAntiAliasing>) -> impl UiNode {
    with_context_var(child, FONT_AA_VAR, aa)
}

/// Sets the [`FONT_SIZE_VAR`] context var and the [`LayoutMetrics::font_size`].
#[property(context, default(FONT_SIZE_VAR))]
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
            subs.var(ctx, &FONT_SIZE_VAR);
            self.child.subscriptions(ctx, subs);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if FONT_SIZE_VAR.is_new(ctx) {
                ctx.updates.layout();
            }
            self.child.update(ctx);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let font_size = FONT_SIZE_VAR.get(ctx.vars).layout(ctx.for_y(), |ctx| ctx.metrics.root_font_size());
            ctx.with_font_size(font_size, |ctx| self.child.measure(ctx))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let font_size = FONT_SIZE_VAR.get(ctx.vars).layout(ctx.for_y(), |ctx| ctx.metrics.root_font_size());
            ctx.with_font_size(font_size, |ctx| self.child.layout(ctx, wl))
        }
    }
    let child = FontSizeNode { child };
    with_context_var(child, FONT_SIZE_VAR, size)
}

/// Sets the [`TEXT_COLOR_VAR`] context var.
#[property(context, default(TEXT_COLOR_VAR))]
pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, TEXT_COLOR_VAR, color)
}

/// Sets the [`TEXT_TRANSFORM_VAR`] context var.
#[property(context, default(TEXT_TRANSFORM_VAR))]
pub fn text_transform(child: impl UiNode, transform: impl IntoVar<TextTransformFn>) -> impl UiNode {
    with_context_var(child, TEXT_TRANSFORM_VAR, transform)
}

/// Sets the [`LINE_HEIGHT_VAR`] context var.
#[property(context, default(LINE_HEIGHT_VAR))]
pub fn line_height(child: impl UiNode, height: impl IntoVar<LineHeight>) -> impl UiNode {
    with_context_var(child, LINE_HEIGHT_VAR, height)
}

/// Sets the [`LETTER_SPACING_VAR`] context var.
#[property(context, default(LETTER_SPACING_VAR))]
pub fn letter_spacing(child: impl UiNode, extra: impl IntoVar<LetterSpacing>) -> impl UiNode {
    with_context_var(child, LETTER_SPACING_VAR, extra)
}

/// Sets the [`LINE_SPACING_VAR`] context var.
#[property(context, default(LINE_SPACING_VAR))]
pub fn line_spacing(child: impl UiNode, extra: impl IntoVar<LineSpacing>) -> impl UiNode {
    with_context_var(child, LINE_SPACING_VAR, extra)
}

/// Sets the [`WORD_SPACING_VAR`] context var.
#[property(context, default(WORD_SPACING_VAR))]
pub fn word_spacing(child: impl UiNode, extra: impl IntoVar<WordSpacing>) -> impl UiNode {
    with_context_var(child, WORD_SPACING_VAR, extra)
}

/// Sets the [`PARAGRAPH_SPACING_VAR`] context var.
#[property(context, default(PARAGRAPH_SPACING_VAR))]
pub fn paragraph_spacing(child: impl UiNode, extra: impl IntoVar<ParagraphSpacing>) -> impl UiNode {
    with_context_var(child, PARAGRAPH_SPACING_VAR, extra)
}

/// Sets the [`WORD_BREAK_VAR`] context var.
#[property(context, default(WORD_BREAK_VAR))]
pub fn word_break(child: impl UiNode, mode: impl IntoVar<WordBreak>) -> impl UiNode {
    with_context_var(child, WORD_BREAK_VAR, mode)
}

/// Sets the [`LINE_BREAK_VAR`] context var.
#[property(context, default(LINE_BREAK_VAR))]
pub fn line_break(child: impl UiNode, mode: impl IntoVar<LineBreak>) -> impl UiNode {
    with_context_var(child, LINE_BREAK_VAR, mode)
}

/// Sets the [`TEXT_ALIGN_VAR`] context var.
#[property(context, default(TEXT_ALIGN_VAR))]
pub fn text_align(child: impl UiNode, mode: impl IntoVar<TextAlign>) -> impl UiNode {
    with_context_var(child, TEXT_ALIGN_VAR, mode)
}

/// Sets the [`TAB_LENGTH_VAR`] context var.
#[property(context, default(TAB_LENGTH_VAR))]
pub fn tab_length(child: impl UiNode, length: impl IntoVar<TabLength>) -> impl UiNode {
    with_context_var(child, TAB_LENGTH_VAR, length)
}

/// Sets the [`WHITE_SPACE_VAR`] context var.
#[property(context, default(WHITE_SPACE_VAR))]
pub fn white_space(child: impl UiNode, transform: impl IntoVar<WhiteSpace>) -> impl UiNode {
    with_context_var(child, WHITE_SPACE_VAR, transform)
}

/// Includes the font variation config in the widget context.
///
/// The variation `name` is set for the [`FONT_VARIATIONS_VAR`] in this context, variations already set in the parent
/// context that are not the same `name` are also included.
pub fn with_font_variation(child: impl UiNode, name: FontVariationName, value: impl IntoVar<f32>) -> impl UiNode {
    with_context_var(
        child,
        FONT_VARIATIONS_VAR,
        merge_var!(FONT_VARIATIONS_VAR, value.into_var(), move |variations, value| {
            let mut variations = variations.clone();
            variations.insert(name, *value);
            variations
        }),
    )
}

/// Include the font feature config in the widget context.
///
/// The modifications done in `set_feature` are visible only in the [`FONT_FEATURES_VAR`] in this context, and features
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
        FONT_FEATURES_VAR,
        merge_var!(FONT_FEATURES_VAR, state.into_var(), move |features, state| {
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
    with_context_var(child, FONT_VARIATIONS_VAR, variations)
}

/// Sets font features.
///
/// **Note:** This property fully replaces the font variations for the widget and descendants, use [`with_font_variation`]
/// to create a property that sets a variation but retains others from the context.
#[property(context)]
pub fn font_features(child: impl UiNode, features: impl IntoVar<FontFeatures>) -> impl UiNode {
    with_context_var(child, FONT_FEATURES_VAR, features)
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

/// Sets the [`LANG_VAR`] context var.
#[property(context, default(LANG_VAR))]
pub fn lang(child: impl UiNode, lang: impl IntoVar<Lang>) -> impl UiNode {
    with_context_var(child, LANG_VAR, lang)
}

/// Sets the [`UNDERLINE_THICKNESS_VAR`] and [`UNDERLINE_STYLE_VAR`].
#[property(context, default(UNDERLINE_THICKNESS_VAR, UNDERLINE_STYLE_VAR))]
pub fn underline(child: impl UiNode, thickness: impl IntoVar<UnderlineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, UNDERLINE_THICKNESS_VAR, thickness);
    with_context_var(child, UNDERLINE_STYLE_VAR, style)
}
/// Sets the [`UNDERLINE_COLOR_VAR`].
#[property(context, default(UNDERLINE_COLOR_VAR))]
pub fn underline_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, UNDERLINE_COLOR_VAR, color)
}
/// Sets the [`UNDERLINE_SKIP_VAR`].
#[property(context, default(UNDERLINE_SKIP_VAR))]
pub fn underline_skip(child: impl UiNode, skip: impl IntoVar<UnderlineSkip>) -> impl UiNode {
    with_context_var(child, UNDERLINE_SKIP_VAR, skip)
}
/// Sets the [`UNDERLINE_POSITION_VAR`].
#[property(context, default(UNDERLINE_POSITION_VAR))]
pub fn underline_position(child: impl UiNode, position: impl IntoVar<UnderlinePosition>) -> impl UiNode {
    with_context_var(child, UNDERLINE_POSITION_VAR, position)
}

/// Sets the [`OVERLINE_THICKNESS_VAR`] and [`OVERLINE_STYLE_VAR`].
#[property(context, default(OVERLINE_THICKNESS_VAR, OVERLINE_STYLE_VAR))]
pub fn overline(child: impl UiNode, thickness: impl IntoVar<TextLineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, OVERLINE_THICKNESS_VAR, thickness);
    with_context_var(child, OVERLINE_STYLE_VAR, style)
}
/// Sets the [`OVERLINE_COLOR_VAR`].
#[property(context, default(OVERLINE_COLOR_VAR))]
pub fn overline_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, OVERLINE_COLOR_VAR, color)
}

/// Sets the [`STRIKETHROUGH_THICKNESS_VAR`] and [`STRIKETHROUGH_STYLE_VAR`].
#[property(context, default(STRIKETHROUGH_THICKNESS_VAR, STRIKETHROUGH_STYLE_VAR))]
pub fn strikethrough(child: impl UiNode, thickness: impl IntoVar<TextLineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, STRIKETHROUGH_THICKNESS_VAR, thickness);
    with_context_var(child, STRIKETHROUGH_STYLE_VAR, style)
}
/// Sets the [`STRIKETHROUGH_COLOR_VAR`].
#[property(context, default(STRIKETHROUGH_COLOR_VAR))]
pub fn strikethrough_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, STRIKETHROUGH_COLOR_VAR, color)
}

/// Sets the [`CARET_COLOR_VAR`].
#[property(context, default(CARET_COLOR_VAR))]
pub fn caret_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
    with_context_var(child, CARET_COLOR_VAR, color)
}

/// Sets the [`TEXT_EDITABLE_VAR`].
#[property(context, default(TEXT_EDITABLE_VAR))]
pub fn text_editable(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, TEXT_EDITABLE_VAR, enabled)
}

/// Sets the [`TEXT_PADDING_VAR`] that is used in the text-input layout.
#[property(context, default(TEXT_PADDING_VAR))]
pub fn text_padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
    with_context_var(child, TEXT_PADDING_VAR, padding)
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

    /// The [`caret_color`](fn@caret_color) value.
    pub caret_color: TextLineColor,
}
impl<'a> TextContext<'a> {
    /// Register all text context variables in the widget.
    pub fn subscribe(vars: &VarsRead, widget: &mut WidgetSubscriptions) {
        widget
            .vars(vars)
            .var(&LANG_VAR)
            .var(&FONT_FAMILY_VAR)
            .var(&FONT_STYLE_VAR)
            .var(&FONT_WEIGHT_VAR)
            .var(&FONT_STRETCH_VAR)
            .var(&TEXT_TRANSFORM_VAR)
            .var(&WHITE_SPACE_VAR)
            .var(&FONT_SIZE_VAR)
            .var(&FONT_VARIATIONS_VAR)
            .var(&LINE_HEIGHT_VAR)
            .var(&LETTER_SPACING_VAR)
            .var(&WORD_SPACING_VAR)
            .var(&LINE_SPACING_VAR)
            .var(&WORD_BREAK_VAR)
            .var(&LINE_BREAK_VAR)
            .var(&TAB_LENGTH_VAR)
            .var(&FONT_FEATURES_VAR)
            .var(&TEXT_ALIGN_VAR)
            .var(&TEXT_COLOR_VAR)
            .var(&FONT_SYNTHESIS_VAR)
            .var(&FONT_AA_VAR)
            .var(&OVERLINE_THICKNESS_VAR)
            .var(&OVERLINE_STYLE_VAR)
            .var(&OVERLINE_COLOR_VAR)
            .var(&STRIKETHROUGH_THICKNESS_VAR)
            .var(&STRIKETHROUGH_STYLE_VAR)
            .var(&STRIKETHROUGH_COLOR_VAR)
            .var(&UNDERLINE_THICKNESS_VAR)
            .var(&UNDERLINE_COLOR_VAR)
            .var(&UNDERLINE_SKIP_VAR)
            .var(&UNDERLINE_POSITION_VAR)
            .var(&CARET_COLOR_VAR);
    }

    /// Borrow or copy all the text contextual values.
    pub fn get<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> Self {
        let vars = vars.as_ref();

        TextContext {
            lang: LANG_VAR.get(vars),
            font_family: FONT_FAMILY_VAR.get(vars),
            font_style: FONT_STYLE_VAR.copy(vars),
            font_weight: FONT_WEIGHT_VAR.copy(vars),
            font_stretch: FONT_STRETCH_VAR.copy(vars),

            text_transform: TEXT_TRANSFORM_VAR.get_clone(vars),
            white_space: WHITE_SPACE_VAR.copy(vars),

            font_size: FONT_SIZE_VAR.get(vars),
            font_variations: FONT_VARIATIONS_VAR.get(vars),

            line_height: LINE_HEIGHT_VAR.get(vars),
            letter_spacing: LETTER_SPACING_VAR.get(vars),
            word_spacing: WORD_SPACING_VAR.get(vars),
            line_spacing: LINE_SPACING_VAR.get(vars),
            word_break: WORD_BREAK_VAR.copy(vars),
            line_break: LINE_BREAK_VAR.copy(vars),
            tab_length: TAB_LENGTH_VAR.get(vars),
            font_features: FONT_FEATURES_VAR.get(vars),

            text_align: TEXT_ALIGN_VAR.copy(vars),

            text_color: TEXT_COLOR_VAR.copy(vars),

            font_synthesis: FONT_SYNTHESIS_VAR.copy(vars),
            font_aa: FONT_AA_VAR.copy(vars),

            overline: (OVERLINE_THICKNESS_VAR.get(vars), OVERLINE_STYLE_VAR.copy(vars)),
            overline_color: OVERLINE_COLOR_VAR.copy(vars),

            strikethrough: (STRIKETHROUGH_THICKNESS_VAR.get(vars), STRIKETHROUGH_STYLE_VAR.copy(vars)),
            strikethrough_color: STRIKETHROUGH_COLOR_VAR.copy(vars),

            underline: (UNDERLINE_THICKNESS_VAR.get(vars), UNDERLINE_STYLE_VAR.copy(vars)),
            underline_color: UNDERLINE_COLOR_VAR.copy(vars),
            underline_skip: UNDERLINE_SKIP_VAR.copy(vars),
            underline_position: UNDERLINE_POSITION_VAR.copy(vars),

            caret_color: CARET_COLOR_VAR.copy(vars),
        }
    }

    /// Gets the properties that affect the font face.
    pub fn font_face<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (&'a Lang, &'a [FontName], FontStyle, FontWeight, FontStretch) {
        let vars = vars.as_ref();
        (
            LANG_VAR.get(vars),
            FONT_FAMILY_VAR.get(vars),
            FONT_STYLE_VAR.copy(vars),
            FONT_WEIGHT_VAR.copy(vars),
            FONT_STRETCH_VAR.copy(vars),
        )
    }
    /// Gets [`font_face`](Self::font_face) if any of the properties updated.
    pub fn font_face_update<Vw: AsRef<Vars>>(vars: &'a Vw) -> Option<(&'a Lang, &'a [FontName], FontStyle, FontWeight, FontStretch)> {
        let vars = vars.as_ref();
        if LANG_VAR.is_new(vars)
            || FONT_FAMILY_VAR.is_new(vars)
            || FONT_STYLE_VAR.is_new(vars)
            || FONT_WEIGHT_VAR.is_new(vars)
            || FONT_STRETCH_VAR.is_new(vars)
        {
            Some(Self::font_face(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect the text characters.
    pub fn text<Vr: WithVarsRead>(vars: &Vr) -> (TextTransformFn, WhiteSpace) {
        vars.with_vars_read(|vars| (TEXT_TRANSFORM_VAR.get_clone(vars), WHITE_SPACE_VAR.copy(vars)))
    }
    /// Gets [`text`](Self::text) if any of the properties updated.
    pub fn text_update<Vw: WithVars>(vars: &Vw) -> Option<(TextTransformFn, WhiteSpace)> {
        vars.with_vars(|vars| {
            if TEXT_TRANSFORM_VAR.is_new(vars) || WHITE_SPACE_VAR.is_new(vars) {
                Some(Self::text(vars))
            } else {
                None
            }
        })
    }

    /// Gets the properties that affect the sized font.
    pub fn font<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (&'a FontSize, &'a FontVariations) {
        let vars = vars.as_ref();
        (FONT_SIZE_VAR.get(vars), FONT_VARIATIONS_VAR.get(vars))
    }
    /// Gets [`font`](Self::font) if any of the properties updated.
    pub fn font_update<Vw: AsRef<Vars>>(vars: &'a Vw) -> Option<(&'a FontSize, &'a FontVariations)> {
        let vars = vars.as_ref();
        if FONT_SIZE_VAR.is_new(vars) || FONT_VARIATIONS_VAR.is_new(vars) {
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
            LETTER_SPACING_VAR.get(vars),
            WORD_SPACING_VAR.get(vars),
            LINE_SPACING_VAR.get(vars),
            LINE_HEIGHT_VAR.get(vars),
            TAB_LENGTH_VAR.get(vars),
            LANG_VAR.get(vars),
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
        if LETTER_SPACING_VAR.is_new(vars)
            || WORD_SPACING_VAR.is_new(vars)
            || LINE_SPACING_VAR.is_new(vars)
            || LINE_HEIGHT_VAR.is_new(vars)
            || TAB_LENGTH_VAR.is_new(vars)
            || LANG_VAR.is_new(vars)
        {
            Some(Self::shaping(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect text wrapping only.
    pub fn wrapping<Vr: WithVarsRead>(vars: &'a Vr) -> (WordBreak, LineBreak) {
        vars.with_vars_read(|vars| (WORD_BREAK_VAR.copy(vars), LINE_BREAK_VAR.copy(vars)))
    }

    /// Gets [`wrapping`](Self::wrapping) if any of the properties updated.
    pub fn wrapping_update<Vw: WithVars>(vars: &'a Vw) -> Option<(WordBreak, LineBreak)> {
        vars.with_vars(|vars| {
            if WORD_BREAK_VAR.is_new(vars) || LINE_BREAK_VAR.is_new(vars) {
                Some((WORD_BREAK_VAR.copy(vars), LINE_BREAK_VAR.copy(vars)))
            } else {
                None
            }
        })
    }

    /// Gets the property that affect color.
    pub fn color<Vr: WithVarsRead>(vars: &Vr) -> Rgba {
        TEXT_COLOR_VAR.copy(vars)
    }
    /// Gets [`color`](Self::color) if the property updated.
    pub fn color_update<Vw: WithVars>(vars: &Vw) -> Option<Rgba> {
        TEXT_COLOR_VAR.copy_new(vars)
    }

    /// Gets the properties that affects what font synthesis is used.
    pub fn font_synthesis<Vr: WithVarsRead>(vars: &Vr) -> (FontSynthesis, FontStyle, FontWeight) {
        vars.with_vars_read(|vars| (FONT_SYNTHESIS_VAR.copy(vars), FONT_STYLE_VAR.copy(vars), FONT_WEIGHT_VAR.copy(vars)))
    }

    /// Gets [`font_synthesis`](Self::font_synthesis) if any of the properties changed.
    pub fn font_synthesis_update<Vw: WithVars>(vars: &Vw) -> Option<(FontSynthesis, FontStyle, FontWeight)> {
        vars.with_vars(|vars| {
            if FONT_SYNTHESIS_VAR.is_new(vars) || FONT_STYLE_VAR.is_new(vars) || FONT_WEIGHT_VAR.is_new(vars) {
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
            caret_color: self.caret_color,
        }
    }
}
