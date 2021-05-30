//! Context properties for theming the [`text!`](module@crate::widgets::text) widget.

use crate::core::text::{font_features::*, *};
use crate::prelude::new_property::*;
use std::marker::PhantomData;

context_var! {
    /// Font family of [`text`](crate::widgets::text) spans.
    pub struct FontFamilyVar: FontNames = once FontNames::default();

    /// Font weight of [`text`](crate::widgets::text) spans.
    pub struct FontWeightVar: FontWeight = const FontWeight::NORMAL;

    /// Font style of [`text`](crate::widgets::text) spans.
    pub struct FontStyleVar: FontStyle = const FontStyle::Normal;

    /// Font stretch of [`text`](crate::widgets::text) spans.
    pub struct FontStretchVar: FontStretch = const FontStretch::NORMAL;

    /// Font synthesis of [`text`](crate::widgets::text) spans.
    pub struct FontSynthesisVar: FontSynthesis = const FontSynthesis::ENABLED;

    /// Font size of [`text`](crate::widgets::text) spans.
    pub struct FontSizeVar: Length = once Length::pt(11.0);

    /// Text color of [`text`](crate::widgets::text) spans.
    pub struct TextColorVar: Rgba = const colors::WHITE;
    /// Text color of [`text`](crate::widgets::text) spans inside a disabled widget.
    pub struct TextColorDisabledVar: Rgba = const colors::GRAY;

    /// Text transformation function applied to [`text`](crate::widgets::text) spans.
    pub struct TextTransformVar: TextTransformFn = return &TextTransformFn::None;

    /// Text line height of [`text`](crate::widgets::text) spans.
    pub struct LineHeightVar: LineHeight = return &LineHeight::Font;

    /// Extra spacing in between lines of [`text`](crate::widgets::text) spans.
    pub struct LineSpacingVar: Length = return &Length::Exact(0.0);

    /// Extra letter spacing of [`text`](crate::widgets::text) spans.
    pub struct LetterSpacingVar: LetterSpacing = return &LetterSpacing::Auto;

    /// Extra word spacing of [`text`](crate::widgets::text) spans.
    pub struct WordSpacingVar: WordSpacing = return &WordSpacing::Auto;

    /// Extra paragraph spacing of text blocks.
    pub struct ParagraphSpacingVar: ParagraphSpacing = return &Length::Exact(0.0);

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

    /// Font features of [`text`](crate::widgets::text) spans.
    pub struct FontFeaturesVar: FontFeatures = once FontFeatures::new();

    /// Font variations of [`text`](crate::widgets::text) spans.
    pub struct FontVariationsVar: FontVariations = once FontVariations::new();
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

/// Sets the [`FontSizeVar`] context var.
#[property(context, default(FontSizeVar))]
pub fn font_size(child: impl UiNode, size: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, FontSizeVar, size)
}

/// Sets the [`TextColorVar`] context var.
#[property(context, default(TextColorVar))]
pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, TextColorVar, color)
}

/// Sets the [`TextColorDisabledVar`] context var.
#[property(context, default(TextColorDisabledVar))]
pub fn text_color_disabled(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, TextColorDisabledVar, color)
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
pub fn line_spacing(child: impl UiNode, extra: impl IntoVar<Length>) -> impl UiNode {
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
    struct WithFontVariationNode<C, V> {
        child: C,

        name: FontVariationName,
        value: V,

        variations: FontVariations,
        version: u32,
    }
    impl<C, V> UiNode for WithFontVariationNode<C, V>
    where
        C: UiNode,
        V: Var<f32>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.variations = FontVariationsVar::get(ctx.vars).clone();
            self.variations.insert(self.name, *self.value.get(ctx.vars));

            self.version = FontVariationsVar::version(ctx.vars);
            let is_new = FontVariationsVar::is_new(ctx.vars);

            if is_new {
                self.version = self.version.wrapping_add(1);
            }

            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontVariationsVar, &self.variations, is_new, self.version, || {
                    child.init(ctx);
                })
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontVariationsVar, &self.variations, false, self.version, || {
                    child.update_hp(ctx);
                });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut is_new = false;

            if let Some(new_ctx) = FontVariationsVar::get_new(ctx.vars) {
                self.variations = new_ctx.clone();
                self.variations.insert(self.name, *self.value.get(ctx.vars));
                self.version = self.version.wrapping_add(1);
                is_new = true;
            } else if let Some(value) = self.value.get_new(ctx.vars) {
                self.variations.insert(self.name, *value);
                self.version = self.version.wrapping_add(1);
                is_new = true;
            }

            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontVariationsVar, &self.variations, is_new, self.version, || {
                    child.update(ctx);
                });
        }

        fn update_hp(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontVariationsVar, &self.variations, false, self.version, || {
                    child.update_hp(ctx);
                });
        }

        fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args)
        where
            Self: Sized,
        {
            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontVariationsVar, &self.variations, false, self.version, || {
                    child.event(ctx, update, args);
                });
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            let child = &mut self.child;
            ctx.vars.with_context_var(FontVariationsVar, &self.variations, self.version, || {
                child.measure(ctx, available_size)
            })
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            let child = &mut self.child;
            ctx.vars.with_context_var(FontVariationsVar, &self.variations, self.version, || {
                child.arrange(ctx, final_size);
            });
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let child = &self.child;
            ctx.vars.with_context_var(FontVariationsVar, &self.variations, self.version, || {
                child.render(ctx, frame);
            });
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            let child = &self.child;
            ctx.vars.with_context_var(FontVariationsVar, &self.variations, self.version, || {
                child.render_update(ctx, update);
            });
        }
    }
    WithFontVariationNode {
        child,
        name,
        value: value.into_var(),
        variations: FontVariations::default(),
        version: 0,
    }
}

/// Include the font feature config in the widget context.
///
/// The modifications done in `modify` are visible only in the [`FontFeaturesVar`] in this context, and features
/// already set in a parent context are included.
pub fn with_font_feature<C, S, V, D>(child: C, state: V, set_feature: D) -> impl UiNode
where
    C: UiNode,
    S: VarValue,
    V: IntoVar<S>,
    D: FnMut(&mut FontFeatures, S) -> S + 'static,
{
    // TODO review performance of this, every feature set duplicates the hash-map,
    // all the intermediary maps are probably only used to make an inner feature map,
    // on the other hand the values are tiny, but if we are so sure it is tiny why are we using a hash-map?.
    struct WithFontFeatureNode<C, S, V, D> {
        child: C,
        _s: PhantomData<S>,
        var: V,
        set_feature: D,

        features: FontFeatures,
        version: u32,
    }
    impl<C, S, V, D> UiNode for WithFontFeatureNode<C, S, V, D>
    where
        C: UiNode,
        S: VarValue,
        V: Var<S>,
        D: FnMut(&mut FontFeatures, S) -> S + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.features = FontFeaturesVar::get(ctx.vars).clone();
            self.version = FontFeaturesVar::version(ctx.vars);
            let is_new = FontFeaturesVar::is_new(ctx.vars);
            if is_new {
                self.version = self.version.wrapping_add(1);
            }

            (self.set_feature)(&mut self.features, self.var.get(ctx.vars).clone());

            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontFeaturesVar, &self.features, is_new, self.version, || {
                    child.init(ctx);
                })
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_var(FontFeaturesVar, &self.features, false, self.version, || {
                child.init(ctx);
            });
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            let mut is_new = false;

            if let Some(new_ctx) = FontFeaturesVar::get_new(ctx.vars) {
                self.features = new_ctx.clone();
                (self.set_feature)(&mut self.features, self.var.get(ctx.vars).clone());
                self.version = self.version.wrapping_add(1);
                is_new = true;
            } else if let Some(value) = self.var.get_new(ctx.vars) {
                (self.set_feature)(&mut self.features, value.clone());
                self.version = self.version.wrapping_add(1);
                is_new = true;
            }

            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontFeaturesVar, &self.features, is_new, self.version, || {
                    child.update(ctx);
                });
        }

        fn update_hp(&mut self, ctx: &mut WidgetContext) {
            let child = &mut self.child;
            ctx.vars.with_context_var(FontFeaturesVar, &self.features, false, self.version, || {
                child.init(ctx);
            });
        }

        fn event<EU: EventUpdate>(&mut self, ctx: &mut WidgetContext, update: EU, args: &EU::Args)
        where
            Self: Sized,
        {
            let child = &mut self.child;
            ctx.vars.with_context_var(FontFeaturesVar, &self.features, false, self.version, || {
                child.event(ctx, update, args);
            });
        }

        fn measure(&mut self, ctx: &mut LayoutContext, available_size: LayoutSize) -> LayoutSize {
            let child = &mut self.child;
            ctx.vars
                .with_context_var(FontFeaturesVar, &self.features, self.version, || child.measure(ctx, available_size))
        }

        fn arrange(&mut self, ctx: &mut LayoutContext, final_size: LayoutSize) {
            let child = &mut self.child;
            ctx.vars.with_context_var(FontFeaturesVar, &self.features, self.version, || {
                child.arrange(ctx, final_size);
            });
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let child = &self.child;
            ctx.vars.with_context_var(FontFeaturesVar, &self.features, self.version, || {
                child.render(ctx, frame);
            });
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            let child = &self.child;
            ctx.vars.with_context_var(FontFeaturesVar, &self.features, self.version, || {
                child.render_update(ctx, update);
            });
        }
    }
    WithFontFeatureNode {
        child,
        _s: PhantomData,

        var: state.into_var(),
        set_feature,

        features: FontFeatures::new(),
        version: 0,
    }
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

/// All the text contextual values.
#[derive(Debug)]
pub struct TextContext<'a> {
    /* Affects font */
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
    pub font_size: Length,
    /// The [`font_variations`](fn@font_variations) value.    
    pub font_variations: &'a FontVariations,

    /* Affects measure */
    /// The [`line_height`](fn@line_height) value.
    pub line_height: LineHeight,
    /// The [`letter_spacing`](fn@letter_spacing) value.
    pub letter_spacing: LetterSpacing,
    /// The [`word_spacing`](fn@word_spacing) value.
    pub word_spacing: WordSpacing,
    /// The [`line_spacing`](fn@line_spacing) value.
    pub line_spacing: Length,
    /// The [`word_break`](fn@word_break) value.
    pub word_break: WordBreak,
    /// The [`line_break`](fn@line_break) value.
    pub line_break: LineBreak,
    /// The [`tab_length`](fn@tab_length) value.
    pub tab_length: TabLength,
    /// The [`font_features`](fn@font_features) value.
    pub font_features: &'a FontFeatures,

    /* Affects arrange */
    /// The [`text_align`](fn@text_align) value.
    pub text_align: TextAlign,

    /* Affects render only */
    /// The [`text_color`](fn@text_color) value.
    pub text_color: Rgba,

    /* Maybe affects render only */
    /// The [`font_synthesis`](fn@font_synthesis) value.
    pub font_synthesis: FontSynthesis,
}
impl<'a> TextContext<'a> {
    /// Borrow or copy all the text contextual values.
    pub fn get(vars: &'a VarsRead) -> Self {
        TextContext {
            font_family: FontFamilyVar::get(vars),
            font_style: *FontStyleVar::get(vars),
            font_weight: *FontWeightVar::get(vars),
            font_stretch: *FontStretchVar::get(vars),

            text_transform: TextTransformVar::get(vars).clone(),
            white_space: *WhiteSpaceVar::get(vars),

            font_size: *FontSizeVar::get(vars),
            font_variations: FontVariationsVar::get(vars),

            line_height: *LineHeightVar::get(vars),
            letter_spacing: *LetterSpacingVar::get(vars),
            word_spacing: *WordSpacingVar::get(vars),
            line_spacing: *LineSpacingVar::get(vars),
            word_break: *WordBreakVar::get(vars),
            line_break: *LineBreakVar::get(vars),
            tab_length: *TabLengthVar::get(vars),
            font_features: FontFeaturesVar::get(vars),

            text_align: *TextAlignVar::get(vars),

            text_color: *TextColorVar::get(vars),

            font_synthesis: *FontSynthesisVar::get(vars),
        }
    }

    /// Gets the properties that affect the font face.
    pub fn font_face(vars: &'a VarsRead) -> (&'a [FontName], FontStyle, FontWeight, FontStretch) {
        (
            FontFamilyVar::get(vars),
            *FontStyleVar::get(vars),
            *FontWeightVar::get(vars),
            *FontStretchVar::get(vars),
        )
    }
    /// Gets [`font_face`](Self::font_face) if any of the properties updated.
    pub fn font_face_update(vars: &'a Vars) -> Option<(&'a [FontName], FontStyle, FontWeight, FontStretch)> {
        if FontFamilyVar::is_new(vars) || FontStyleVar::is_new(vars) || FontWeightVar::is_new(vars) || FontStretchVar::is_new(vars) {
            Some(Self::font_face(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect the text characters.
    #[inline]
    pub fn text(vars: &'a VarsRead) -> (TextTransformFn, WhiteSpace) {
        (TextTransformVar::get(vars).clone(), *WhiteSpaceVar::get(vars))
    }
    /// Gets [`text`](Self::text) if any of the properties updated.
    pub fn text_update(vars: &'a Vars) -> Option<(TextTransformFn, WhiteSpace)> {
        if TextTransformVar::is_new(vars) || WhiteSpaceVar::is_new(vars) {
            Some(Self::text(vars))
        } else {
            None
        }
    }

    /// Gets the properties that affect the sized font. The [`Length`] is `font_size`.
    #[inline]
    pub fn font(vars: &'a VarsRead) -> (Length, &'a FontVariations) {
        (*FontSizeVar::get(vars), FontVariationsVar::get(vars))
    }
    /// Gets [`font`](Self::font) if any of the properties updated.
    #[inline]
    pub fn font_update(vars: &'a Vars) -> Option<(Length, &'a FontVariations)> {
        if FontSizeVar::is_new(vars) || FontVariationsVar::is_new(vars) {
            Some(Self::font(vars))
        } else {
            None
        }
    }

    /// Gets the property that affect color.
    #[inline]
    pub fn color(vars: &'a VarsRead) -> Rgba {
        *TextColorVar::get(vars)
    }
    /// Gets [`color`](Self::color) if the property updated.
    #[inline]
    pub fn color_update(vars: &'a Vars) -> Option<Rgba> {
        TextColorVar::get_new(vars).copied()
    }

    /// Gets the properties that affects what font synthesis is used.
    #[inline]
    pub fn font_synthesis(vars: &'a VarsRead) -> (FontSynthesis, FontStyle, FontWeight) {
        (*FontSynthesisVar::get(vars), *FontStyleVar::get(vars), *FontWeightVar::get(vars))
    }

    /// Gets [`font_synthesis`](Self::font_synthesis) if any of the properties changed.
    #[inline]
    pub fn font_synthesis_update(vars: &'a Vars) -> Option<(FontSynthesis, FontStyle, FontWeight)> {
        if FontSynthesisVar::is_new(vars) || FontStyleVar::is_new(vars) || FontWeightVar::is_new(vars) {
            Some(Self::font_synthesis(vars))
        } else {
            None
        }
    }
}
impl<'a> Clone for TextContext<'a> {
    fn clone(&self) -> Self {
        TextContext {
            font_features: self.font_features,
            text_transform: self.text_transform.clone(),
            font_family: self.font_family,
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_style: self.font_style,
            font_variations: self.font_variations,
            font_stretch: self.font_stretch,
            font_synthesis: self.font_synthesis,
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
        }
    }
}
