use crate::core::text::{font_features::*, *};
use crate::prelude::new_property::*;

use super::Text;

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
    pub static LINE_SPACING_VAR: LineSpacing = LineSpacing::Default;

    /// Extra letter spacing of [`text`](crate::widgets::text) spans.
    pub static LETTER_SPACING_VAR: LetterSpacing = LetterSpacing::Default;

    /// Extra word spacing of [`text`](crate::widgets::text) spans.
    pub static WORD_SPACING_VAR: WordSpacing = WordSpacing::Default;

    /// Extra paragraph spacing of text blocks.
    pub static PARAGRAPH_SPACING_VAR: ParagraphSpacing = 1.em();

    /// Configuration of line breaks inside words during text wrap.
    pub static WORD_BREAK_VAR: WordBreak = WordBreak::Normal;

    /// Configuration of line breaks in Chinese, Japanese, or Korean text.
    pub static LINE_BREAK_VAR: LineBreak = LineBreak::Auto;

    /// Text line alignment inside a text block.
    pub static TEXT_ALIGN_VAR: Align = Align::START;

    /// Text justify mode when text align is fill.
    pub static JUSTIFY_VAR: Option<Justify> = None;

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

    /// Flow direction of [`text`](crate::widgets::text) spans.
    pub static DIRECTION_VAR: LayoutDirection = LayoutDirection::default();

    /// Underline thickness.
    pub static UNDERLINE_THICKNESS_VAR: UnderlineThickness = 0;
    /// Underline style.
    pub static UNDERLINE_STYLE_VAR: LineStyle = LineStyle::Hidden;
    /// Underline color, inherits from [`TEXT_COLOR_VAR`].
    pub static UNDERLINE_COLOR_VAR: Rgba = TEXT_COLOR_VAR;
    /// Parts of text skipped by underline.
    pub static UNDERLINE_SKIP_VAR: UnderlineSkip = UnderlineSkip::DEFAULT;
    /// Position of the underline.
    pub static UNDERLINE_POSITION_VAR: UnderlinePosition = UnderlinePosition::Font;

    /// Overline thickness.
    pub static OVERLINE_THICKNESS_VAR: TextLineThickness = 0;
    /// Overline style.
    pub static OVERLINE_STYLE_VAR: LineStyle = LineStyle::Hidden;
    /// Overline color, inherits from [`TEXT_COLOR_VAR`].
    pub static OVERLINE_COLOR_VAR: Rgba = TEXT_COLOR_VAR;

    /// Strikethrough thickness.
    pub static STRIKETHROUGH_THICKNESS_VAR: TextLineThickness = 0;
    /// Strikethrough style.
    pub static  STRIKETHROUGH_STYLE_VAR: LineStyle = LineStyle::Hidden;
    /// Strikethrough color, inherits from [`TEXT_COLOR_VAR`].
    pub static STRIKETHROUGH_COLOR_VAR: Rgba = TEXT_COLOR_VAR;

    /// Caret color, inherits from [`TEXT_COLOR_VAR`].
    pub static CARET_COLOR_VAR: Rgba = TEXT_COLOR_VAR;

    /// Text is editable.
    pub static TEXT_EDITABLE_VAR: bool = false;

    /// If line breaks are automatically inserted to fill the available space.
    ///
    /// The [`LINE_BREAK_VAR`], [`WORD_BREAK_VAR`] and [`HYPHENS_VAR`] configure how the text is split.
    ///
    /// Is `true` by default.
    pub static TEXT_WRAP_VAR: bool = true;

    /// Text hyphenation config.
    pub static HYPHENS_VAR: Hyphens = Hyphens::default();

    /// Hyphen text rendered when auto-hyphenating.
    pub static HYPHEN_CHAR_VAR: Txt = Txt::from_char('-');
}

/// Font family name or list of names for texts in this widget or descendants.
///
/// All fonts in the list are resolved according to the [`font_style`], [`font_weight`] and [`font_stretch`] config.
/// During text shaping the first font on the list is preferred, but if the font does not cover a character or word, that
/// character or word falls-back to the second font in the list and so on.
///
/// Sets the [`FONT_FAMILY_VAR`] context var.
///
/// [`font_style`]: fn@font_style
/// [`font_weight`]: fn@font_weight
/// [`font_stretch`]: fn@font_stretch
#[property(CONTEXT, default(FONT_FAMILY_VAR), impl(Text))]
pub fn font_family(child: impl UiNode, names: impl IntoVar<FontNames>) -> impl UiNode {
    with_context_var(child, FONT_FAMILY_VAR, names)
}

/// Defines the degree of blackness or stroke thickness of the font glyphs.
///
/// This value influences font resolution, the variant within the font family that is closest to this config will be selected.
///
/// Sets the [`FONT_STYLE_VAR`] context var.
#[property(CONTEXT, default(FONT_STYLE_VAR), impl(Text))]
pub fn font_style(child: impl UiNode, style: impl IntoVar<FontStyle>) -> impl UiNode {
    with_context_var(child, FONT_STYLE_VAR, style)
}

/// Defines how condensed or expanded the preferred font should be.
///
/// This value influences font resolution, the variant within the font family that is closest to this config will be selected.
///
/// Sets the [`FONT_WEIGHT_VAR`] context var.
#[property(CONTEXT, default(FONT_WEIGHT_VAR), impl(Text))]
pub fn font_weight(child: impl UiNode, weight: impl IntoVar<FontWeight>) -> impl UiNode {
    with_context_var(child, FONT_WEIGHT_VAR, weight)
}

/// Defines how condensed or expanded the preferred font should be.
///
/// This value influences font resolution, the variant within the font family that is closest to this config will be selected.
///
/// Sets the [`FONT_STRETCH_VAR`] context var.
#[property(CONTEXT, default(FONT_STRETCH_VAR), impl(Text))]
pub fn font_stretch(child: impl UiNode, stretch: impl IntoVar<FontStretch>) -> impl UiNode {
    with_context_var(child, FONT_STRETCH_VAR, stretch)
}

/// Configure if a synthetic font is generated for fonts that do not implement **bold** or *oblique* variants.
///
/// Not all fonts implement the requested [`font_weight`] and [`font_style`], this config allows the renderer
/// to try and generate the style and weight anyway, using transforms and the glyph outlines.
///
/// Sets the [`FONT_SYNTHESIS_VAR`] context var.
///
/// [`font_weight`]: fn@font_weight
/// [`font_style`]: fn@font_style
#[property(CONTEXT, default(FONT_SYNTHESIS_VAR), impl(Text))]
pub fn font_synthesis(child: impl UiNode, enabled: impl IntoVar<FontSynthesis>) -> impl UiNode {
    with_context_var(child, FONT_SYNTHESIS_VAR, enabled)
}

/// Configure the anti-aliasing used to render text glyphs inside the widget.
///
/// Uses the operating system configuration by default.
///
/// Sets the [`FONT_AA_VAR`] context var.
#[property(CONTEXT, default(FONT_AA_VAR), impl(Text))]
pub fn font_aa(child: impl UiNode, aa: impl IntoVar<FontAntiAliasing>) -> impl UiNode {
    with_context_var(child, FONT_AA_VAR, aa)
}

/// Sets the font size for the widget and descendants.
///
/// This property affects all texts inside the widget and the [`Length::Em`] unit.
///
/// Sets the [`FONT_SIZE_VAR`] context var and the [`LayoutMetrics::font_size`].
#[property(CONTEXT, default(FONT_SIZE_VAR), impl(Text))]
pub fn font_size(child: impl UiNode, size: impl IntoVar<FontSize>) -> impl UiNode {
    #[ui_node(struct FontSizeNode {
        child: impl UiNode,
    })]
    impl UiNode for FontSizeNode {
        fn init(&mut self) {
            WIDGET.sub_var(&FONT_SIZE_VAR);
            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if FONT_SIZE_VAR.is_new() {
                WIDGET.layout();
            }
            self.child.update(updates);
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            let font_size = FONT_SIZE_VAR.get();
            let font_size_px = font_size.layout_dft_x(LAYOUT.root_font_size());
            if font_size_px >= Px(0) {
                LAYOUT.with_font_size(font_size_px, || self.child.measure(wm))
            } else {
                tracing::error!("invalid font size {font_size:?} => {font_size_px:?}");
                self.child.measure(wm)
            }
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let font_size = FONT_SIZE_VAR.get();
            let font_size_px = font_size.layout_dft_x(LAYOUT.root_font_size());
            if font_size_px >= Px(0) {
                LAYOUT.with_font_size(font_size_px, || self.child.layout(wl))
            } else {
                tracing::error!("invalid font size {font_size:?} => {font_size_px:?}");
                self.child.layout(wl)
            }
        }
    }
    let child = FontSizeNode { child };
    with_context_var(child, FONT_SIZE_VAR, size)
}

/// Sets the [`TEXT_COLOR_VAR`] context var.
#[property(CONTEXT, default(TEXT_COLOR_VAR), impl(Text))]
pub fn txt_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, TEXT_COLOR_VAR, color)
}

/// Sets the [`TEXT_TRANSFORM_VAR`] context var.
#[property(CONTEXT, default(TEXT_TRANSFORM_VAR), impl(Text))]
pub fn txt_transform(child: impl UiNode, transform: impl IntoVar<TextTransformFn>) -> impl UiNode {
    with_context_var(child, TEXT_TRANSFORM_VAR, transform)
}

/// Height of each text line. If not set inherits the `line_height` from the parent widget.
///
/// The [`Default`] value is computed from the font metrics, `ascent - descent + line_gap`, this is
/// usually similar to `1.2.em()`. Relative values are computed from the default value, so `200.pct()` is double
/// the default line height.
///
/// The text is vertically centralized inside the height.
///
/// [`Default`]: Length::Default
///
/// Sets the [`LINE_HEIGHT_VAR`] context var.
#[property(CONTEXT, default(LINE_HEIGHT_VAR), impl(Text))]
pub fn line_height(child: impl UiNode, height: impl IntoVar<LineHeight>) -> impl UiNode {
    with_context_var(child, LINE_HEIGHT_VAR, height)
}

/// Extra spacing added in between text letters. If not set inherits the `letter_spacing` from the parent widget.
///
/// Letter spacing is computed using the font data, this unit represents
/// extra space added to the computed spacing.
///
/// A "letter" is a character glyph cluster, e.g.: `a`, `â`, `1`, `-`, `漢`.
///
/// The [`Default`] value signals that letter spacing can be tweaked when text *justification* is enabled, all other
/// values disable automatic adjustments for justification inside words.
///
/// Relative values are computed from the length of the space `' '` character.
///
/// [`Default`]: Length::Default
///
/// This property sets the [`LETTER_SPACING_VAR`] context var that affects all inner texts.
#[property(CONTEXT, default(LETTER_SPACING_VAR), impl(Text))]
pub fn letter_spacing(child: impl UiNode, extra: impl IntoVar<LetterSpacing>) -> impl UiNode {
    with_context_var(child, LETTER_SPACING_VAR, extra)
}

/// Extra spacing in-between text lines. If not set inherits the `line_spacing` from the parent widget.
///
/// The [`Default`] value is zero. Relative values are calculated from the [`LineHeight`], so `50.pct()` is half
/// the computed line height. If the text only has one line this property is not used.
///
/// [`Default`]: Length::Default
///
/// Sets the [`LINE_SPACING_VAR`] context var.
#[property(CONTEXT, default(LINE_SPACING_VAR), impl(Text))]
pub fn line_spacing(child: impl UiNode, extra: impl IntoVar<LineSpacing>) -> impl UiNode {
    with_context_var(child, LINE_SPACING_VAR, extra)
}

/// Extra spacing added to the Unicode `U+0020 SPACE` character. If not set inherits the `letter_spacing` from the parent widget.
///
/// Word spacing is done using the space character "advance" as defined in the font,
/// this unit represents extra spacing added to that default spacing.
///
/// A "word" is the sequence of characters in-between space characters. This extra
/// spacing is applied per space character not per word, if there are three spaces between words
/// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
/// see [`WhiteSpace`](crate::core::text::WhiteSpace).
///
/// The [`Default`] value signals that word spacing can be tweaked when text *justification* is enabled, all other
/// values disable automatic adjustments for justification. Relative values are computed from the length of the space `' '` character,
/// so a word spacing of `100.pct()` visually adds *another* space in between words.
///
/// [`Default`]: Length::Default
///
/// This property sets the [`WORD_SPACING_VAR`] context var that affects all inner widgets.
#[property(CONTEXT, default(WORD_SPACING_VAR), impl(Text))]
pub fn word_spacing(child: impl UiNode, extra: impl IntoVar<WordSpacing>) -> impl UiNode {
    with_context_var(child, WORD_SPACING_VAR, extra)
}

/// Extra spacing in-between paragraphs.
///
/// The default value is `1.em()`. Note that the [`text!`] widget does not implement this property, as raw text does not encode
/// paragraph breaks, this property and context var exists to configure *rich-text* widgets, like the [`markdown!`] widget.
///
/// Sets the [`PARAGRAPH_SPACING_VAR`] context var.
///
/// [`text!`]: mod@crate::widgets::text
/// [`markdown!`]: mod@crate::widgets::markdown
#[property(CONTEXT, default(PARAGRAPH_SPACING_VAR), impl(Text))]
pub fn paragraph_spacing(child: impl UiNode, extra: impl IntoVar<ParagraphSpacing>) -> impl UiNode {
    with_context_var(child, PARAGRAPH_SPACING_VAR, extra)
}

/// Configure line breaks inside words during text wrap.
///
/// This value is only considered if it is impossible to fit a full word to a line.
///
/// Hyphens can be inserted in word breaks using the [`hyphens`] configuration.
///
/// Sets the [`WORD_BREAK_VAR`] context var.
///
/// [`hyphens`]: fn@hyphens
#[property(CONTEXT, default(WORD_BREAK_VAR), impl(Text))]
pub fn word_break(child: impl UiNode, mode: impl IntoVar<WordBreak>) -> impl UiNode {
    with_context_var(child, WORD_BREAK_VAR, mode)
}

/// Configuration of text wrapping for Chinese, Japanese, or Korean text.
///
/// Sets the [`LINE_BREAK_VAR`] context var.
#[property(CONTEXT, default(LINE_BREAK_VAR), impl(Text))]
pub fn line_break(child: impl UiNode, mode: impl IntoVar<LineBreak>) -> impl UiNode {
    with_context_var(child, LINE_BREAK_VAR, mode)
}

/// Alignment of text lines inside text blocks.
///
/// Note that the [`text!`] widget only implements this for text inside each instance in isolation, multiple
/// text instances in an inline row will not all align together by the [`text!`] layout implementation alone.
///
/// Sets the [`TEXT_ALIGN_VAR`] context var.
///
/// [`text!`]: mod@crate::widgets::text
#[property(CONTEXT, default(TEXT_ALIGN_VAR), impl(Text))]
pub fn txt_align(child: impl UiNode, mode: impl IntoVar<Align>) -> impl UiNode {
    with_context_var(child, TEXT_ALIGN_VAR, mode)
}

/// Config the automatic spacing inserted between words and letters when text is aligned to fill.
///
/// Text alignment can be set to [`Align::FILL`], if this config is set to `Some(mode)` when that happens
/// the text layout will automatically insert spaces to try and *fill* the text block. When justify is not
/// enabled, that is set to `None`, fill alignment is the same as [`Align::START`].
///
/// Sets the [`JUSTIFY_VAR`] context var.
#[property(CONTEXT, default(JUSTIFY_VAR), impl(Text))]
pub fn justify(child: impl UiNode, mode: impl IntoVar<Option<Justify>>) -> impl UiNode {
    with_context_var(child, JUSTIFY_VAR, mode)
}

/// Length of the TAB character space, relative to the normal space advance.
///
/// Is set to `400.pct()` by default, so 4 times a space.
///
/// Sets the [`TAB_LENGTH_VAR`] context var.
#[property(CONTEXT, default(TAB_LENGTH_VAR), impl(Text))]
pub fn tab_length(child: impl UiNode, length: impl IntoVar<TabLength>) -> impl UiNode {
    with_context_var(child, TAB_LENGTH_VAR, length)
}

/// Text white space transform.
///
/// Can be used to collapse a sequence of spaces into a single one, or to ignore line-breaks.
/// Is [`WhiteSpace::Preserve`] by default.
///
/// Sets the [`WHITE_SPACE_VAR`] context var.
#[property(CONTEXT, default(WHITE_SPACE_VAR), impl(Text))]
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
    D: FnMut(&mut FontFeatures, S) -> S + Send + 'static,
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
#[property(CONTEXT, default(FONT_VARIATIONS_VAR), impl(Text))]
pub fn font_variations(child: impl UiNode, variations: impl IntoVar<FontVariations>) -> impl UiNode {
    with_context_var(child, FONT_VARIATIONS_VAR, variations)
}

/// Sets font features.
///
/// **Note:** This property fully replaces the font variations for the widget and descendants, use [`with_font_variation`]
/// to create a property that sets a variation but retains others from the context.
#[property(CONTEXT, default(FONT_FEATURES_VAR), impl(Text))]
pub fn font_features(child: impl UiNode, features: impl IntoVar<FontFeatures>) -> impl UiNode {
    with_context_var(child, FONT_FEATURES_VAR, features)
}

/// Sets the font kerning feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_kerning(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.kerning().set(s))
}

/// Sets the font common ligatures features.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_common_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.common_lig().set(s))
}

/// Sets the font discretionary ligatures feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_discretionary_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.discretionary_lig().set(s))
}

/// Sets the font historical ligatures feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_historical_lig(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.historical_lig().set(s))
}

/// Sets the font contextual alternatives feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_contextual_alt(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.contextual_alt().set(s))
}

/// Sets the font capital variant features.
#[property(CONTEXT, default(CapsVariant::Auto), impl(Text))]
pub fn font_caps(child: impl UiNode, state: impl IntoVar<CapsVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.caps().set(s))
}

/// Sets the font numeric variant features.
#[property(CONTEXT, default(NumVariant::Auto), impl(Text))]
pub fn font_numeric(child: impl UiNode, state: impl IntoVar<NumVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.numeric().set(s))
}

/// Sets the font numeric spacing features.
#[property(CONTEXT, default(NumSpacing::Auto), impl(Text))]
pub fn font_num_spacing(child: impl UiNode, state: impl IntoVar<NumSpacing>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.num_spacing().set(s))
}

/// Sets the font numeric fraction features.
#[property(CONTEXT, default(NumFraction::Auto), impl(Text))]
pub fn font_num_fraction(child: impl UiNode, state: impl IntoVar<NumFraction>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.num_fraction().set(s))
}

/// Sets the font swash features.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_swash(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.swash().set(s))
}

/// Sets the font stylistic alternative feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_stylistic(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.stylistic().set(s))
}

/// Sets the font historical forms alternative feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_historical_forms(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.historical_forms().set(s))
}

/// Sets the font ornaments alternative feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_ornaments(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.ornaments().set(s))
}

/// Sets the font annotation alternative feature.
#[property(CONTEXT, default(FontFeatureState::auto()), impl(Text))]
pub fn font_annotation(child: impl UiNode, state: impl IntoVar<FontFeatureState>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.annotation().set(s))
}

/// Sets the font stylistic set alternative feature.
#[property(CONTEXT, default(FontStyleSet::auto()), impl(Text))]
pub fn font_style_set(child: impl UiNode, state: impl IntoVar<FontStyleSet>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.style_set().set(s))
}

/// Sets the font character variant alternative feature.
#[property(CONTEXT, default(CharVariant::auto()), impl(Text))]
pub fn font_char_variant(child: impl UiNode, state: impl IntoVar<CharVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.char_variant().set(s))
}

/// Sets the font sub/super script position alternative feature.
#[property(CONTEXT, default(FontPosition::Auto), impl(Text))]
pub fn font_position(child: impl UiNode, state: impl IntoVar<FontPosition>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.position().set(s))
}

/// Sets the Japanese logographic set.
#[property(CONTEXT, default(JpVariant::Auto), impl(Text))]
pub fn font_jp_variant(child: impl UiNode, state: impl IntoVar<JpVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.jp_variant().set(s))
}

/// Sets the Chinese logographic set.
#[property(CONTEXT, default(CnVariant::Auto), impl(Text))]
pub fn font_cn_variant(child: impl UiNode, state: impl IntoVar<CnVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.cn_variant().set(s))
}

/// Sets the East Asian figure width.
#[property(CONTEXT, default(EastAsianWidth::Auto), impl(Text))]
pub fn font_ea_width(child: impl UiNode, state: impl IntoVar<EastAsianWidth>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.ea_width().set(s))
}

/// Sets the text language and script for the widget and descendants.
///
/// This property affects all texts inside the widget and the layout direction.
///
/// Sets the [`LANG_VAR`] and [`DIRECTION_VAR`] context vars and the [`LayoutMetrics::direction`].
#[property(CONTEXT, default(LANG_VAR), impl(Text))]
pub fn lang(child: impl UiNode, lang: impl IntoVar<Lang>) -> impl UiNode {
    let lang = lang.into_var();
    let child = direction(child, lang.map(|l| l.character_direction().into()));
    with_context_var(child, LANG_VAR, lang)
}

/// Sets the layout direction used in the layout of the widget and descendants.
///
/// Note that the [`lang`] property already sets the direction, this property can be used to directly override the direction.
///
/// Sets the [`DIRECTION_VAR`] context var and the [`LayoutMetrics::direction`].
///
/// [`lang`]: fn@lang
#[property(CONTEXT+1, default(DIRECTION_VAR), impl(Text))]
pub fn direction(child: impl UiNode, direction: impl IntoVar<LayoutDirection>) -> impl UiNode {
    #[ui_node(struct DirectionNode {
        child: impl UiNode,
        #[var] direction: impl Var<LayoutDirection>,
    })]
    impl UiNode for DirectionNode {
        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);

            if self.direction.is_new() {
                WIDGET.layout();
            }
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            LAYOUT.with_direction(self.direction.get(), || self.child.measure(wm))
        }

        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            LAYOUT.with_direction(self.direction.get(), || self.child.layout(wl))
        }
    }
    let direction = direction.into_var();
    let child = DirectionNode {
        child,
        direction: direction.clone(),
    };
    with_context_var(child, DIRECTION_VAR, direction)
}

/// Draw lines *under* each text line.
///
/// Sets the [`UNDERLINE_THICKNESS_VAR`] and [`UNDERLINE_STYLE_VAR`].
#[property(CONTEXT, default(UNDERLINE_THICKNESS_VAR, UNDERLINE_STYLE_VAR), impl(Text))]
pub fn underline(child: impl UiNode, thickness: impl IntoVar<UnderlineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, UNDERLINE_THICKNESS_VAR, thickness);
    with_context_var(child, UNDERLINE_STYLE_VAR, style)
}
/// Custom [`underline`](fn@underline) color, if not set
/// the [`txt_color`](fn@txt_color) is used.
///
/// Sets the [`UNDERLINE_COLOR_VAR`].
#[property(CONTEXT, default(UNDERLINE_COLOR_VAR), impl(Text))]
pub fn underline_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, UNDERLINE_COLOR_VAR, color)
}
/// Defines what segments of each text line are skipped when tracing the [`underline`](fn@underline).
///
/// By default skips glyphs that intercept the underline.
///
/// Sets the [`UNDERLINE_SKIP_VAR`].
#[property(CONTEXT, default(UNDERLINE_SKIP_VAR), impl(Text))]
pub fn underline_skip(child: impl UiNode, skip: impl IntoVar<UnderlineSkip>) -> impl UiNode {
    with_context_var(child, UNDERLINE_SKIP_VAR, skip)
}
/// Defines what font line gets traced by the underline.
///
/// By default uses the font configuration, but it usually crosses over glyph *descents* causing skips on
/// the line, you can set this [`UnderlinePosition::Descent`] to fully clear all glyph *descents*.
///
/// Sets the [`UNDERLINE_POSITION_VAR`].
#[property(CONTEXT, default(UNDERLINE_POSITION_VAR), impl(Text))]
pub fn underline_position(child: impl UiNode, position: impl IntoVar<UnderlinePosition>) -> impl UiNode {
    with_context_var(child, UNDERLINE_POSITION_VAR, position)
}

/// Draw lines *above* each text line.
///
/// Sets the [`OVERLINE_THICKNESS_VAR`] and [`OVERLINE_STYLE_VAR`].
#[property(CONTEXT, default(OVERLINE_THICKNESS_VAR, OVERLINE_STYLE_VAR), impl(Text))]
pub fn overline(child: impl UiNode, thickness: impl IntoVar<TextLineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, OVERLINE_THICKNESS_VAR, thickness);
    with_context_var(child, OVERLINE_STYLE_VAR, style)
}
/// Custom [`overline`](fn@overline) color, if not set
/// the [`txt_color`](fn@txt_color) is used.
///
/// Sets the [`OVERLINE_COLOR_VAR`].
#[property(CONTEXT, default(OVERLINE_COLOR_VAR), impl(Text))]
pub fn overline_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, OVERLINE_COLOR_VAR, color)
}

/// Draw lines across each text line.
///
/// Sets the [`STRIKETHROUGH_THICKNESS_VAR`] and [`STRIKETHROUGH_STYLE_VAR`].
#[property(CONTEXT, default(STRIKETHROUGH_THICKNESS_VAR, STRIKETHROUGH_STYLE_VAR), impl(Text))]
pub fn strikethrough(child: impl UiNode, thickness: impl IntoVar<TextLineThickness>, style: impl IntoVar<LineStyle>) -> impl UiNode {
    let child = with_context_var(child, STRIKETHROUGH_THICKNESS_VAR, thickness);
    with_context_var(child, STRIKETHROUGH_STYLE_VAR, style)
}
/// Custom [`strikethrough`](fn@strikethrough) color, if not set
/// the [`txt_color`](fn@txt_color) is used.
///
/// Sets the [`STRIKETHROUGH_COLOR_VAR`].
#[property(CONTEXT, default(STRIKETHROUGH_COLOR_VAR), impl(Text))]
pub fn strikethrough_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, STRIKETHROUGH_COLOR_VAR, color)
}

/// Sets the [`CARET_COLOR_VAR`].
#[property(CONTEXT, default(CARET_COLOR_VAR), impl(Text))]
pub fn caret_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, CARET_COLOR_VAR, color)
}

/// Enable text selection, copy, caret and input; and makes the widget focusable.
///
/// If the `text` variable is read-only, this only enables text selection, if the var is writeable this
/// enables text input and modifies the variable.
///
/// Sets the [`TEXT_EDITABLE_VAR`].
#[property(CONTEXT, default(TEXT_EDITABLE_VAR), impl(Text))]
pub fn txt_editable(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, TEXT_EDITABLE_VAR, enabled)
}

/// Enables or disables text wrap.
///
/// If enabled, line-breaks and hyphens are automatically inserted to flow the text to fill the available width. Wrap
/// can be configured using the [`line_break`], [`word_break`] and [`hyphens`] properties.
///
/// Sets the [`TEXT_WRAP_VAR`].
///
/// [`line_break`]: fn@line_break
/// [`word_break`]: fn@word_break
/// [`hyphens`]: fn@hyphens
#[property(CONTEXT, default(TEXT_WRAP_VAR), impl(Text))]
pub fn txt_wrap(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    with_context_var(child, TEXT_WRAP_VAR, enabled)
}

/// Configure hyphenation.
///
/// Note that for automatic hyphenation to work the [`lang`] must also be set and the [`Hyphenation`] service must support it.
///
/// The auto hyphenation char can be defined using [`hyphen_char`].
///
/// [`Hyphenation`]: crate::core::text::Hyphenation
/// [`lang`]: fn@lang
/// [`hyphen_char`]: fn@hyphen_char
#[property(CONTEXT, default(HYPHENS_VAR), impl(Text))]
pub fn hyphens(child: impl UiNode, hyphens: impl IntoVar<Hyphens>) -> impl UiNode {
    with_context_var(child, HYPHENS_VAR, hyphens)
}

/// The char or small string that is rendered when text is auto-hyphenated.
///
/// Note that hyphenation is enabled by the [`hyphens`] property.
///
/// [`hyphens`]: fn@hyphens
#[property(CONTEXT, default(HYPHEN_CHAR_VAR), impl(Text))]
pub fn hyphen_char(child: impl UiNode, hyphen: impl IntoVar<Txt>) -> impl UiNode {
    with_context_var(child, HYPHEN_CHAR_VAR, hyphen)
}
