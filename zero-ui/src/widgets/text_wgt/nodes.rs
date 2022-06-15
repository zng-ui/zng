//! UI nodes used for building a text widget.

use std::cell::{Cell, RefCell};

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

    /// Version updated every time the `shaped_text` text changes.
    pub shaped_text_version: u32,

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
        fn with<R>(&self, vars: &VarsRead, f: impl FnOnce(&C) -> R) -> R {
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

        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            TextContext::subscribe(ctx.vars, subs.var(ctx.vars, &self.text));
            self.with(ctx.vars, |c| c.subscriptions(ctx, subs))
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

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.with(ctx.vars, |c| c.measure(ctx))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.with_mut(ctx.vars, |c| c.layout(ctx, wl));
            self.resolved.as_mut().unwrap().reshape = false;
            size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.with(ctx.vars, |c| c.render(ctx, frame))
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.with(ctx.vars, |c| c.render_update(ctx, update))
        }
    }
    ResolveTextNode {
        child: child.cfg_boxed(),
        text: text.into_var(),
        resolved: None,
    }
    .cfg_boxed()
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
            const QUICK_RESHAPE = 0b0001_1111;
            const RESHAPE       = 0b0011_1111;
        }
    }
    struct FinalText {
        layout: Option<LayoutText>,
        shaping_args: TextShapingArgs,
    }
    impl FinalText {
        fn measure(&mut self, ctx: &mut MeasureContext) -> Option<PxSize> {
            ctx.constrains().fill_or_exact()
        }

        fn layout(
            &mut self,
            vars: &VarsRead,
            metrics: &LayoutMetrics,
            padding: &SideOffsets,
            t: &ResolvedText,
            pending: &mut Layout,
        ) -> PxSize {
            if t.reshape {
                pending.insert(Layout::RESHAPE);
            }

            let padding = padding.layout(metrics, |_| PxSideOffsets::zero());

            let (font_size, variations) = TextContext::font(vars);

            let font_size = {
                let m = metrics.clone().with_constrains(|c| c.with_less_y(padding.vertical()));
                font_size.layout(m.for_y(), |m| m.metrics.root_font_size())
            };

            if self.layout.is_none() {
                let fonts = t.faces.sized(font_size, variations.finalize());
                self.layout = Some(LayoutText {
                    shaped_text: ShapedText::new(fonts.best()),
                    shaped_text_version: 0,
                    fonts,
                    overlines: vec![],
                    overline_thickness: Px(0),
                    strikethroughs: vec![],
                    strikethrough_thickness: Px(0),
                    underlines: vec![],
                    underline_thickness: Px(0),
                });
                pending.insert(Layout::RESHAPE);
            }

            let r = self.layout.as_mut().unwrap();

            if font_size != r.fonts.requested_size() {
                r.fonts = t.faces.sized(font_size, variations.finalize());
                pending.insert(Layout::RESHAPE);
            }

            if !pending.contains(Layout::QUICK_RESHAPE) && r.shaped_text.padding() != padding {
                pending.insert(Layout::QUICK_RESHAPE);
            }

            let font = r.fonts.best();

            let (letter_spacing, word_spacing, line_spacing, line_height, tab_length, _) = TextContext::shaping(vars);
            let space_len = font.space_x_advance();
            let dft_tab_len = space_len * 3;

            let (letter_spacing, word_spacing, tab_length) = {
                let m = metrics.clone().with_constrains(|_| PxConstrains2d::new_exact(space_len, space_len));
                (
                    letter_spacing.layout(m.for_x(), |_| Px(0)),
                    word_spacing.layout(m.for_x(), |_| Px(0)),
                    tab_length.layout(m.for_x(), |_| dft_tab_len),
                )
            };

            let dft_line_height = font.metrics().line_height();
            let line_height = {
                let m = metrics
                    .clone()
                    .with_constrains(|_| PxConstrains2d::new_exact(dft_line_height, dft_line_height));
                line_height.layout(m.for_y(), |_| dft_line_height)
            };
            let line_spacing = {
                let m = metrics
                    .clone()
                    .with_constrains(|_| PxConstrains2d::new_exact(line_height, line_height));
                line_spacing.layout(m.for_y(), |_| Px(0))
            };

            if !pending.contains(Layout::RESHAPE)
                && (letter_spacing != self.shaping_args.letter_spacing
                    || word_spacing != self.shaping_args.word_spacing
                    || tab_length != self.shaping_args.tab_x_advance)
            {
                pending.insert(Layout::RESHAPE);
            }
            if !pending.contains(Layout::QUICK_RESHAPE)
                && (line_spacing != self.shaping_args.line_spacing || line_height != self.shaping_args.line_height)
            {
                pending.insert(Layout::QUICK_RESHAPE);
            }

            self.shaping_args.letter_spacing = letter_spacing;
            self.shaping_args.word_spacing = word_spacing;
            self.shaping_args.tab_x_advance = tab_length;
            self.shaping_args.line_height = line_height;
            self.shaping_args.line_spacing = line_spacing;

            let dft_thickness = font.metrics().underline_thickness;
            let (overline, strikethrough, underline) = {
                let m = metrics
                    .clone()
                    .with_constrains(|_| PxConstrains2d::new_exact(line_height, line_height));
                (
                    OverlineThicknessVar::get(vars).layout(m.for_y(), |_| dft_thickness),
                    StrikethroughThicknessVar::get(vars).layout(m.for_y(), |_| dft_thickness),
                    UnderlineThicknessVar::get(vars).layout(m.for_y(), |_| dft_thickness),
                )
            };

            if !pending.contains(Layout::OVERLINE) && (r.overline_thickness == Px(0) && overline > Px(0)) {
                pending.insert(Layout::OVERLINE);
            }
            if !pending.contains(Layout::STRIKETHROUGH) && (r.strikethrough_thickness == Px(0) && strikethrough > Px(0)) {
                pending.insert(Layout::STRIKETHROUGH);
            }
            if !pending.contains(Layout::UNDERLINE) && (r.underline_thickness == Px(0) && underline > Px(0)) {
                pending.insert(Layout::UNDERLINE);
            }
            r.overline_thickness = overline;
            r.strikethrough_thickness = strikethrough;
            r.underline_thickness = underline;

            let align = *TextAlignVar::get(vars);
            if !pending.contains(Layout::QUICK_RESHAPE) && align != r.shaped_text.align() {
                pending.insert(Layout::QUICK_RESHAPE);
            }

            /*
                APPLY
            */
            let prev_final_size = r.shaped_text.box_size();

            if pending.contains(Layout::RESHAPE) {
                r.shaped_text = r.fonts.shape_text(&t.text, &self.shaping_args);
            }

            if !pending.contains(Layout::QUICK_RESHAPE) && prev_final_size != metrics.constrains().fill_size_or(r.shaped_text.box_size()) {
                pending.insert(Layout::QUICK_RESHAPE);
                r.shaped_text_version = r.shaped_text_version.wrapping_add(1);
            }

            if pending.contains(Layout::QUICK_RESHAPE) {
                r.shaped_text.reshape(
                    padding,
                    line_height,
                    line_spacing,
                    |size| PxRect::from_size(metrics.constrains().fill_size_or(size)),
                    align,
                );

                let baseline = r.shaped_text.box_baseline() + padding.bottom;
                t.baseline.set(baseline);
            }
            if pending.contains(Layout::OVERLINE) {
                if r.overline_thickness > Px(0) {
                    r.overlines = r.shaped_text.lines().map(|l| l.overline()).collect();
                } else {
                    r.overlines = vec![];
                }
            }
            if pending.contains(Layout::STRIKETHROUGH) {
                if r.strikethrough_thickness > Px(0) {
                    r.strikethroughs = r.shaped_text.lines().map(|l| l.strikethrough()).collect();
                } else {
                    r.strikethroughs = vec![];
                }
            }
            if pending.contains(Layout::UNDERLINE) {
                if r.underline_thickness > Px(0) {
                    let skip = *UnderlineSkipVar::get(vars);
                    match *UnderlinePositionVar::get(vars) {
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

            r.shaped_text.align_box().size
        }
    }
    struct LayoutTextNode<C, P> {
        child: C,
        padding: P,
        txt: RefCell<FinalText>,
        pending: Layout,
    }
    impl<C: UiNode, P> LayoutTextNode<C, P> {
        fn with_mut<R>(&mut self, vars: &Vars, f: impl FnOnce(&mut C) -> R) -> R {
            let txt = self.txt.borrow();
            vars.with_context_var(LayoutTextVar, ContextVarData::fixed(&txt.layout), || f(&mut self.child))
        }
        fn with(&self, vars: &VarsRead, f: impl FnOnce(&C)) {
            let txt = self.txt.borrow();
            vars.with_context_var(LayoutTextVar, ContextVarData::fixed(&txt.layout), || f(&self.child))
        }
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, P: Var<SideOffsets>> UiNode for LayoutTextNode<C, P> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.var(ctx, &self.padding);
            // other subscriptions are handled by the `resolve_text` node.

            self.child.subscriptions(ctx, subs)
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.txt.get_mut().layout = None;
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if TextContext::font_update(ctx).is_some() {
                self.pending.insert(Layout::RESHAPE);
                ctx.updates.layout();
            }

            if let Some((_, _, _, _, _, lang)) = TextContext::shaping_update(ctx.vars) {
                self.txt.get_mut().shaping_args.lang = lang.clone();
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

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let mut txt = self.txt.borrow_mut();

            if let Some(size) = txt.measure(ctx) {
                size
            } else {
                let t = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `measure`");
                let mut pending = self.pending;
                self.txt
                    .borrow_mut()
                    .layout(ctx.vars, ctx.metrics, self.padding.get(ctx.vars), t, &mut pending)
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let t = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `layout`");

            let size = self
                .txt
                .get_mut()
                .layout(ctx.vars, ctx.metrics, self.padding.get(ctx.vars), t, &mut self.pending);

            if self.pending != Layout::empty() {
                ctx.updates.render();
                self.pending = Layout::empty();
            }

            wl.set_baseline(t.baseline.get());

            ctx.with_constrains(
                |c| c.with_max_size(size).with_fill(true, true),
                |ctx| {
                    self.with_mut(ctx.vars, |c| c.layout(ctx, wl));
                },
            );

            size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.with(ctx.vars, |c| c.render(ctx, frame))
        }
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.with(ctx.vars, |c| c.render_update(ctx, update))
        }
    }
    LayoutTextNode {
        child: child.cfg_boxed(),
        padding: padding.into_var(),
        txt: RefCell::new(FinalText {
            layout: None,
            shaping_args: TextShapingArgs::default(),
        }),
        pending: Layout::empty(),
    }
    .cfg_boxed()
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
    RenderUnderlineNode { child: child.cfg_boxed() }.cfg_boxed()
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
    RenderStrikethroughsNode { child: child.cfg_boxed() }.cfg_boxed()
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
    RenderOverlineNode { child: child.cfg_boxed() }.cfg_boxed()
}

/// An UI node that renders the parent [`LayoutText`].
///
/// This node renders the text only, decorators are rendered by other nodes.
///
/// This is the `text!` widget inner most leaf node, introduced in the `new_child` constructor.
pub fn render_text() -> impl UiNode {
    #[derive(Clone, Copy, PartialEq)]
    struct RenderedText {
        version: u32,
        synthesis: FontSynthesis,
        color: Rgba,
        aa: FontAntiAliasing,
    }
    struct RenderTextNode {
        reuse: RefCell<ReuseGroup>,
        rendered: Cell<Option<RenderedText>>,
    }
    #[impl_ui_node(none)]
    impl UiNode for RenderTextNode {
        // subscriptions are handled by the `resolve_text` node.
        fn update(&mut self, ctx: &mut WidgetContext) {
            if TextColorVar::is_new(ctx) || FontAaVar::is_new(ctx) {
                ctx.updates.render();
            }
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            let r = ResolvedText::get(ctx.vars).expect("expected `ResolvedText` in `render_text`");
            let t = LayoutText::get(ctx.vars).expect("expected `LayoutText` in `render_text`");

            let clip = t.shaped_text.align_box();
            let color = *TextColorVar::get(ctx.vars);

            let aa = *FontAaVar::get(ctx.vars);

            let mut reuse = self.reuse.borrow_mut();

            let rendered = Some(RenderedText {
                version: t.shaped_text_version,
                synthesis: r.synthesis,
                color,
                aa,
            });
            if self.rendered.get() != rendered {
                self.rendered.set(rendered);
                reuse.clear();
            }

            frame.push_reuse_group(&mut reuse, |frame| {
                for (font, glyphs) in t.shaped_text.glyphs() {
                    frame.push_text(clip, glyphs, font, color.into(), r.synthesis, aa);
                }
            });
        }
    }
    RenderTextNode {
        reuse: RefCell::default(),
        rendered: Cell::new(None),
    }
}
