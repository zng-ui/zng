//! Context properties for theming the [`text!`](module@crate::widgets::text) widget.

use crate::core::{
    color::{web_colors, RenderColor, Rgba},
    context::{LayoutContext, Vars, WidgetContext},
    impl_ui_node, property,
    render::FrameBuilder,
    text::{font_features::*, *},
    units::*,
    var::{context_var, IntoVar, Var, VarValue},
    UiNode,
};
use crate::properties::with_context_var;
use std::cell::RefCell;
use std::marker::PhantomData;

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

    struct FontFeaturesVar: Option<RefCell<FontFeatures>> = return &None;
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

/// Access to a widget contextual [`FontFeatures`].
///
/// This type is a wrapper
#[derive(Copy, Clone, Debug)]
pub struct FontFeaturesContext;
impl FontFeaturesContext {
    /// Reference the contextual font features, if any is set.
    #[inline]
    pub fn get(vars: &Vars) -> Option<std::cell::Ref<FontFeatures>> {
        vars.context::<FontFeaturesVar>().as_ref().map(RefCell::borrow)
    }

    /// Calls `action` with the contextual feature set to `new_state`.
    pub fn with_feature<S, D, A>(set_feature_state: &mut D, new_state: S, vars: &Vars, action: A)
    where
        S: VarValue,
        D: FnMut(&mut FontFeatures, S) -> S,
        A: FnOnce(),
    {
        if let Some(cell) = vars.context::<FontFeaturesVar>() {
            let prev_state = set_feature_state(&mut *cell.borrow_mut(), new_state);
            action();
            set_feature_state(&mut *cell.borrow_mut(), prev_state);
        } else {
            let mut features = FontFeatures::default();
            set_feature_state(&mut features, new_state);
            vars.with_context(FontFeaturesVar, &Some(RefCell::new(features)), false, 0, action);
            //TODO version?
        }
    }
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
        let child = &mut self.child;
        FontFeaturesContext::with_feature(&mut self.delegate, self.var.get(ctx.vars).clone(), ctx.vars, || child.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        FontFeaturesContext::with_feature(&mut self.delegate, self.var.get(ctx.vars).clone(), ctx.vars, || child.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        let child = &mut self.child;
        FontFeaturesContext::with_feature(&mut self.delegate, self.var.get(ctx.vars).clone(), ctx.vars, || child.update(ctx));
    }

    // TODO update_hp?
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
pub fn font_style_set(child: impl UiNode, state: impl IntoVar<FontStyleSet>) -> impl UiNode {
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

/// Sets the Japanese logographic set.
#[property(context)]
pub fn font_jp_variant(child: impl UiNode, state: impl IntoVar<JpVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.jp_variant().set(s))
}

/// Sets the Chinese logographic set.
#[property(context)]
pub fn font_cn_variant(child: impl UiNode, state: impl IntoVar<CnVariant>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.cn_variant().set(s))
}

/// Sets the East Asian figure width.
#[property(context)]
pub fn font_ea_width(child: impl UiNode, state: impl IntoVar<EastAsianWidth>) -> impl UiNode {
    with_font_feature(child, state, |f, s| f.ea_width().set(s))
}

/// All the text contextual values.
#[derive(Debug)]
pub struct TextContext<'a> {
    pub font_family: &'a [FontName],
    pub font_size: Length,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_strech: FontStretch,
    pub text_color: Rgba,
    pub text_transform: TextTransformFn,
    pub line_height: LineHeight,
    pub letter_spacing: LetterSpacing,
    pub word_spacing: WordSpacing,
    pub word_break: WordBreak,
    pub line_break: LineBreak,
    pub text_align: TextAlign,
    pub tab_length: TabLength,
    pub white_space: WhiteSpace,
    pub font_features: Option<std::cell::Ref<'a, FontFeatures>>,
}
impl<'a> TextContext<'a> {
    /// Borrow or copy all the text contextual values.
    pub fn get(vars: &'a Vars) -> Self {
        TextContext {
            font_family: vars.context::<FontFamilyVar>(),
            font_size: *vars.context::<FontSizeVar>(),
            font_weight: *vars.context::<FontWeightVar>(),
            font_style: *vars.context::<FontStyleVar>(),
            font_strech: *vars.context::<FontStretchVar>(),
            text_color: *vars.context::<TextColorVar>(),
            text_transform: vars.context::<TextTransformVar>().clone(),
            line_height: *vars.context::<LineHeightVar>(),
            letter_spacing: *vars.context::<LetterSpacingVar>(),
            word_spacing: *vars.context::<WordSpacingVar>(),
            word_break: *vars.context::<WordBreakVar>(),
            line_break: *vars.context::<LineBreakVar>(),
            text_align: *vars.context::<TextAlignVar>(),
            tab_length: *vars.context::<TabLengthVar>(),
            white_space: *vars.context::<WhiteSpaceVar>(),
            font_features: FontFeaturesContext::get(vars),
        }
    }

    pub fn make_layout_data(&self, text: Text, ctx: &mut WidgetContext) -> TextLayoutData {
        let font = ctx
            .window_services
            .req::<Fonts>()
            .get_or_default(self.font_family, self.font_style, self.font_weight, self.font_strech);

        let text = self.text_transform.transform(text);
        let text = self.white_space.transform(text);

        TextLayoutData {
            text,
            font,
            font_size: self.font_size,

            font_instance: None,
            shaped: vec![],
            size: None,
            color: self.text_color.into(),
        }
    }
}
impl<'a> Clone for TextContext<'a> {
    fn clone(&self) -> Self {
        TextContext {
            font_features: self.font_features.as_ref().map(std::cell::Ref::clone),
            text_transform: self.text_transform.clone(),

            font_family: self.font_family,
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_style: self.font_style,
            font_strech: self.font_strech,
            text_color: self.text_color,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            word_spacing: self.word_spacing,
            word_break: self.word_break,
            line_break: self.line_break,
            text_align: self.text_align,
            tab_length: self.tab_length,
            white_space: self.white_space,
        }
    }
}

/// Text layout data.
pub struct TextLayoutData {
    /// Transformed text.
    pub text: Text,

    /// Reference to font.
    pub font: Font,
    pub font_size: Length,

    /// Reference to font instance at a size.
    pub font_instance: Option<FontInstance>,
    pub shaped: Vec<ShapedLine>,
    pub size: Option<LayoutSize>,

    pub color: RenderColor,
}
impl TextLayoutData {
    pub fn update(&mut self, ctx: &mut WidgetContext) {
        todo!()
    }

    pub fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        todo!()
    }

    pub fn render(&mut self, frame: &mut FrameBuilder) {
        todo!()
    }
}
