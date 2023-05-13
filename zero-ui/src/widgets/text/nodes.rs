//! UI nodes used for building a text widget.

use std::{fmt, sync::Arc};

use atomic::{Atomic, Ordering};
use font_features::FontVariations;
use zero_ui_core::keyboard::Key;

use super::text_properties::*;
use crate::core::{
    focus::{FocusInfoBuilder, FOCUS, FOCUS_CHANGED_EVENT},
    keyboard::{CHAR_INPUT_EVENT, KEYBOARD, KEY_INPUT_EVENT},
    text::*,
    window::{WindowLoadingHandle, WINDOW_CTRL},
};
use crate::prelude::new_widget::*;

/// Represents the resolved fonts and the transformed, white space corrected and segmented text.
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

    /// Caret opacity.
    ///
    /// This variable is replaced often, the text resolver subscribes to it for
    /// [`UpdateOp::RenderUpdate`] automatically.
    pub caret_opacity: ReadOnlyArcVar<Factor>,

    /// Baseline set by `layout_text` during measure and used by `new_border` during arrange.
    baseline: Atomic<Px>,
}
impl Clone for ResolvedText {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            faces: self.faces.clone(),
            synthesis: self.synthesis,
            reshape: self.reshape,
            caret_opacity: self.caret_opacity.clone(),
            baseline: Atomic::new(self.baseline.load(Ordering::Relaxed)),
        }
    }
}
impl ResolvedText {
    fn no_context() -> Self {
        panic!("no `ResolvedText` in context")
    }

    /// Gets if the current code has resolved text in context.
    pub fn in_context() -> bool {
        !RESOLVED_TEXT.is_default()
    }

    /// Get the current contextual resolved text.
    ///
    /// # Panics
    ///
    /// Panics if requested out of context.
    pub fn get() -> Arc<ResolvedText> {
        RESOLVED_TEXT.get()
    }
}
impl fmt::Debug for ResolvedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedText")
            .field("text", &self.text)
            .field("faces", &self.faces)
            .field("synthesis", &self.synthesis)
            .field("reshape", &self.reshape)
            .field("caret_opacity", &self.caret_opacity.debug())
            .field("baseline", &self.baseline)
            .finish()
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
    fn no_context() -> Self {
        panic!("no `LayoutText` in context")
    }

    /// Gets if the current code has layout text in context.
    pub fn in_context() -> bool {
        !LAYOUT_TEXT.is_default()
    }

    /// Get the current contextual layout text.
    pub fn get() -> Arc<LayoutText> {
        LAYOUT_TEXT.get()
    }
}

context_local! {
    /// Represents the contextual [`ResolvedText`] setup by the [`resolve_text`] node.
    static RESOLVED_TEXT: ResolvedText = ResolvedText::no_context();
    /// Represents the contextual [`LayoutText`] setup by the [`layout_text`] node.
    static LAYOUT_TEXT: LayoutText  = LayoutText::no_context();
}

/// An UI node that resolves the text context vars, applies the text transform and white space correction and segments the `text`.
///
/// This node setups the [`ResolvedText`] for all inner nodes, the `Text!` widget includes this node in the [`NestGroup::EVENT`] group,
/// so all properties except [`NestGroup::CONTEXT`] have access using the [`ResolvedText::get`] function.
///
/// This node also subscribes to all the text context vars so other `Text!` properties don't need to.
pub fn resolve_text(child: impl UiNode, text: impl IntoVar<Txt>) -> impl UiNode {
    struct LoadingFontFaceList {
        _var_handle: VarHandle,
        result: ResponseVar<FontFaceList>,
        _loading: Option<WindowLoadingHandle>,
    }
    impl LoadingFontFaceList {
        fn new(face: ResponseVar<FontFaceList>) -> Self {
            Self {
                _var_handle: face.subscribe(UpdateOp::Update, WIDGET.id()),
                result: face,
                _loading: WINDOW_CTRL.loading_handle(1.secs()),
            }
        }
    }

    let text = text.into_var();
    let mut loading_faces = None;
    let mut resolved = None;
    let mut event_handles = EventHandles::default();
    let mut _caret_opacity_handle = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            // for r.text
            WIDGET
                .sub_var(&text)
                .sub_var(&TEXT_TRANSFORM_VAR)
                .sub_var(&WHITE_SPACE_VAR)
                .sub_var(&DIRECTION_VAR);
            // for r.font_face & r.synthesis
            WIDGET
                .sub_var(&FONT_FAMILY_VAR)
                .sub_var(&FONT_STYLE_VAR)
                .sub_var(&FONT_WEIGHT_VAR)
                .sub_var(&FONT_STRETCH_VAR)
                .sub_var(&FONT_SYNTHESIS_VAR)
                .sub_var(&LANG_VAR);
            // for editable mode
            WIDGET.sub_var(&TEXT_EDITABLE_VAR);

            let style = FONT_STYLE_VAR.get();
            let weight = FONT_WEIGHT_VAR.get();

            let f = FONT_FAMILY_VAR.with(|family| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

            let f = if f.is_done() {
                f.into_rsp().unwrap()
            } else {
                loading_faces = Some(LoadingFontFaceList::new(f));
                FontFaceList::empty()
            };

            let txt = text.get();
            let txt = TEXT_TRANSFORM_VAR.with(|t| t.transform(txt));
            let txt = WHITE_SPACE_VAR.with(|t| t.transform(txt));

            let editable = TEXT_EDITABLE_VAR.get();
            let caret_opacity = if editable && FOCUS.focused().get().map(|p| p.widget_id()) == Some(WIDGET.id()) {
                let v = KEYBOARD.caret_animation();
                _caret_opacity_handle = Some(v.subscribe(UpdateOp::Update, WIDGET.id()));
                v
            } else {
                var(0.fct()).read_only()
            };

            resolved = Some(ResolvedText {
                synthesis: FONT_SYNTHESIS_VAR.get() & f.best().synthesis_for(style, weight),
                faces: f,
                text: SegmentedText::new(txt, DIRECTION_VAR.get()),
                reshape: false,
                baseline: Atomic::new(Px(0)),
                caret_opacity,
            });

            if editable {
                event_handles.push(CHAR_INPUT_EVENT.subscribe(WIDGET.id()));
                event_handles.push(KEY_INPUT_EVENT.subscribe(WIDGET.id()));
                event_handles.push(FOCUS_CHANGED_EVENT.subscribe(WIDGET.id()));
            }

            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.init());
        }
        UiNodeOp::Deinit => {
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.deinit());
            event_handles.clear();
            _caret_opacity_handle = None;
            loading_faces = None;
            resolved = None;
        }
        UiNodeOp::Info { info } => {
            if TEXT_EDITABLE_VAR.get() {
                FocusInfoBuilder::new(info).focusable(true);
            }
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.info(info));
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = CHAR_INPUT_EVENT.on(update) {
                if !args.propagation().is_stopped() && text.capabilities().contains(VarCapabilities::MODIFY) && args.is_enabled(WIDGET.id())
                {
                    if (args.character == '\t' && !ACCEPTS_TAB_VAR.get()) || ("\r\n".contains(args.character) && !ACCEPTS_ENTER_VAR.get()) {
                        return;
                    }
                    args.propagation().stop();

                    let new_animation = KEYBOARD.caret_animation();
                    _caret_opacity_handle = Some(new_animation.subscribe(UpdateOp::Update, WIDGET.id()));
                    resolved.as_mut().unwrap().caret_opacity = new_animation;

                    if args.is_backspace() {
                        let _ = text.modify(move |t| {
                            if !t.as_ref().is_empty() {
                                t.to_mut().to_mut().pop();
                            }
                        });
                    } else if args.is_delete() {
                        let _ = text.modify(move |t| {
                            if !t.as_ref().is_empty() {
                                t.to_mut().to_mut().remove(0);
                            }
                        });
                    } else {
                        let c = args.character;
                        let _ = text.modify(move |t| {
                            t.to_mut().to_mut().push(c);
                        });
                    }
                }
            } else if let Some(args) = KEY_INPUT_EVENT.on(update) {
                if (args.key == Some(Key::Tab) && ACCEPTS_TAB_VAR.get()) || (args.key == Some(Key::Enter) && ACCEPTS_ENTER_VAR.get()) {
                    args.propagation().stop();
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if TEXT_EDITABLE_VAR.get() {
                    if args.is_focused(WIDGET.id()) {
                        let new_animation = KEYBOARD.caret_animation();
                        _caret_opacity_handle = Some(new_animation.subscribe(UpdateOp::RenderUpdate, WIDGET.id()));
                        resolved.as_mut().unwrap().caret_opacity = new_animation;
                    } else {
                        _caret_opacity_handle = None;
                        resolved.as_mut().unwrap().caret_opacity = var(0.fct()).read_only();
                    }
                }
            } else if let Some(_args) = FONT_CHANGED_EVENT.on(update) {
                // font query may return a different result.

                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR.with(|family| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

                if faces.is_done() {
                    let faces = faces.rsp().unwrap();

                    let r = resolved.as_mut().unwrap();

                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                        r.faces = faces;

                        r.reshape = true;
                        WIDGET.layout();
                    }
                } else {
                    loading_faces = Some(LoadingFontFaceList::new(faces));
                }
            }
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.event(update));
        }
        UiNodeOp::Update { updates } => {
            let r = resolved.as_mut().unwrap();

            // update `r.text`, affects layout.
            if text.is_new() || TEXT_TRANSFORM_VAR.is_new() || WHITE_SPACE_VAR.is_new() || DIRECTION_VAR.is_new() {
                let text = text.get();
                let text = TEXT_TRANSFORM_VAR.with(|t| t.transform(text));
                let text = WHITE_SPACE_VAR.with(|t| t.transform(text));
                let direction = DIRECTION_VAR.get();
                if r.text.text() != text || r.text.base_direction() != direction {
                    r.text = SegmentedText::new(text, direction);

                    r.reshape = true;
                    WIDGET.layout();
                }
            }

            // update `r.font_face`, affects layout
            if FONT_FAMILY_VAR.is_new()
                || FONT_STYLE_VAR.is_new()
                || FONT_STRETCH_VAR.is_new()
                || FONT_WEIGHT_VAR.is_new()
                || LANG_VAR.is_new()
            {
                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR.with(|family| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

                if faces.is_done() {
                    let faces = faces.rsp().unwrap();

                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                        r.faces = faces;

                        r.reshape = true;
                        WIDGET.layout();
                    }
                } else {
                    loading_faces = Some(LoadingFontFaceList::new(faces));
                }
            }
            // update `r.synthesis`, affects render
            if FONT_SYNTHESIS_VAR.is_new() || FONT_STYLE_VAR.is_new() || FONT_WEIGHT_VAR.is_new() {
                let synthesis = FONT_SYNTHESIS_VAR.get() & r.faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                if r.synthesis != synthesis {
                    r.synthesis = synthesis;
                    WIDGET.render();
                }
            }

            if let Some(enabled) = TEXT_EDITABLE_VAR.get_new() {
                if enabled && event_handles.0.is_empty() {
                    // actually enabled.

                    let id = WIDGET.id();
                    event_handles.push(CHAR_INPUT_EVENT.subscribe(id));
                    event_handles.push(KEY_INPUT_EVENT.subscribe(id));
                    event_handles.push(FOCUS_CHANGED_EVENT.subscribe(id));

                    if FOCUS.focused().get().map(|p| p.widget_id()) == Some(id) {
                        let new_animation = KEYBOARD.caret_animation();
                        _caret_opacity_handle = Some(new_animation.subscribe(UpdateOp::RenderUpdate, id));
                        r.caret_opacity = new_animation;
                    }
                } else {
                    event_handles.clear();
                    _caret_opacity_handle = None;
                    r.caret_opacity = var(0.fct()).read_only();
                }
            }

            if let Some(f) = loading_faces.take() {
                if f.result.is_done() {
                    let faces = f.result.into_rsp().unwrap();
                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                        r.faces = faces;

                        r.reshape = true;
                        WIDGET.layout();
                    }
                } else {
                    loading_faces = Some(f);
                }
            }

            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.update(updates));
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = RESOLVED_TEXT.with_context_opt(&mut resolved, || child.layout(wl));
            resolved.as_mut().unwrap().reshape = false;
        }
        op => {
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.op(op));
        }
    })
}

/// An UI node that layouts the parent [`ResolvedText`] defined by the text context vars.
///
/// This node setups the [`LayoutText`] for all inner nodes in the layout and render methods, the `Text!` widget includes this
/// node in the `NestGroup::CHILD_LAYOUT + 100` nest group, so all properties in [`NestGroup::CHILD_LAYOUT`] can affect the layout normally and
/// custom properties can be created to be inside this group and have access to the [`LayoutText::get`] function.
pub fn layout_text(child: impl UiNode) -> impl UiNode {
    bitflags::bitflags! {
        #[derive(Clone, Copy, PartialEq, Eq)]
        struct Layout: u8 {
            const UNDERLINE     = 0b0000_0001;
            const STRIKETHROUGH = 0b0000_0010;
            const OVERLINE      = 0b0000_0100;
            const RESHAPE_LINES = 0b0001_1111;
            const RESHAPE       = 0b0011_1111;
        }
    }
    struct FinalText {
        txt: Option<LayoutText>,
        shaping_args: TextShapingArgs,
        pending: Layout,

        txt_is_measured: bool,
        last_layout: (LayoutMetrics, Option<InlineConstraintsMeasure>),
    }
    impl FinalText {
        fn measure(&mut self, metrics: &LayoutMetrics) -> Option<PxSize> {
            if metrics.inline_constraints().is_some() {
                return None;
            }

            metrics.constraints().fill_or_exact()
        }

        fn layout(&mut self, metrics: &LayoutMetrics, t: &ResolvedText, is_measure: bool) -> PxSize {
            if t.reshape {
                self.pending.insert(Layout::RESHAPE);
            }

            let font_size = metrics.font_size();

            if self.txt.is_none() {
                let fonts = t.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
                self.txt = Some(LayoutText {
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
                self.pending.insert(Layout::RESHAPE);
            }

            let r = self.txt.as_mut().unwrap();

            if font_size != r.fonts.requested_size() || !r.fonts.is_sized_from(&t.faces) {
                r.fonts = t.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
                self.pending.insert(Layout::RESHAPE);
            }

            if TEXT_WRAP_VAR.get() && !metrics.constraints().x.is_unbounded() {
                let max_width = metrics.constraints().x.max().unwrap();
                if self.shaping_args.max_width != max_width {
                    self.shaping_args.max_width = max_width;

                    if !self.pending.contains(Layout::RESHAPE) && r.shaped_text.can_rewrap(max_width) {
                        self.pending.insert(Layout::RESHAPE);
                    }
                }
            } else if self.shaping_args.max_width != Px::MAX {
                self.shaping_args.max_width = Px::MAX;
                if !self.pending.contains(Layout::RESHAPE) && r.shaped_text.can_rewrap(Px::MAX) {
                    self.pending.insert(Layout::RESHAPE);
                }
            }

            if let Some(inline) = metrics.inline_constraints() {
                match inline {
                    InlineConstraints::Measure(m) => {
                        if self.shaping_args.inline_constraints != Some(m) {
                            self.shaping_args.inline_constraints = Some(m);
                            self.pending.insert(Layout::RESHAPE);
                        }
                    }
                    InlineConstraints::Layout(l) => {
                        if !self.pending.contains(Layout::RESHAPE)
                            && (Some(l.first_segs.len()) != r.shaped_text.first_line().map(|l| l.segs_len())
                                || Some(l.last_segs.len()) != r.shaped_text.last_line().map(|l| l.segs_len()))
                        {
                            self.pending.insert(Layout::RESHAPE);
                        }

                        if !self.pending.contains(Layout::RESHAPE_LINES)
                            && (r.shaped_text.mid_clear() != l.mid_clear
                                || r.shaped_text.first_line().map(|l| l.rect()) != Some(l.first)
                                || r.shaped_text.last_line().map(|l| l.rect()) != Some(l.last))
                        {
                            self.pending.insert(Layout::RESHAPE_LINES);
                        }
                    }
                }
            } else if self.shaping_args.inline_constraints.is_some() {
                self.shaping_args.inline_constraints = None;
                self.pending.insert(Layout::RESHAPE);
            }

            if !self.pending.contains(Layout::RESHAPE_LINES) {
                let size = r.shaped_text.size();
                if metrics.constraints().fill_size_or(size) != r.shaped_text.align_size() {
                    self.pending.insert(Layout::RESHAPE_LINES);
                }
            }

            let font = r.fonts.best();

            let space_len = font.space_x_advance();
            let dft_tab_len = space_len * 3;

            let (letter_spacing, word_spacing, tab_length) = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(space_len, space_len), || {
                    (
                        LETTER_SPACING_VAR.layout_x(),
                        WORD_SPACING_VAR.layout_x(),
                        TAB_LENGTH_VAR.layout_dft_x(dft_tab_len),
                    )
                })
            };

            let dft_line_height = font.metrics().line_height();
            let line_height = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(dft_line_height, dft_line_height), || {
                    LINE_HEIGHT_VAR.layout_dft_y(dft_line_height)
                })
            };
            let line_spacing =
                { LAYOUT.with_constraints(PxConstraints2d::new_exact(line_height, line_height), || LINE_SPACING_VAR.layout_y()) };

            if !self.pending.contains(Layout::RESHAPE)
                && (letter_spacing != self.shaping_args.letter_spacing
                    || word_spacing != self.shaping_args.word_spacing
                    || tab_length != self.shaping_args.tab_x_advance)
            {
                self.pending.insert(Layout::RESHAPE);
            }
            if !self.pending.contains(Layout::RESHAPE_LINES)
                && (line_spacing != self.shaping_args.line_spacing || line_height != self.shaping_args.line_height)
            {
                self.pending.insert(Layout::RESHAPE_LINES);
            }

            self.shaping_args.letter_spacing = letter_spacing;
            self.shaping_args.word_spacing = word_spacing;
            self.shaping_args.tab_x_advance = tab_length;
            self.shaping_args.line_height = line_height;
            self.shaping_args.line_spacing = line_spacing;

            let dft_thickness = font.metrics().underline_thickness;
            let (overline, strikethrough, underline) = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(line_height, line_height), || {
                    (
                        OVERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        STRIKETHROUGH_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                    )
                })
            };

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

            let align = TEXT_ALIGN_VAR.get();
            if !self.pending.contains(Layout::RESHAPE_LINES) && align != r.shaped_text.align() {
                self.pending.insert(Layout::RESHAPE_LINES);
            }

            /*
                APPLY
            */
            let prev_final_size = r.shaped_text.size();

            if self.pending.contains(Layout::RESHAPE) {
                r.shaped_text = r.fonts.shape_text(&t.text, &self.shaping_args);
                self.pending = self.pending.intersection(Layout::RESHAPE_LINES);
            }

            if !self.pending.contains(Layout::RESHAPE_LINES) && prev_final_size != metrics.constraints().fill_size_or(r.shaped_text.size())
            {
                self.pending.insert(Layout::RESHAPE_LINES);
            }

            if !is_measure {
                self.last_layout = (metrics.clone(), self.shaping_args.inline_constraints);

                if self.pending.contains(Layout::RESHAPE_LINES) {
                    r.shaped_text.reshape_lines(
                        metrics.constraints(),
                        metrics.inline_constraints().map(|c| c.layout()),
                        align,
                        line_height,
                        line_spacing,
                        metrics.direction(),
                    );
                    r.shaped_text_version = r.shaped_text_version.wrapping_add(1);
                    t.baseline.store(r.shaped_text.baseline(), Ordering::Relaxed);
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

                // self.pending is cleared in the node layout, after this method call
            }
            self.txt_is_measured = is_measure;

            metrics.constraints().fill_size_or(r.shaped_text.size())
        }

        fn ensure_layout_for_render(&mut self) {
            if self.txt_is_measured {
                let metrics = self.last_layout.0.clone();
                self.shaping_args.inline_constraints = self.last_layout.1;
                LAYOUT.with_context(metrics.clone(), || {
                    self.layout(&metrics, &RESOLVED_TEXT.get(), false);
                });

                debug_assert!(!self.txt_is_measured);
            }
        }

        fn with(&mut self, f: impl FnOnce()) {
            LAYOUT_TEXT.with_context_opt(&mut self.txt, f)
        }
    }

    let mut txt = FinalText {
        txt: None,
        shaping_args: TextShapingArgs::default(),
        pending: Layout::empty(),
        txt_is_measured: false,
        last_layout: (LayoutMetrics::new(1.fct(), PxSize::zero(), Px(0)), None),
    };

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&FONT_SIZE_VAR)
                .sub_var(&FONT_VARIATIONS_VAR)
                .sub_var(&LETTER_SPACING_VAR)
                .sub_var(&WORD_SPACING_VAR)
                .sub_var(&LINE_SPACING_VAR)
                .sub_var(&LINE_HEIGHT_VAR)
                .sub_var(&TAB_LENGTH_VAR);
            WIDGET
                .sub_var(&UNDERLINE_POSITION_VAR)
                .sub_var(&UNDERLINE_SKIP_VAR)
                .sub_var_layout(&OVERLINE_THICKNESS_VAR)
                .sub_var_layout(&STRIKETHROUGH_THICKNESS_VAR)
                .sub_var_layout(&UNDERLINE_THICKNESS_VAR);
            WIDGET
                .sub_var(&LINE_BREAK_VAR)
                .sub_var(&WORD_BREAK_VAR)
                .sub_var(&HYPHENS_VAR)
                .sub_var(&HYPHEN_CHAR_VAR)
                .sub_var(&TEXT_WRAP_VAR);

            WIDGET.sub_var(&FONT_FEATURES_VAR);
            // LANG_VAR already subscribed by `resolve_text`.

            txt.shaping_args.lang = LANG_VAR.get();
            txt.shaping_args.direction = txt.shaping_args.lang.character_direction().into();
            txt.shaping_args.line_break = LINE_BREAK_VAR.get();
            txt.shaping_args.word_break = WORD_BREAK_VAR.get();
            txt.shaping_args.hyphens = HYPHENS_VAR.get();
            txt.shaping_args.hyphen_char = HYPHEN_CHAR_VAR.get();
            txt.shaping_args.font_features = FONT_FEATURES_VAR.with(|f| f.finalize());
        }
        UiNodeOp::Deinit => {
            txt.txt = None;
        }
        UiNodeOp::Update { .. } => {
            if FONT_SIZE_VAR.is_new() || FONT_VARIATIONS_VAR.is_new() {
                txt.pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            }

            if LETTER_SPACING_VAR.is_new()
                || WORD_SPACING_VAR.is_new()
                || LINE_SPACING_VAR.is_new()
                || LINE_HEIGHT_VAR.is_new()
                || TAB_LENGTH_VAR.is_new()
                || LANG_VAR.is_new()
            {
                txt.shaping_args.lang = LANG_VAR.get();
                txt.shaping_args.direction = txt.shaping_args.lang.character_direction().into(); // will be set in layout too.
                txt.pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            }

            if UNDERLINE_POSITION_VAR.is_new() || UNDERLINE_SKIP_VAR.is_new() {
                txt.pending.insert(Layout::UNDERLINE);
                WIDGET.layout();
            }

            if let Some(lb) = LINE_BREAK_VAR.get_new() {
                if txt.shaping_args.line_break != lb {
                    txt.shaping_args.line_break = lb;
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(wb) = WORD_BREAK_VAR.get_new() {
                if txt.shaping_args.word_break != wb {
                    txt.shaping_args.word_break = wb;
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(h) = HYPHENS_VAR.get_new() {
                if txt.shaping_args.hyphens != h {
                    txt.shaping_args.hyphens = h;
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(c) = HYPHEN_CHAR_VAR.get_new() {
                txt.shaping_args.hyphen_char = c;
                if Hyphens::None != txt.shaping_args.hyphens {
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if TEXT_WRAP_VAR.is_new() {
                txt.pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            }

            FONT_FEATURES_VAR.with_new(|f| {
                txt.shaping_args.font_features = f.finalize();
                txt.pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            });
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();

            let metrics = LAYOUT.metrics();

            *desired_size = if let Some(size) = txt.measure(&metrics) {
                size
            } else {
                let size = txt.layout(&metrics, &RESOLVED_TEXT.get(), true);

                if let (Some(inline), Some(l)) = (wm.inline(), txt.txt.as_ref()) {
                    if let Some(first_line) = l.shaped_text.first_line() {
                        inline.first = first_line.original_size();
                        inline.with_first_segs(|i| {
                            for seg in first_line.segs() {
                                i.push(InlineSegment {
                                    width: seg.advance(),
                                    kind: seg.kind(),
                                });
                            }
                        });
                    } else {
                        inline.first = PxSize::zero();
                        inline.with_first_segs(|i| i.clear());
                    }

                    if l.shaped_text.lines_len() == 1 {
                        inline.last = inline.first;
                        inline.last_segs = inline.first_segs.clone();
                    } else if let Some(last_line) = l.shaped_text.last_line() {
                        inline.last = last_line.original_size();
                        inline.with_last_segs(|i| {
                            for seg in last_line.segs() {
                                i.push(InlineSegment {
                                    width: seg.advance(),
                                    kind: seg.kind(),
                                })
                            }
                        })
                    } else {
                        inline.last = PxSize::zero();
                        inline.with_last_segs(|i| i.clear());
                    }

                    inline.first_wrapped = l.shaped_text.first_wrapped();
                    inline.last_wrapped = l.shaped_text.lines_len() > 1;
                }
                size
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();

            let metrics = LAYOUT.metrics();
            let resolved_txt = RESOLVED_TEXT.get();
            *final_size = txt.layout(&metrics, &resolved_txt, false);

            if txt.pending != Layout::empty() {
                WIDGET.render();
                txt.pending = Layout::empty();
            }

            if let (Some(inline), Some(l)) = (wl.inline(), txt.txt.as_ref()) {
                let last_line = l.shaped_text.lines_len().saturating_sub(1);

                inline.first_segs.clear();
                inline.last_segs.clear();

                for (i, line) in l.shaped_text.lines().enumerate() {
                    if i == 0 {
                        let info = l.shaped_text.first_line().unwrap().segs().map(|s| s.inline_info());
                        if LAYOUT.direction().is_rtl() {
                            // help sort
                            inline.set_first_segs(info.rev());
                        } else {
                            inline.set_first_segs(info);
                        }
                    } else if i == last_line {
                        let info = l.shaped_text.last_line().unwrap().segs().map(|s| s.inline_info());
                        if LAYOUT.direction().is_rtl() {
                            // help sort
                            inline.set_last_segs(info.rev());
                        } else {
                            inline.set_last_segs(info);
                        }
                    }

                    inline.rows.push(line.rect());
                }
            }

            let baseline = resolved_txt.baseline.load(Ordering::Relaxed);
            wl.set_baseline(baseline);

            LAYOUT.with_constraints(PxConstraints2d::new_fill_size(*final_size), || {
                txt.with(|| {
                    let _ = child.layout(wl);
                })
            });
        }
        UiNodeOp::Render { frame } => {
            txt.ensure_layout_for_render();
            txt.with(|| child.render(frame))
        }
        UiNodeOp::RenderUpdate { update } => {
            txt.ensure_layout_for_render();
            txt.with(|| child.render_update(update))
        }
        _ => {}
    })
}

/// An Ui node that renders the default underline visual using the parent [`LayoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_strikethroughs`] node.
pub fn render_underlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&UNDERLINE_STYLE_VAR).sub_var_render(&UNDERLINE_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();

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
        }
        _ => {}
    })
}

/// An Ui node that renders the default strikethrough visual using the parent [`LayoutText`].
///
/// The lines are rendered after `child`, over it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_overlines`] node.
pub fn render_strikethroughs(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&STRIKETHROUGH_STYLE_VAR)
                .sub_var_render(&STRIKETHROUGH_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();
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
        }
        _ => {}
    })
}

/// An Ui node that renders the default overline visual using the parent [`LayoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_overlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&OVERLINE_STYLE_VAR).sub_var_render(&OVERLINE_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();
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
        }
        _ => {}
    })
}

/// An Ui node that renders the edit caret visual.
///
/// The caret is rendered after `child`, over it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_caret(child: impl UiNode) -> impl UiNode {
    let color_key = FrameValueKey::new_unique();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render_update(&CARET_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);

            if TEXT_EDITABLE_VAR.get() {
                let t = LayoutText::get();

                let mut c = CARET_COLOR_VAR.get();
                c.alpha = ResolvedText::get().caret_opacity.get().0;

                let mut clip_rect = PxRect::from_size(t.shaped_text.align_size());
                clip_rect.size.width = Dip::new(1).to_px(frame.scale_factor().0);
                clip_rect.size.height = t.shaped_text.line_height();

                frame.push_color(clip_rect, color_key.bind(c.into(), true));
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            child.render_update(update);

            let mut c = CARET_COLOR_VAR.get();
            c.alpha = ResolvedText::get().caret_opacity.get().0;

            if TEXT_EDITABLE_VAR.get() {
                update.update_color(color_key.update(c.into(), true))
            }
        }
        _ => {}
    })
}

/// An UI node that renders the parent [`LayoutText`].
///
/// This node renders the text only, decorators are rendered by other nodes.
///
/// This is the `Text!` widget inner most child node.
pub fn render_text() -> impl UiNode {
    #[derive(Clone, Copy, PartialEq)]
    struct RenderedText {
        version: u32,
        synthesis: FontSynthesis,
        color: Rgba,
        aa: FontAntiAliasing,
    }

    let mut reuse = None;
    let mut rendered = None;
    let mut color_key = None;

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            if TEXT_COLOR_VAR.capabilities().contains(VarCapabilities::NEW) {
                color_key = Some(FrameValueKey::new_unique());
            }
            // subscriptions are handled by the `resolve_text` node.
        }
        UiNodeOp::Deinit => {
            color_key = None;
            reuse = None;
            rendered = None;
        }
        UiNodeOp::Update { .. } => {
            if FONT_AA_VAR.is_new() {
                WIDGET.render();
            } else if TEXT_COLOR_VAR.is_new() {
                WIDGET.render_update();
            }
        }
        UiNodeOp::Render { frame } => {
            let r = ResolvedText::get();
            let t = LayoutText::get();

            let lh = t.shaped_text.line_height();
            let clip = PxRect::from_size(t.shaped_text.align_size()).inflate(lh, lh); // clip inflated to allow some weird glyphs
            let color = TEXT_COLOR_VAR.get();
            let color_value = if let Some(key) = color_key {
                key.bind(color.into(), TEXT_COLOR_VAR.is_animating())
            } else {
                FrameValue::Value(color.into())
            };

            let aa = FONT_AA_VAR.get();

            let rt = Some(RenderedText {
                version: t.shaped_text_version,
                synthesis: r.synthesis,
                color,
                aa,
            });
            if rendered != rt {
                rendered = rt;
                reuse = None;
            }

            frame.push_reuse(&mut reuse, |frame| {
                for (font, glyphs) in t.shaped_text.glyphs() {
                    frame.push_text(clip, glyphs, font, color_value, r.synthesis, aa);
                }
            });
        }
        UiNodeOp::RenderUpdate { update } => {
            if let Some(key) = color_key {
                let color = TEXT_COLOR_VAR.get();

                update.update_color(key.update(color.into(), TEXT_COLOR_VAR.is_animating()));

                let mut r = rendered.unwrap();
                r.color = color;
                rendered = Some(r);
            }
        }
        _ => {}
    })
}

/// Create a node that is sized one text line height by `width`.
///
/// This node can be used to reserve space for a full text in lazy loading contexts.
///
/// The contextual variables affect the layout size.
pub fn line_placeholder(width: impl IntoVar<Length>) -> impl UiNode {
    let child = layout_text(NilUiNode);
    let child = resolve_text(child, " ");
    crate::properties::width(child, width)
}
