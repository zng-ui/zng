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
        /// The [`Text`](crate::core::types::Text) value.
        ///
        /// Set to an empty string (`""`) by default.
        text(impl IntoVar<Text>) = "";

        /// Spacing in between the text and background edges or border.
        ///
        /// Set to `0` by default.
        margin as padding = 0;

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

        /// Draw lines *above* each text line.
        properties::overline;
        /// Custom [`overline`](#wp-overline) color, if not set
        /// the [`color`](#wp-color) is used.
        properties::overline_color;

        /// Draw lines across each text line.
        properties::strikethrough;
        /// Custom [`strikethrough`](#wp-strikethrough) color, if not set
        /// the [`color`](#wp-color) is used.
        properties::strikethrough_color;

        /// Draw lines *under* each text line.
        properties::underline;
        /// Custom [`underline`](#wp-underline) color, if not set
        /// the [`color`](#wp-color) is used.
        properties::underline_color;
        /// Defines what segments of each text line are skipped when tracing the [`underline`](#wp-underline).
        ///
        /// By default skips glyphs that intercept the underline.
        properties::underline_skip;
        /// Defines what font line gets traced by the underline.
        ///
        /// By default uses the font configuration, but it usually crosses over glyph *descents* causing skips on
        /// the line, you can set this [`UnderlinePosition::Descent`] to fully clear all glyph *descents*.
        properties::underline_position;
    }

    fn new_child() -> impl UiNode {
        let child = nodes::render_text();
        let child = nodes::render_overlines(child);
        let child = nodes::render_strikethroughs(child);
        nodes::render_underlines(child)
    }

    fn new_fill(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
        nodes::layout_text(child, padding)
    }

    fn new_border(child: impl UiNode) -> impl UiNode {
        nodes::inner(child)
    }

    fn new_event(child: impl UiNode, text: impl IntoVar<Text>) -> impl UiNode {
        nodes::resolve_text(child, text)
    }

    /// UI nodes used for building a text widget.
    pub mod nodes {
        use std::cell::Cell;

        use super::properties::*;
        use crate::core::text::*;
        use crate::prelude::new_widget::*;

        /// Represents the resolved fonts and the transformed, white space corrected and segmented text.
        #[derive(Debug, Clone)]
        pub struct ResolvedText {
            /// Text transformed, white space corrected and segmented.
            pub text: SegmentedText,
            /// Queried font faces.
            pub faces: FontFaceList,
            /// Font synthesis allowed by the text context and required to render the best font match.
            pub synthesis: FontSynthesis,
            /// Final overline color.
            pub overline_color: Rgba,
            /// Final strikethrough color.
            pub strikethrough_color: Rgba,
            /// Final underline color.
            pub underline_color: Rgba,

            /// If the `text` or `faces` has updated, this value is `true` in the update the value changed and stays `true`
            /// until after layout.
            pub reshape: bool,

            /// Baseline set by `layout_text` during measure and used by `new_border` during arrange.
            baseline: Cell<Px>,
        }
        impl ResolvedText {
            /// Gets the contextual [`ResolvedText`], returns `Some(_)` for any property with priority `event` or up
            /// set directly on a `text!` widget or any widget that uses the [`resolve_text`] node.
            pub fn get<Vr: AsRef<VarsRead>>(vars: &Vr) -> Option<&ResolvedText> {
                ResolvedTextVar::get(vars).as_ref()
            }
        }

        /// Represents the layout text.
        #[derive(Debug, Clone)]
        pub struct LayoutText {
            /// Sized [`faces`].
            ///
            /// [`faces`]: ResolvedText::faces
            pub fonts: FontList,

            /// Layout text.
            pub shaped_text: ShapedText,

            /// List of overline segments, defining origin and width of each line.
            ///
            /// Note that overlines are only computed if the `overline_thickness` is more than `0`.
            ///
            /// Default overlines are rendered by [`render_overlines`].
            pub overlines: Vec<(PxPoint, Px)>,

            /// Computed [`OverlineThicknessVar`].
            pub overline_thickness: Px,

            /// List of strikethrough segments, defining origin and width of each line.
            ///
            /// Note that strikethroughs are only computed if the `strikethrough_thickness` is more than `0`.
            ///
            /// Default overlines are rendered by [`render_strikethroughs`].
            pub strikethroughs: Vec<(PxPoint, Px)>,
            /// Computed [`StrikethroughThicknessVar`].
            pub strikethrough_thickness: Px,

            /// List of underline segments, defining origin and width of each line.
            ///
            /// Note that underlines are only computed if the `underline_thickness` is more than `0`.
            ///
            /// Default overlines are rendered by [`render_underlines`].
            ///
            /// Note that underlines trop down from these lines.
            pub underlines: Vec<(PxPoint, Px)>,
            /// Computed [`UnderlineThicknessVar`].
            pub underline_thickness: Px,
        }
        impl LayoutText {
            /// Gets t he contextual [`LayoutText`], returns `Some(_)` in the node layout and render methods for any property
            /// with priority `border` or `fill` set directly on a `text!` widget or any widget that uses the [`layout_text`] node.
            pub fn get<Vr: AsRef<VarsRead>>(vars: &Vr) -> Option<&LayoutText> {
                LayoutTextVar::get(vars).as_ref()
            }
        }

        context_var! {
            /// Represents the contextual [`ResolvedText`] setup by the [`resolve_text`] node.
            struct ResolvedTextVar: Option<ResolvedText> = None;
            /// Represents the contextual [`LayoutText`] setup by the [`layout_text`] node.
            struct LayoutTextVar: Option<LayoutText> = None;
        }

        /// An UI node that resolves the [`TextContext`], applies the text transform and white space correction and segments the `text`.
        ///
        /// This node setups the [`ResolvedText`] for all inner nodes, the `text!` widget introduces this node at the `new_event` constructor,
        /// so all properties except priority *context* have access using the [`ResolvedText::get`] function.
        ///
        /// This node also subscribes to the entire [`TextContext`] so other `text!` properties don't need to.
        pub fn resolve_text(child: impl UiNode, text: impl IntoVar<Text>) -> impl UiNode {
            struct ResolveTextNode<C, T> {
                child: C,
                text: T,
                resolved: Option<ResolvedText>,
            }
            impl<C: UiNode, T> ResolveTextNode<C, T> {
                fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut C) -> R) -> R {
                    vars.with_context_var(ResolvedTextVar, ContextVarData::fixed(&self.resolved), || f(&mut self.child))
                }
                fn with(&self, vars: &VarsRead, f: impl FnOnce(&C)) {
                    vars.with_context_var(ResolvedTextVar, ContextVarData::fixed(&self.resolved), || f(&self.child))
                }
            }
            impl<C: UiNode, T: Var<Text>> UiNode for ResolveTextNode<C, T> {
                fn init(&mut self, ctx: &mut WidgetContext) {
                    let (lang, family, style, weight, stretch) = TextContext::font_face(ctx.vars);

                    let faces = ctx.services.fonts().get_list(family, style, weight, stretch, lang);

                    let text = self.text.get_clone(ctx);
                    let text = TextTransformVar::get(ctx).transform(text);
                    let text = WhiteSpaceVar::get(ctx).transform(text);

                    let text_color = *TextColorVar::get(ctx);

                    self.resolved = Some(ResolvedText {
                        synthesis: *FontSynthesisVar::get(ctx) & faces.best().synthesis_for(style, weight),
                        faces,
                        text: SegmentedText::new(text),
                        overline_color: OverlineColorVar::get(ctx).unwrap_or(text_color),
                        strikethrough_color: StrikethroughColorVar::get(ctx).unwrap_or(text_color),
                        underline_color: UnderlineColorVar::get(ctx).unwrap_or(text_color),
                        reshape: false,
                        baseline: Cell::new(Px(0)),
                    });

                    self.with_mut(ctx.vars, |c| c.init(ctx))
                }

                fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
                    self.with(ctx.vars, |c| c.info(ctx, info))
                }

                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    TextContext::subscribe(ctx.vars, subscriptions.var(ctx.vars, &self.text));
                    self.with(ctx.vars, |c| c.subscriptions(ctx, subscriptions))
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.with_mut(ctx.vars, |c| c.deinit(ctx));
                    self.resolved = None;
                }

                fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
                    if let Some(_args) = FontChangedEvent.update(args) {
                        // font query may return a different result.

                        let (lang, font_family, font_style, font_weight, font_stretch) = TextContext::font_face(ctx.vars);
                        let faces = ctx
                            .services
                            .fonts()
                            .get_list(font_family, font_style, font_weight, font_stretch, lang);

                        let r = self.resolved.as_mut().unwrap();

                        if r.faces != faces {
                            r.synthesis = *FontSynthesisVar::get(ctx) & faces.best().synthesis_for(font_style, font_weight);
                            r.faces = faces;

                            r.reshape = true;
                            ctx.updates.layout();
                        }

                        self.with_mut(ctx.vars, |c| c.event(ctx, args))
                    } else {
                        self.with_mut(ctx.vars, |c| c.event(ctx, args))
                    }
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    let r = self.resolved.as_mut().unwrap();

                    // update `r.text`, affects layout.
                    if let Some(text) = self.text.get_new(ctx) {
                        let (text_transform, white_space) = TextContext::text(ctx);
                        let text = text_transform.transform(text.clone());
                        let text = white_space.transform(text);
                        if r.text.text() != text {
                            r.text = SegmentedText::new(text);

                            r.reshape = true;
                            ctx.updates.layout();
                        }
                    } else if let Some((text_transform, white_space)) = TextContext::text_update(ctx) {
                        let text = self.text.get_clone(ctx);
                        let text = text_transform.transform(text);
                        let text = white_space.transform(text);
                        if r.text.text() != text {
                            r.text = SegmentedText::new(text);

                            r.reshape = true;
                            ctx.updates.layout();
                        }
                    }

                    // update `r.font_face`, affects layout
                    if let Some((lang, font_family, font_style, font_weight, font_stretch)) = TextContext::font_face_update(ctx.vars) {
                        let faces = ctx
                            .services
                            .fonts()
                            .get_list(font_family, font_style, font_weight, font_stretch, lang);

                        if r.faces != faces {
                            r.synthesis = *FontSynthesisVar::get(ctx) & faces.best().synthesis_for(font_style, font_weight);
                            r.faces = faces;

                            r.reshape = true;
                            ctx.updates.layout();
                        }
                    }

                    // update `r.synthesis`, affects render
                    if let Some((synthesis_allowed, style, weight)) = TextContext::font_synthesis_update(ctx) {
                        let synthesis = synthesis_allowed & r.faces.best().synthesis_for(style, weight);
                        if r.synthesis != synthesis {
                            r.synthesis = synthesis;
                            ctx.updates.render();
                        }
                    }

                    // update decoration line colors, affects render
                    if let Some(c) = OverlineColorVar::get_new(ctx) {
                        let c = c.unwrap_or(*TextColorVar::get(ctx));
                        if c != r.overline_color {
                            r.overline_color = c;
                            ctx.updates.render();
                        }
                    }
                    if let Some(c) = StrikethroughColorVar::get_new(ctx) {
                        let c = c.unwrap_or(*TextColorVar::get(ctx));
                        if c != r.strikethrough_color {
                            r.strikethrough_color = c;
                            ctx.updates.render();
                        }
                    }
                    if let Some(c) = UnderlineColorVar::get_new(ctx) {
                        let c = c.unwrap_or(*TextColorVar::get(ctx));
                        if c != r.underline_color {
                            r.underline_color = c;
                            ctx.updates.render();
                        }
                    }

                    self.with_mut(ctx.vars, |c| c.update(ctx))
                }

                fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                    self.with_mut(ctx.vars, |c| c.measure(ctx, available_size))
                }

                fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                    self.with_mut(ctx.vars, |c| c.arrange(ctx, widget_layout, final_size));
                    self.resolved.as_mut().unwrap().reshape = false;
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    self.with(ctx.vars, |c| c.render(ctx, frame))
                }

                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    self.with(ctx.vars, |c| c.render_update(ctx, update))
                }
            }
            ResolveTextNode {
                child,
                text: text.into_var(),
                resolved: None,
            }
        }

        /// Custom [`implicit_base::nodes::inner`] that setups the text baseline.
        ///
        /// The `text!` widget overrides the `new_border` constructor with this node.
        ///
        /// [``]
        pub fn inner(child: impl UiNode) -> impl UiNode {
            implicit_base::nodes::inner(child, |ctx, _| {
                ResolvedText::get(ctx).expect("expected `ResolvedText` in `inner`").baseline.get()
            })
        }

        /// An UI node that layouts the parent [`ResolvedText`] according with the [`TextContext`].
        ///
        /// This node setups the [`LayoutText`] for all inner nodes in the layout and render methods, the `text!` widget introduces this
        /// node at the `new_fill` constructor, so all properties with priority `fill` have access to the [`LayoutText::get`] function.
        pub fn layout_text(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
            bitflags::bitflags! {
                struct Layout: u8 {
                    const UNDERLINE     = 0b0000_0001;
                    const STRIKETHROUGH = 0b0000_0010;
                    const OVERLINE      = 0b0000_0100;
                    const PADDING       = 0b0000_1111;
                    const RESHAPE       = 0b0001_1111;
                }
            }
            struct LayoutTextNode<C, P> {
                child: C,
                padding: P,
                layout: Option<LayoutText>,
                shaping_args: TextShapingArgs,
                pending: Layout,
            }
            impl<C: UiNode, P> LayoutTextNode<C, P> {
                fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut C) -> R) -> R {
                    vars.with_context_var(LayoutTextVar, ContextVarData::fixed(&self.layout), || f(&mut self.child))
                }
                fn with(&self, vars: &VarsRead, f: impl FnOnce(&C)) {
                    vars.with_context_var(LayoutTextVar, ContextVarData::fixed(&self.layout), || f(&self.child))
                }
            }
            #[impl_ui_node(child)]
            impl<C: UiNode, P: Var<SideOffsets>> UiNode for LayoutTextNode<C, P> {
                fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                    subscriptions.var(ctx, &self.padding);
                    // other subscriptions are handled by the `resolve_text` node.

                    self.child.subscriptions(ctx, subscriptions)
                }

                fn deinit(&mut self, ctx: &mut WidgetContext) {
                    self.child.deinit(ctx);
                    self.layout = None;
                }

                fn update(&mut self, ctx: &mut WidgetContext) {
                    if TextContext::font_update(ctx).is_some() {
                        self.pending.insert(Layout::RESHAPE);
                        ctx.updates.layout();
                    }

                    if let Some((_, _, _, _, _, lang)) = TextContext::shaping_update(ctx.vars) {
                        self.shaping_args.lang = lang.clone();
                        self.pending.insert(Layout::RESHAPE);
                        ctx.updates.layout();
                    }

                    if UnderlinePositionVar::is_new(ctx) || UnderlineSkipVar::is_new(ctx) {
                        self.pending.insert(Layout::UNDERLINE);
                        ctx.updates.layout();
                    }

                    if OverlineThicknessVar::is_new(ctx)
                        || StrikethroughThicknessVar::is_new(ctx)
                        || UnderlineThicknessVar::is_new(ctx)
                        || self.padding.is_new(ctx)
                    {
                        ctx.updates.layout();
                    }

                    self.child.update(ctx);
                }

                fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
                    let t = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `layout_text`");

                    if t.reshape {
                        self.pending.insert(Layout::RESHAPE);
                    }

                    let padding = self.padding.get(ctx.vars).to_layout(ctx, available_size, PxSideOffsets::zero());
                    let diff = PxSize::new(padding.horizontal(), padding.vertical());
                    let available_size = available_size.sub_px(diff);

                    let (font_size, variations) = TextContext::font(ctx);
                    let font_size = font_size.to_layout(ctx, available_size.width, ctx.metrics.root_font_size);

                    if self.layout.is_none() {
                        self.layout = Some(LayoutText {
                            fonts: t.faces.sized(font_size, variations.finalize()),
                            shaped_text: ShapedText::default(),
                            overlines: vec![],
                            overline_thickness: Px(0),
                            strikethroughs: vec![],
                            strikethrough_thickness: Px(0),
                            underlines: vec![],
                            underline_thickness: Px(0),
                        });

                        self.pending.insert(Layout::RESHAPE);
                    }

                    let r = self.layout.as_mut().unwrap();

                    if font_size != r.fonts.requested_size() {
                        r.fonts = t.faces.sized(font_size, variations.finalize());

                        self.pending.insert(Layout::RESHAPE);
                    }

                    if !self.pending.contains(Layout::PADDING) && r.shaped_text.padding() != padding {
                        self.pending.insert(Layout::PADDING);
                    }

                    let font = r.fonts.best();

                    let (letter_spacing, word_spacing, line_spacing, line_height, tab_length, _) = TextContext::shaping(ctx.vars);
                    let space_len = font.space_x_advance();
                    let dft_tab_len = space_len * 3;
                    let space_len = AvailablePx::Finite(space_len);
                    let letter_spacing = letter_spacing.to_layout(ctx, space_len, Px(0));
                    let word_spacing = word_spacing.to_layout(ctx, space_len, Px(0));
                    let tab_length = tab_length.to_layout(ctx, space_len, dft_tab_len);
                    let dft_line_height = font.metrics().line_height();
                    let line_height = line_height.to_layout(ctx, AvailablePx::Finite(dft_line_height), dft_line_height);
                    let line_spacing = line_spacing.to_layout(ctx, AvailablePx::Finite(line_height), Px(0));

                    if !self.pending.contains(Layout::RESHAPE)
                        && (letter_spacing != self.shaping_args.letter_spacing
                            || word_spacing != self.shaping_args.word_spacing
                            || tab_length != self.shaping_args.tab_x_advance
                            || line_spacing != self.shaping_args.line_spacing
                            || line_height != self.shaping_args.line_height)
                    {
                        self.pending.insert(Layout::RESHAPE);
                    }

                    self.shaping_args.letter_spacing = letter_spacing;
                    self.shaping_args.word_spacing = word_spacing;
                    self.shaping_args.tab_x_advance = tab_length;
                    self.shaping_args.line_height = line_height;
                    self.shaping_args.line_spacing = line_spacing;

                    let dft_thickness = font.metrics().underline_thickness;
                    let av_height = AvailablePx::Finite(line_height);
                    let overline = OverlineThicknessVar::get(ctx.vars).to_layout(ctx, av_height, dft_thickness);
                    let strikethrough = StrikethroughThicknessVar::get(ctx.vars).to_layout(ctx, av_height, dft_thickness);
                    let underline = UnderlineThicknessVar::get(ctx.vars).to_layout(ctx, av_height, dft_thickness);

                    if !self.pending.contains(Layout::OVERLINE) && (r.overline_thickness == Px(0) && overline > Px(0)) {
                        self.pending.insert(Layout::OVERLINE);
                    }
                    if !self.pending.contains(Layout::STRIKETHROUGH) && (r.strikethrough_thickness == Px(0) && strikethrough > Px(0)) {
                        self.pending.insert(Layout::STRIKETHROUGH);
                    }
                    if !self.pending.contains(Layout::UNDERLINE) && (r.underline_thickness == Px(0) && underline > Px(0)) {
                        self.pending.insert(Layout::UNDERLINE);
                    }
                    r.overline_thickness = overline;
                    r.strikethrough_thickness = strikethrough;
                    r.underline_thickness = underline;

                    /*
                        APPLY
                    */
                    if self.pending.contains(Layout::RESHAPE) {
                        r.shaped_text = r.fonts.shape_text(&t.text, &self.shaping_args);
                    }
                    if self.pending.contains(Layout::PADDING) {
                        r.shaped_text.set_padding(padding);

                        let baseline = r.shaped_text.box_baseline() + padding.bottom;
                        t.baseline.set(baseline);
                    }
                    if self.pending.contains(Layout::OVERLINE) {
                        if r.overline_thickness > Px(0) {
                            r.overlines = r.shaped_text.lines().map(|l| l.overline()).collect();
                        } else {
                            r.overlines = vec![];
                        }
                    }
                    if self.pending.contains(Layout::STRIKETHROUGH) {
                        if r.strikethrough_thickness > Px(0) {
                            r.strikethroughs = r.shaped_text.lines().map(|l| l.strikethrough()).collect();
                        } else {
                            r.strikethroughs = vec![];
                        }
                    }
                    if self.pending.contains(Layout::UNDERLINE) {
                        if r.underline_thickness > Px(0) {
                            let skip = *UnderlineSkipVar::get(ctx);
                            match *UnderlinePositionVar::get(ctx) {
                                UnderlinePosition::Font => {
                                    if skip == UnderlineSkip::GLYPHS | UnderlineSkip::SPACES {
                                        r.underlines = r
                                            .shaped_text
                                            .lines()
                                            .flat_map(|l| l.underline_skip_glyphs_and_spaces(r.underline_thickness))
                                            .collect();
                                    } else if skip.contains(UnderlineSkip::GLYPHS) {
                                        r.underlines = r
                                            .shaped_text
                                            .lines()
                                            .flat_map(|l| l.underline_skip_glyphs(r.underline_thickness))
                                            .collect();
                                    } else if skip.contains(UnderlineSkip::SPACES) {
                                        r.underlines = r.shaped_text.lines().flat_map(|l| l.underline_skip_spaces()).collect();
                                    } else {
                                        r.underlines = r.shaped_text.lines().map(|l| l.underline()).collect();
                                    }
                                }
                                UnderlinePosition::Descent => {
                                    // descent clears all glyphs, so we only need to care about spaces
                                    if skip.contains(UnderlineSkip::SPACES) {
                                        r.underlines = r.shaped_text.lines().flat_map(|l| l.underline_descent_skip_spaces()).collect();
                                    } else {
                                        r.underlines = r.shaped_text.lines().map(|l| l.underline_descent()).collect();
                                    }
                                }
                            }
                        } else {
                            r.underlines = vec![];
                        }
                    }

                    if self.pending != Layout::empty() {
                        ctx.updates.render();
                        self.pending = Layout::empty();
                    }

                    let desired_size = r.shaped_text.size();
                    self.with_mut(ctx.vars, |c| c.measure(ctx, AvailableSize::finite(desired_size)));
                    desired_size
                }
                fn arrange(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
                    // TODO, text wrapping

                    self.with_mut(ctx.vars, |c| c.arrange(ctx, widget_layout, final_size))
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    self.with(ctx.vars, |c| c.render(ctx, frame))
                }
                fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
                    self.with(ctx.vars, |c| c.render_update(ctx, update))
                }
            }
            LayoutTextNode {
                child,
                padding: padding.into_var(),
                layout: None,
                shaping_args: TextShapingArgs::default(),
                pending: Layout::empty(),
            }
        }

        /// An Ui node that renders the default underline visual using the parent [`LayoutText`].
        ///
        /// The lines are rendered before `child`, under it.
        ///
        /// The `text!` widgets introduces this node in `new_child`, around the [`render_strikethroughs`] node.
        pub fn render_underlines(child: impl UiNode) -> impl UiNode {
            struct RenderUnderlineNode<C> {
                child: C,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for RenderUnderlineNode<C> {
                // subscriptions are handled by the `resolve_text` node.
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if UnderlineStyleVar::is_new(ctx) {
                        ctx.updates.render();
                    }

                    self.child.update(ctx);
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    let t = LayoutText::get(ctx.vars).expect("expected `LayoutText` in `render_underlines`");
                    if !t.underlines.is_empty() {
                        let r = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `render_underlines`");

                        let style = *UnderlineStyleVar::get(ctx);
                        if style != LineStyle::Hidden {
                            let color = r.underline_color.into();
                            for &(origin, width) in &t.underlines {
                                frame.push_line(
                                    PxRect::new(origin, PxSize::new(width, t.underline_thickness)),
                                    LineOrientation::Horizontal,
                                    color,
                                    style,
                                );
                            }
                        }
                    }

                    self.child.render(ctx, frame);
                }
            }
            RenderUnderlineNode { child }
        }

        /// An Ui node that renders the default strikethrough visual using the parent [`LayoutText`].
        ///
        /// The lines are rendered after `child`, over it.
        ///
        /// The `text!` widgets introduces this node in `new_child`, around the [`render_overlines`] node.
        pub fn render_strikethroughs(child: impl UiNode) -> impl UiNode {
            struct RenderStrikethroughsNode<C> {
                child: C,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for RenderStrikethroughsNode<C> {
                // subscriptions are handled by the `resolve_text` node.
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if StrikethroughStyleVar::is_new(ctx) {
                        ctx.updates.render();
                    }

                    self.child.update(ctx);
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    let t = LayoutText::get(ctx.vars).expect("expected `LayoutText` in `render_strikethroughs`");
                    if !t.strikethroughs.is_empty() {
                        let r = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `render_strikethroughs`");

                        let style = *StrikethroughStyleVar::get(ctx);
                        if style != LineStyle::Hidden {
                            let color = r.strikethrough_color.into();
                            for &(origin, width) in &t.strikethroughs {
                                frame.push_line(
                                    PxRect::new(origin, PxSize::new(width, t.strikethrough_thickness)),
                                    LineOrientation::Horizontal,
                                    color,
                                    style,
                                );
                            }
                        }
                    }

                    self.child.render(ctx, frame);
                }
            }
            RenderStrikethroughsNode { child }
        }

        /// An Ui node that renders the default overline visual using the parent [`LayoutText`].
        ///
        /// The lines are rendered before `child`, under it.
        ///
        /// The `text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
        pub fn render_overlines(child: impl UiNode) -> impl UiNode {
            struct RenderOverlineNode<C> {
                child: C,
            }
            #[impl_ui_node(child)]
            impl<C: UiNode> UiNode for RenderOverlineNode<C> {
                // subscriptions are handled by the `resolve_text` node.
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if OverlineStyleVar::is_new(ctx) {
                        ctx.updates.render();
                    }

                    self.child.update(ctx);
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    let t = LayoutText::get(ctx.vars).expect("expected `LayoutText` in `render_overlines`");
                    if !t.overlines.is_empty() {
                        let r = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `render_overlines`");

                        let style = *OverlineStyleVar::get(ctx);
                        if style != LineStyle::Hidden {
                            let color = r.overline_color.into();
                            for &(origin, width) in &t.overlines {
                                frame.push_line(
                                    PxRect::new(origin, PxSize::new(width, t.overline_thickness)),
                                    LineOrientation::Horizontal,
                                    color,
                                    style,
                                );
                            }
                        }
                    }

                    self.child.render(ctx, frame);
                }
            }
            RenderOverlineNode { child }
        }

        /// An UI node that renders the parent [`LayoutText`].
        ///
        /// This node renders the text only, decorators are rendered by other nodes.
        ///
        /// This is the `text!` widget inner most leaf node, introduced in the `new_child` constructor.
        pub fn render_text() -> impl UiNode {
            struct RenderTextNode;
            #[impl_ui_node(none)]
            impl UiNode for RenderTextNode {
                // subscriptions are handled by the `resolve_text` node.
                fn update(&mut self, ctx: &mut WidgetContext) {
                    if TextColorVar::is_new(ctx) {
                        ctx.updates.render();
                    }
                }

                fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
                    let r = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `render_text`");
                    let t = LayoutText::get(ctx.vars).expect("expected `LayoutText` in `render_text`");

                    let clip = PxRect::from_size(t.shaped_text.size());
                    let color = (*TextColorVar::get(ctx.vars)).into();

                    for (font, glyphs) in t.shaped_text.glyphs() {
                        frame.push_text(clip, glyphs, font, color, r.synthesis);
                    }
                }
            }
            RenderTextNode
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
        pub fn strikethrough(
            child: impl UiNode,
            thickness: impl IntoVar<TextLineThickness>,
            style: impl IntoVar<LineStyle>,
        ) -> impl UiNode {
            let child = with_context_var(child, StrikethroughThicknessVar, thickness);
            with_context_var(child, StrikethroughStyleVar, style)
        }
        /// Sets the [`StrikethroughColorVar`].
        #[property(context, default(StrikethroughColorVar))]
        pub fn strikethrough_color(child: impl UiNode, color: impl IntoVar<TextLineColor>) -> impl UiNode {
            with_context_var(child, StrikethroughColorVar, color)
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
    text! { text; }
}

#[widget($crate::widgets::text_::strong)]
mod strong {
    use super::*;

    properties! {
        child {
            text(impl IntoVar<Text>);
        }
    }

    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let child = text::nodes::render_text();
        let child = text::nodes::layout_text(child, 0);
        let child = text::nodes::resolve_text(child, text);
        font_weight(child, FontWeight::BOLD)
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
        let child = text::nodes::render_text();
        let child = text::nodes::layout_text(child, 0);
        let child = text::nodes::resolve_text(child, text);
        font_style(child, FontStyle::Italic)
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
