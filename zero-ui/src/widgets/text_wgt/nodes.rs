//! UI nodes used for building a text widget.

use std::cell::{Cell, RefCell};

use font_features::FontVariations;

use super::properties::*;
use crate::core::{focus::FocusInfoBuilder, keyboard::CHAR_INPUT_EVENT, text::*};
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

    /// If the `text` or `faces` has updated, this value is `true` in the update the value changed and stays `true`
    /// until after layout.
    pub reshape: bool,

    /// Baseline set by `layout_text` during measure and used by `new_border` during arrange.
    baseline: Cell<Px>,
}
impl ResolvedText {
    /// If any [`ResolvedText`] is set in the current context.
    ///
    /// This is `true` for any property with priority `event` or up set in a `text!` widget or
    /// any widget that uses the [`resolve_text`] node.
    pub fn is_some() -> bool {
        RESOLVED_TEXT_VAR.with(Option::is_some)
    }

    /// Calls `f` if [`is_some`], returns the result of `f`.
    ///
    /// [`is_some`]: Self::is_some
    pub fn with<R>(f: impl FnOnce(&Self) -> R) -> Option<R> {
        RESOLVED_TEXT_VAR.with(|opt| opt.as_ref().map(f))
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

    /// Computed [`OVERLINE_THICKNESS_VAR`].
    pub overline_thickness: Px,

    /// List of strikethrough segments, defining origin and width of each line.
    ///
    /// Note that strikethroughs are only computed if the `strikethrough_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_strikethroughs`].
    pub strikethroughs: Vec<(PxPoint, Px)>,
    /// Computed [`STRIKETHROUGH_THICKNESS_VAR`].
    pub strikethrough_thickness: Px,

    /// List of underline segments, defining origin and width of each line.
    ///
    /// Note that underlines are only computed if the `underline_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_underlines`].
    ///
    /// Note that underlines trop down from these lines.
    pub underlines: Vec<(PxPoint, Px)>,
    /// Computed [`UNDERLINE_THICKNESS_VAR`].
    pub underline_thickness: Px,
}
impl LayoutText {
    /// If any [`ResolvedText`] is set in the current context.
    ///
    /// This is `true` only during layout & render, in properties with priority `border` or `fill` set in a `text!` widget or
    /// any widget that uses the [`layout_text`] node.
    pub fn is_some() -> bool {
        LAYOUT_TEXT_VAR.with(Option::is_some)
    }

    /// Calls `f` if [`is_some`], returns the result of `f`.
    ///
    /// [`is_some`]: Self::is_some
    pub fn with<R>(f: impl FnOnce(&Self) -> R) -> Option<R> {
        LAYOUT_TEXT_VAR.with(|opt| opt.as_ref().map(f))
    }
}

context_var! {
    /// Represents the contextual [`ResolvedText`] setup by the [`resolve_text`] node.
    static RESOLVED_TEXT_VAR: Option<ResolvedText> = None;
    /// Represents the contextual [`LayoutText`] setup by the [`layout_text`] node.
    static LAYOUT_TEXT_VAR: Option<LayoutText> = None;
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
        resolved: RefCell<Option<ResolvedText>>,
        char_input_handle: Option<EventWidgetHandle>,
    }
    impl<C: UiNode, T> ResolveTextNode<C, T> {
        fn with_mut<R>(&mut self, f: impl FnOnce(&mut C) -> R) -> R {
            // !! TODO, review this, we alloc a new box for every call, in previous API there was not alloc.
            let (resolved, r) = RESOLVED_TEXT_VAR.with_context(self.resolved.get_mut().take(), || f(&mut self.child));
            *self.resolved.get_mut() = resolved.into_value();
            r
        }
        fn with<R>(&self, f: impl FnOnce(&C) -> R) -> R {
            let (resolved, r) = RESOLVED_TEXT_VAR.with_context(self.resolved.borrow_mut().take(), || f(&self.child));
            *self.resolved.borrow_mut() = resolved.into_value();
            r
        }
    }
    impl<C: UiNode, T: Var<Text>> UiNode for ResolveTextNode<C, T> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            // !!: TextContext::subscribe(ctx.vars, subs.var(ctx.vars, &self.text));

            let style = FONT_STYLE_VAR.get();
            let weight = FONT_WEIGHT_VAR.get();

            let faces = FONT_FAMILY_VAR
                .with(|family| Fonts::req(ctx.services).list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

            let text = self.text.get();
            let text = TEXT_TRANSFORM_VAR.with(|t| t.transform(text));
            let text = WHITE_SPACE_VAR.with(|t| t.transform(text));

            *self.resolved.get_mut() = Some(ResolvedText {
                synthesis: FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight),
                faces,
                text: SegmentedText::new(text),
                reshape: false,
                baseline: Cell::new(Px(0)),
            });

            if TEXT_EDITABLE_VAR.get() {
                self.char_input_handle = Some(CHAR_INPUT_EVENT.subscribe(ctx.path.widget_id()));
            }

            self.with_mut(|c| c.init(ctx))
        }

        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            if TEXT_EDITABLE_VAR.get() {
                FocusInfoBuilder::get(info).focusable(true);
            }
            self.with(|c| c.info(ctx, info))
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.char_input_handle = None;
            self.with_mut(|c| c.deinit(ctx));
            *self.resolved.get_mut() = None;
        }

        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = CHAR_INPUT_EVENT.on(update) {
                if !args.propagation().is_stopped()
                    && self.text.capabilities().contains(VarCapabilities::MODIFY)
                    && args.is_enabled(ctx.path.widget_id())
                {
                    args.propagation().stop();

                    if args.is_backspace() {
                        let _ = self.text.modify(ctx.vars, move |t| {
                            if !t.get().is_empty() {
                                t.get_mut().to_mut().pop();
                            }
                        });
                    } else {
                        let c = args.character;
                        let _ = self.text.modify(ctx.vars, move |t| {
                            t.get_mut().to_mut().push(c);
                        });
                    }
                }
            } else if let Some(_args) = FONT_CHANGED_EVENT.on(update) {
                // font query may return a different result.

                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR
                    .with(|family| Fonts::req(ctx.services).list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

                let r = self.resolved.get_mut().as_mut().unwrap();

                if r.faces != faces {
                    r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                    r.faces = faces;

                    r.reshape = true;
                    ctx.updates.layout();
                }
            }
            self.with_mut(|c| c.event(ctx, update))
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            let r = self.resolved.get_mut().as_mut().unwrap();

            // update `r.text`, affects layout.
            if self.text.is_new(ctx) || TEXT_TRANSFORM_VAR.is_new(ctx) || WHITE_SPACE_VAR.is_new(ctx) {
                let text = self.text.get();
                let text = TEXT_TRANSFORM_VAR.with(|t| t.transform(text));
                let text = WHITE_SPACE_VAR.with(|t| t.transform(text));
                if r.text.text() != text {
                    r.text = SegmentedText::new(text);

                    r.reshape = true;
                    ctx.updates.layout();
                }
            }

            // update `r.font_face`, affects layout
            if FONT_FAMILY_VAR.is_new(ctx)
                || FONT_STYLE_VAR.is_new(ctx)
                || FONT_STRETCH_VAR.is_new(ctx)
                || FONT_WEIGHT_VAR.is_new(ctx)
                || LANG_VAR.is_new(ctx)
            {
                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR
                    .with(|family| Fonts::req(ctx.services).list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

                if r.faces != faces {
                    r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                    r.faces = faces;

                    r.reshape = true;
                    ctx.updates.layout();
                }
            }

            // update `r.synthesis`, affects render
            if FONT_SYNTHESIS_VAR.is_new(ctx) || FONT_STYLE_VAR.is_new(ctx) || FONT_WEIGHT_VAR.is_new(ctx) {
                let synthesis = FONT_SYNTHESIS_VAR.get() & r.faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                if r.synthesis != synthesis {
                    r.synthesis = synthesis;
                    ctx.updates.render();
                }
            }
            if let Some(enabled) = TEXT_EDITABLE_VAR.get_new(ctx) {
                if enabled && self.char_input_handle.is_none() {
                    self.char_input_handle = Some(CHAR_INPUT_EVENT.subscribe(ctx.path.widget_id()));
                } else {
                    self.char_input_handle = None;
                }
            }

            self.with_mut(|c| c.update(ctx, updates))
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            self.with(|c| c.measure(ctx))
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            let size = self.with_mut(|c| c.layout(ctx, wl));
            self.resolved.get_mut().as_mut().unwrap().reshape = false;
            size
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.with(|c| c.render(ctx, frame))
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.with(|c| c.render_update(ctx, update))
        }
    }
    ResolveTextNode {
        child: child.cfg_boxed(),
        text: text.into_var(),
        resolved: RefCell::new(None),
        char_input_handle: None,
    }
    .cfg_boxed()
}

/// An UI node that layouts the parent [`ResolvedText`] according with the [`TextContext`].
///
/// This node setups the [`LayoutText`] for all inner nodes in the layout and render methods, the `text!` widget introduces this
/// node at the `new_fill` constructor, so all properties with priority `fill` have access to the [`LayoutText::get`] function.
pub fn layout_text(child: impl UiNode) -> impl UiNode {
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

        fn layout(&mut self, metrics: &LayoutMetrics, t: &ResolvedText, pending: &mut Layout) -> PxSize {
            if t.reshape {
                pending.insert(Layout::RESHAPE);
            }

            let padding = TEXT_PADDING_VAR.get().layout(metrics, |_| PxSideOffsets::zero());

            let font_size = {
                let m = metrics.clone().with_constrains(|c| c.with_less_y(padding.vertical()));
                FONT_SIZE_VAR.get().layout(m.for_y(), |m| m.metrics.root_font_size())
            };

            if self.layout.is_none() {
                let fonts = t.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
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

            if font_size != r.fonts.requested_size() || !r.fonts.is_sized_from(&t.faces) {
                r.fonts = t.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
                pending.insert(Layout::RESHAPE);
            }

            if !pending.contains(Layout::QUICK_RESHAPE) && r.shaped_text.padding() != padding {
                pending.insert(Layout::QUICK_RESHAPE);
            }

            let font = r.fonts.best();

            let space_len = font.space_x_advance();
            let dft_tab_len = space_len * 3;

            let (letter_spacing, word_spacing, tab_length) = {
                let m = metrics.clone().with_constrains(|_| PxConstrains2d::new_exact(space_len, space_len));
                (
                    LETTER_SPACING_VAR.get().layout(m.for_x(), |_| Px(0)),
                    WORD_SPACING_VAR.get().layout(m.for_x(), |_| Px(0)),
                    TAB_LENGTH_VAR.get().layout(m.for_x(), |_| dft_tab_len),
                )
            };

            let dft_line_height = font.metrics().line_height();
            let line_height = {
                let m = metrics
                    .clone()
                    .with_constrains(|_| PxConstrains2d::new_exact(dft_line_height, dft_line_height));
                LINE_HEIGHT_VAR.get().layout(m.for_y(), |_| dft_line_height)
            };
            let line_spacing = {
                let m = metrics
                    .clone()
                    .with_constrains(|_| PxConstrains2d::new_exact(line_height, line_height));
                LINE_SPACING_VAR.get().layout(m.for_y(), |_| Px(0))
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
                    OVERLINE_THICKNESS_VAR.get().layout(m.for_y(), |_| dft_thickness),
                    STRIKETHROUGH_THICKNESS_VAR.get().layout(m.for_y(), |_| dft_thickness),
                    UNDERLINE_THICKNESS_VAR.get().layout(m.for_y(), |_| dft_thickness),
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

            let align = TEXT_ALIGN_VAR.get();
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
            }

            if pending.contains(Layout::QUICK_RESHAPE) {
                r.shaped_text.reshape(
                    padding,
                    line_height,
                    line_spacing,
                    |size| PxRect::from_size(metrics.constrains().fill_size_or(size)),
                    align,
                );
                r.shaped_text_version = r.shaped_text_version.wrapping_add(1);

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
                    let skip = UNDERLINE_SKIP_VAR.get();
                    match UNDERLINE_POSITION_VAR.get() {
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
    struct LayoutTextNode<C> {
        child: C,
        txt: RefCell<FinalText>,
        pending: Layout,
    }
    impl<C: UiNode> LayoutTextNode<C> {
        fn with_mut<R>(&mut self, f: impl FnOnce(&mut C) -> R) -> R {
            let txt = self.txt.get_mut();
            let (layout, r) = LAYOUT_TEXT_VAR.with_context(txt.layout.take(), || f(&mut self.child));
            txt.layout = layout.into_value();
            r
        }
        fn with(&self, f: impl FnOnce(&C)) {
            let mut txt = self.txt.borrow_mut();
            let (layout, ()) = LAYOUT_TEXT_VAR.with_context(txt.layout.take(), || f(&self.child));
            txt.layout = layout.into_value();
        }
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for LayoutTextNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.sub_var(&TEXT_PADDING_VAR);
            // other subscriptions are handled by the `resolve_text` node.

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.txt.get_mut().layout = None;
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if FONT_SIZE_VAR.is_new(ctx) || FONT_VARIATIONS_VAR.is_new(ctx) {
                self.pending.insert(Layout::RESHAPE);
                ctx.updates.layout();
            }

            if LETTER_SPACING_VAR.is_new(ctx)
                || WORD_SPACING_VAR.is_new(ctx)
                || LINE_SPACING_VAR.is_new(ctx)
                || LINE_HEIGHT_VAR.is_new(ctx)
                || TAB_LENGTH_VAR.is_new(ctx)
                || LANG_VAR.is_new(ctx)
            {
                self.txt.get_mut().shaping_args.lang = LANG_VAR.get();
                self.pending.insert(Layout::RESHAPE);
                ctx.updates.layout();
            }

            if UNDERLINE_POSITION_VAR.is_new(ctx) || UNDERLINE_SKIP_VAR.is_new(ctx) {
                self.pending.insert(Layout::UNDERLINE);
                ctx.updates.layout();
            }

            if OVERLINE_THICKNESS_VAR.is_new(ctx)
                || STRIKETHROUGH_THICKNESS_VAR.is_new(ctx)
                || UNDERLINE_THICKNESS_VAR.is_new(ctx)
                || TEXT_PADDING_VAR.is_new(ctx)
            {
                ctx.updates.layout();
            }

            self.child.update(ctx, updates);
        }

        fn measure(&self, ctx: &mut MeasureContext) -> PxSize {
            let mut txt = self.txt.borrow_mut();

            if let Some(size) = txt.measure(ctx) {
                size
            } else {
                ResolvedText::with(|t| {
                    let mut pending = self.pending;
                    txt.layout(ctx.metrics, t, &mut pending)
                })
                .expect("expected `ResolvedText` in `measure`")
            }
        }
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize {
            ResolvedText::with(|t| {
                let size = self.txt.get_mut().layout(ctx.metrics, t, &mut self.pending);

                if self.pending != Layout::empty() {
                    ctx.updates.render();
                    self.pending = Layout::empty();
                }

                wl.set_baseline(t.baseline.get());

                ctx.with_constrains(
                    |_| PxConstrains2d::new_fill_size(size),
                    |ctx| {
                        self.with_mut(|c| c.layout(ctx, wl));
                    },
                );

                size
            })
            .expect("expected `ResolvedText` in `layout`")
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.with(|c| c.render(ctx, frame))
        }
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.with(|c| c.render_update(ctx, update))
        }
    }
    LayoutTextNode {
        child: child.cfg_boxed(),
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
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if UNDERLINE_STYLE_VAR.is_new(ctx) || UNDERLINE_COLOR_VAR.is_new(ctx) {
                ctx.updates.render();
            }

            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            LayoutText::with(|t| {
                if !t.underlines.is_empty() {
                    let style = UNDERLINE_STYLE_VAR.get();
                    if style != LineStyle::Hidden {
                        let color = UNDERLINE_COLOR_VAR.get().into();
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
            })
            .expect("expected `LayoutText` in `render_underlines`");

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
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if STRIKETHROUGH_STYLE_VAR.is_new(ctx) || STRIKETHROUGH_COLOR_VAR.is_new(ctx) {
                ctx.updates.render();
            }

            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            LayoutText::with(|t| {
                if !t.strikethroughs.is_empty() {
                    let style = STRIKETHROUGH_STYLE_VAR.get();
                    if style != LineStyle::Hidden {
                        let color = STRIKETHROUGH_COLOR_VAR.get().into();
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
            })
            .expect("expected `LayoutText` in `render_strikethroughs`");

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
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if OVERLINE_STYLE_VAR.is_new(ctx) || OVERLINE_COLOR_VAR.is_new(ctx) {
                ctx.updates.render();
            }

            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            LayoutText::with(|t| {
                if !t.overlines.is_empty() {
                    let style = OVERLINE_STYLE_VAR.get();
                    if style != LineStyle::Hidden {
                        let color = OVERLINE_COLOR_VAR.get().into();
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
            })
            .expect("expected `LayoutText` in `render_overlines`");

            self.child.render(ctx, frame);
        }
    }
    RenderOverlineNode { child: child.cfg_boxed() }.cfg_boxed()
}

/// An Ui node that renders the edit caret visual.
///
/// The caret is rendered after `child`, over it.
///
/// The `text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_caret(child: impl UiNode) -> impl UiNode {
    struct RenderCaretNode<C> {
        child: C,
        color_key: FrameValueKey<RenderColor>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for RenderCaretNode<C> {
        // subscriptions are handled by the `editable` property node.
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if TEXT_EDITABLE_VAR.is_new(ctx) || CARET_COLOR_VAR.is_new(ctx) {
                ctx.updates.render();
            }

            self.child.update(ctx, updates);
        }

        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.child.render(ctx, frame);

            if TEXT_EDITABLE_VAR.get() {
                LayoutText::with(|t| {
                    let mut clip_rect = t.shaped_text.align_box();
                    clip_rect.size.width = Dip::new(1).to_px(frame.scale_factor().0);
                    clip_rect.size.height = t.shaped_text.line_height();

                    let padding = t.shaped_text.padding();
                    clip_rect.origin.x += padding.left;
                    clip_rect.origin.y += padding.top;

                    let color = CARET_COLOR_VAR.get().into();
                    frame.push_color(clip_rect, self.color_key.bind(color, true));
                })
                .expect("expected `LayoutText` in `render_text`");
            }
        }

        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.child.render_update(ctx, update);

            if TEXT_EDITABLE_VAR.get() {
                let color = CARET_COLOR_VAR.get().into();
                update.update_color(self.color_key.update(color, true))
            }
        }
    }
    RenderCaretNode {
        child,
        color_key: FrameValueKey::new_unique(),
    }
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
        reuse: RefCell<Option<ReuseRange>>,
        rendered: Cell<Option<RenderedText>>,
        color_key: Option<FrameValueKey<RenderColor>>,
    }
    #[impl_ui_node(none)]
    impl UiNode for RenderTextNode {
        fn init(&mut self, _: &mut WidgetContext) {
            if TEXT_COLOR_VAR.capabilities().contains(VarCapabilities::CHANGE) {
                self.color_key = Some(FrameValueKey::new_unique());
            }
        }

        fn deinit(&mut self, _: &mut WidgetContext) {
            self.color_key = None;
        }

        // subscriptions are handled by the `resolve_text` node.
        fn update(&mut self, ctx: &mut WidgetContext, _: &mut WidgetUpdates) {
            if FONT_AA_VAR.is_new(ctx) {
                ctx.updates.render();
            } else if TEXT_COLOR_VAR.is_new(ctx) {
                ctx.updates.render_update();
            }
        }

        fn render(&self, _: &mut RenderContext, frame: &mut FrameBuilder) {
            let mut render = move |r: &ResolvedText, t: &LayoutText| {
                let clip = t.shaped_text.align_box();
                let color = TEXT_COLOR_VAR.get();
                let color_value = if let Some(key) = self.color_key {
                    key.bind(color.into(), TEXT_COLOR_VAR.is_animating())
                } else {
                    FrameValue::Value(color.into())
                };

                let aa = FONT_AA_VAR.get();

                let mut reuse = self.reuse.borrow_mut();

                let rendered = Some(RenderedText {
                    version: t.shaped_text_version,
                    synthesis: r.synthesis,
                    color,
                    aa,
                });
                if self.rendered.get() != rendered {
                    self.rendered.set(rendered);
                    *reuse = None;
                }

                frame.push_reuse(&mut reuse, |frame| {
                    for (font, glyphs) in t.shaped_text.glyphs() {
                        frame.push_text(clip, glyphs, font, color_value, r.synthesis, aa);
                    }
                });
            };

            ResolvedText::with(move |r| LayoutText::with(move |t| render(r, t)).expect("expected `LayoutText` in `render_text`"))
                .expect("expected `ResolvedText` in `render_text`");
        }

        fn render_update(&self, _: &mut RenderContext, update: &mut FrameUpdate) {
            if let Some(key) = self.color_key {
                let color = TEXT_COLOR_VAR.get();

                update.update_color(key.update(color.into(), TEXT_COLOR_VAR.is_animating()));

                let mut rendered = self.rendered.get().unwrap();
                rendered.color = color;
                self.rendered.set(Some(rendered));
            }
        }
    }
    RenderTextNode {
        reuse: RefCell::default(),
        rendered: Cell::new(None),
        color_key: None,
    }
}
