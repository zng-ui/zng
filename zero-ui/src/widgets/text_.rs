use crate::prelude::new_widget::*;

/// A configured [`text`](../fn.text.html).
///
/// # Example
///
/// ```
/// use zero_ui::prelude::text;
///
/// let hello_txt = text! {
///     font_family = "Arial";
///     font_size = 18;
///     text = "Hello!";
/// };
/// ```
/// # `text()`
///
/// If you don't need to configure the text, you can just use the function [`text`](../fn.text.html).
#[widget($crate::widgets::text)]
pub mod text {
    use super::*;

    properties! {
        child {
            /// The [`Text`](crate::core::types::Text) value.
            ///
            /// Set to an empty string (`""`).
            text(impl IntoVar<Text>) = "";

            /// Spacing in between the text and background edges or border.
            side_offsets as padding;
        }

        /// The text font. If not set inherits the `font_family` from the parent widget.
        properties::font_family;
        /// The font style. If not set inherits the `font_style` from the parent widget.
        properties::font_style;
        /// The font weight. If not set inherits the `font_weight` from the parent widget.
        properties::font_weight;
        /// The font stretch. If not set inherits the `font_stretch` from the parent widget.
        properties::font_stretch;
        /// The font size. If not set inherits the `font_size` from the parent widget.
        properties::font_size;
        /// The text color. If not set inherits the `text_color` from the parent widget.
        properties::text_color as color;

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
        properties::letter_spacing;

        /// Extra spacing added to the Unicode `U+0020 SPACE` character. If not set inherits the `letter_spacing` from the parent widget.
        ///
        /// Word spacing is done using the space character "advance" as defined in the font,
        /// this unit represents extra spacing added to that default spacing.
        ///
        /// A "word" is the sequence of characters in-between space characters. This extra
        /// spacing is applied per space character not per word, if there are three spaces between words
        /// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
        /// see [`WhiteSpace`](crate::text::WhiteSpace).
        ///
        /// The [`Default`] value signals that word spacing can be tweaked when text *justification* is enabled, all other
        /// values disable automatic adjustments for justification. Relative values are computed from the length of the space `' '` character,
        /// so a word spacing of `100.pct()` visually adds *another* space in between words.
        ///
        /// [`Default`]: Length::Default
        properties::word_spacing;

        /// Height of each text line. If not set inherits the `line_height` from the parent widget.
        ///
        /// The [`Default`] value is computed from the font metrics, `ascent - descent + line_gap`, this is
        /// usually similar to `1.2.em()`. Relative values are computed from the default value, so `200.pct()` is double
        /// the default line height.
        ///
        /// The text is vertically centralized inside the height.
        ///
        /// [`Default`]: Length::Default
        properties::line_height;
        /// Extra spacing in-between text lines. If not set inherits the `line_spacing` from the parent widget.
        ///
        /// The [`Default`] value is zero. Relative values are calculated from the [`LineHeight`], so `50.pct()` is half
        /// the computed line height. If the text only has one line this property is not used.
        ///
        /// [`Default`]: Length::Default
        properties::line_spacing;
    }

    #[inline]
    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        nodes::TextNode::new(text.into_var())
    }

    /// UI nodes used for building a text widget.
    pub mod nodes {
        use super::properties::*;
        use crate::core::text::*;
        use crate::prelude::new_widget::*;

        /// An UI node that renders a text using the [contextual text theme](TextContext).
        pub struct TextNode<T: Var<Text>> {
            text_var: T,

            /* init, update data */
            // Transformed and white space corrected, or empty before init.
            text: SegmentedText,

            // Loaded from [font query](Fonts::get_or_default) during init.
            font_face: Option<FontFaceRef>,

            synthesis_used: FontSynthesis,

            /* measure, arrange data */
            //
            shaping_args: TextShapingArgs,

            #[allow(unused)] // TODO
            layout_line_spacing: f32,
            // Font instance using the actual font_size.
            font: Option<FontRef>,
            // Shaped and wrapped text.
            shaped_text: Option<ShapedText>,
            // Box size of the text block.
            size: PxSize,
        }

        impl<T: Var<Text>> TextNode<T> {
            /// New text node from a [`Text`] variable.
            ///
            /// All other text configuration is taken from context variables.
            pub fn new(text: T) -> TextNode<T> {
                TextNode {
                    text_var: text,

                    text: SegmentedText::default(),
                    font_face: None,

                    synthesis_used: FontSynthesis::DISABLED,

                    shaping_args: TextShapingArgs::default(),
                    layout_line_spacing: 0.0,
                    font: None,
                    shaped_text: None,
                    size: PxSize::zero(),
                }
            }
        }

        #[impl_ui_node(none)]
        impl<T: Var<Text>> UiNode for TextNode<T> {
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                TextContext::subscribe(ctx.vars, subscriptions.var(ctx.vars, &self.text_var));
            }

            fn init(&mut self, ctx: &mut WidgetContext) {
                let (lang, family, style, weight, stretch) = TextContext::font_face(ctx.vars);

                // TODO use the full list.
                let font_face = ctx.services.fonts().get_list(family, style, weight, stretch, lang).best().clone();
                self.synthesis_used = *FontSynthesisVar::get(ctx) & font_face.synthesis_for(style, weight);
                self.font_face = Some(font_face);

                let text = self.text_var.get_clone(ctx);
                let text = TextTransformVar::get(ctx).transform(text);
                let text = WhiteSpaceVar::get(ctx).transform(text);
                self.text = SegmentedText::new(text);
            }

            fn deinit(&mut self, _: &mut WidgetContext) {
                self.font = None;
                self.font_face = None;
                self.shaped_text = None;
                self.text = SegmentedText::default();
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                // update `self.text`, affects shaping and layout
                if let Some(text) = self.text_var.get_new(ctx) {
                    let (text_transform, white_space) = TextContext::text(ctx);
                    let text = text_transform.transform(text.clone());
                    let text = white_space.transform(text);
                    if self.text.text() != text {
                        self.text = SegmentedText::new(text);
                        self.shaped_text = None;

                        ctx.updates.layout();
                    }
                } else if let Some((text_transform, white_space)) = TextContext::text_update(ctx) {
                    let text = self.text_var.get_clone(ctx);
                    let text = text_transform.transform(text);
                    let text = white_space.transform(text);
                    if self.text.text() != text {
                        self.text = SegmentedText::new(text);
                        self.shaped_text = None;

                        ctx.updates.layout();
                    }
                }

                // update `self.font_face`, affects shaping and layout
                if let Some((lang, font_family, font_style, font_weight, font_stretch)) = TextContext::font_face_update(ctx.vars) {
                    let face = ctx
                        .services
                        .fonts()
                        .get_list(font_family, font_style, font_weight, font_stretch, lang)
                        .best()
                        .clone();

                    if !self.font_face.as_ref().map(|f| f.ptr_eq(&face)).unwrap_or_default() {
                        self.synthesis_used = *FontSynthesisVar::get(ctx) & face.synthesis_for(font_style, font_weight);
                        self.font_face = Some(face);
                        self.font = None;
                        self.shaped_text = None;

                        ctx.updates.layout();
                    }
                }

                // update `self.font_instance`, affects shaping and layout
                if TextContext::font_update(ctx).is_some() {
                    self.font = None;
                    self.shaped_text = None;
                    ctx.updates.layout();
                }

                // TODO features, wrapping.

                if let Some((_, _, _, _, _, lang)) = TextContext::shaping_update(ctx) {
                    self.shaping_args.lang = lang.clone();
                    self.shaped_text = None;
                    ctx.updates.layout();
                }

                // update `self.color`
                if TextContext::color_update(ctx).is_some() {
                    ctx.updates.render();
                }

                // update `self.font_synthesis`
                if let Some((synthesis_allowed, style, weight)) = TextContext::font_synthesis_update(ctx) {
                    if let Some(face) = &self.font_face {
                        let synthesis_used = synthesis_allowed & face.synthesis_for(style, weight);
                        if synthesis_used != self.synthesis_used {
                            self.synthesis_used = synthesis_used;
                            ctx.updates.render();
                        }
                    }
                }
            }

            fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                let (font_size, variations) = TextContext::font(ctx);
                let font_size = font_size.to_layout(ctx, available_size.width, ctx.metrics.root_font_size);

                if self.font.as_ref().map(|f| f.size() != font_size).unwrap_or(true) {
                    self.font = Some(
                        self.font_face
                            .as_ref()
                            .expect("font not inited in measure")
                            .sized(font_size, variations.finalize()),
                    );
                    self.shaped_text = None;

                    ctx.updates.render();
                }

                let font = self.font.as_ref().unwrap();

                let (letter_spacing, word_spacing, line_spacing, line_height, tab_length, _lang) = TextContext::shaping(ctx.vars);
                let space_len = font.space_x_advance();
                let dft_tab_len = space_len * 3;
                let space_len = AvailablePx::Finite(space_len);
                let letter_spacing = letter_spacing.to_layout(ctx, space_len, Px(0));
                let word_spacing = word_spacing.to_layout(ctx, space_len, Px(0));
                let tab_length = tab_length.to_layout(ctx, space_len, dft_tab_len);

                let dft_line_height = font.metrics().line_height();
                let line_height = line_height.to_layout(ctx, AvailablePx::Finite(dft_line_height), dft_line_height);
                let line_spacing = line_spacing.to_layout(ctx, AvailablePx::Finite(line_height), Px(0));

                if self.shaped_text.is_some() && letter_spacing != self.shaping_args.letter_spacing
                    || word_spacing != self.shaping_args.word_spacing
                    || tab_length != self.shaping_args.tab_x_advance
                    || line_spacing != self.shaping_args.line_spacing
                    || line_height != self.shaping_args.line_height
                {
                    self.shaped_text = None;
                }
                self.shaping_args.letter_spacing = letter_spacing;
                self.shaping_args.word_spacing = word_spacing;
                self.shaping_args.tab_x_advance = tab_length;
                self.shaping_args.line_height = line_height;
                self.shaping_args.line_spacing = line_spacing;

                if self.shaped_text.is_none() {
                    let shaped_text = font.shape_text(&self.text, &self.shaping_args);
                    self.size = shaped_text.size();
                    self.shaped_text = Some(shaped_text);

                    ctx.updates.render();
                }

                if available_size.width < self.size.width {
                    //TODO wrap here? or estimate the height pos wrap?
                }

                self.size
            }

            fn arrange(&mut self, _ctx: &mut LayoutContext, _: &mut WidgetLayout, _final_size: PxSize) {
                // TODO use final size for wrapping?
                // http://www.unicode.org/reports/tr14/tr14-45.html
            }

            fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                frame.push_text(
                    PxRect::from_size(self.size),
                    self.shaped_text.as_ref().expect("shaped text not inited in render").glyphs(),
                    self.font.as_ref().expect("font not initied in render"),
                    RenderColor::from(*TextColorVar::get(ctx)),
                    self.synthesis_used,
                );
            }
        }
    }

    /// Properties and context variables that configure the appearance of text widgets.
    pub mod properties {
        //! Context properties for theming the [`text!`](module@crate::widgets::text) widget.

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
            pub struct TextAlignVar: TextAlign = TextAlign::Start;

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

        /// Sets the [`FontSizeVar`] context var and the [`LayoutMetrics::font_size`].
        #[property(context, default(FontSizeVar))]
        pub fn font_size(child: impl UiNode, size: impl IntoVar<FontSize>) -> impl UiNode {
            struct FontSizeNode<C> {
                child: C,
                size_new: bool,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for FontSizeNode<C> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    self.size_new = true;
                    self.child.init(ctx);
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    subscriptions.var(ctx, &FontSizeVar::new());
                    self.child.subscriptions(ctx, subscriptions);
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if FontSizeVar::is_new(ctx) {
                        self.size_new = true;
                        ctx.updates.layout();
                    }
                    self.child.update(ctx);
                }

                fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                    let font_size = FontSizeVar::get(ctx.vars).to_layout(ctx, available_size.height, ctx.metrics.root_font_size);
                    ctx.with_font_size(font_size, self.size_new, |ctx| self.child.measure(ctx, available_size))
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                    let font_size =
                        FontSizeVar::get(ctx.vars).to_layout(ctx, AvailablePx::Finite(final_size.height), ctx.metrics.root_font_size);

                    ctx.with_font_size(font_size, self.size_new, |ctx| self.child.arrange(ctx, widget_layout, final_size));
                    self.size_new = false;
                }
            }
            let child = FontSizeNode { child, size_new: true };
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
                    .var(&FontSynthesisVar::new());
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
            pub fn font_face_update<Vw: AsRef<Vars>>(
                vars: &'a Vw,
            ) -> Option<(&'a Lang, &'a [FontName], FontStyle, FontWeight, FontStretch)> {
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
            #[inline]
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
            #[inline]
            pub fn font<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (&'a FontSize, &'a FontVariations) {
                let vars = vars.as_ref();
                (FontSizeVar::get(vars), FontVariationsVar::get(vars))
            }
            /// Gets [`font`](Self::font) if any of the properties updated.
            #[inline]
            pub fn font_update<Vw: AsRef<Vars>>(vars: &'a Vw) -> Option<(&'a FontSize, &'a FontVariations)> {
                let vars = vars.as_ref();
                if FontSizeVar::is_new(vars) || FontVariationsVar::is_new(vars) {
                    Some(Self::font(vars))
                } else {
                    None
                }
            }

            /// Gets the properties that affect text shaping.
            #[inline]
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
            #[inline]
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
            #[inline]
            pub fn wrapping<Vr: AsRef<VarsRead>>(vars: &'a Vr) -> (WordBreak, LineBreak) {
                (*WordBreakVar::get(vars), *LineBreakVar::get(vars))
            }

            /// Gets [`wrapping`](Self::wrapping) if any of the properties updated.
            #[inline]
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
            #[inline]
            pub fn color<Vr: AsRef<VarsRead>>(vars: &Vr) -> Rgba {
                *TextColorVar::get(vars)
            }
            /// Gets [`color`](Self::color) if the property updated.
            #[inline]
            pub fn color_update<Vw: WithVars>(vars: &Vw) -> Option<Rgba> {
                vars.with_vars(|vars| TextColorVar::get_new(vars).copied())
            }

            /// Gets the properties that affects what font synthesis is used.
            #[inline]
            pub fn font_synthesis<Vr: WithVarsRead>(vars: &Vr) -> (FontSynthesis, FontStyle, FontWeight) {
                vars.with_vars_read(|vars| (*FontSynthesisVar::get(vars), *FontStyleVar::get(vars), *FontWeightVar::get(vars)))
            }

            /// Gets [`font_synthesis`](Self::font_synthesis) if any of the properties changed.
            #[inline]
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
                    font_features: self.font_features,
                    text_transform: self.text_transform.clone(),
                    lang: self.lang,
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
    }
}

/// Simple text run.
///
/// # Configure
///
/// Text spans can be configured by setting [`font_family`], [`font_size`] and other properties in parent widgets.
///
/// # Example
/// ```
/// # fn main() -> () {
/// use zero_ui::widgets::{container, text, text::properties::{font_family, font_size}};
///
/// let hello_txt = container! {
///     font_family = "Arial";
///     font_size = 18;
///     content = text("Hello!");
/// };
/// # }
/// ```
///
/// # `text!`
///
/// There is a specific widget for creating configured text runs: [`text!`].
///
/// [`font_family`]: fn@crate::widgets::text::properties::font_family
/// [`font_size`]: fn@crate::widgets::text::properties::font_size
/// [`text_color`]: fn@crate::widgets::text::properties::text_color
/// [`text!`]: mod@text
pub fn text(text: impl IntoVar<Text> + 'static) -> impl Widget {
    // TODO remove 'static when rust issue #42940 is fixed.
    text! {
        text;
    }
}

#[widget($crate::widgets::text_::strong)]
mod strong {
    use super::*;

    properties! {
        child {
            text(impl IntoVar<Text>);
        }
    }

    #[inline]
    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let text = text::nodes::TextNode::new(text.into_var());
        font_weight(text, FontWeight::BOLD)
    }
}

/// A simple text run with **bold** font weight.
///
/// # Configure
///
/// Apart from the font weight this widget can be configured with contextual properties like [`text`](function@text).
pub fn strong(text: impl IntoVar<Text> + 'static) -> impl Widget {
    strong! { text; }
}

#[widget($crate::widgets::text_::em)]
mod em {
    use super::*;

    properties! {
        child {
            text(impl IntoVar<Text>);
        }
    }

    #[inline]
    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let text = text::nodes::TextNode::new(text.into_var());
        font_style(text, FontStyle::Italic)
    }
}

/// A simple text run with *italic* font style.
///
/// # Configure
///
/// Apart from the font style this widget can be configured with contextual properties like [`text`](function@text).
pub fn em(text: impl IntoVar<Text> + 'static) -> impl Widget {
    em! { text; }
}
