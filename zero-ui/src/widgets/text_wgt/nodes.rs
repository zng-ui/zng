//! UI nodes used for building a text widget.

use std::{
    cell::{Cell, RefCell},
    fmt,
    sync::Arc,
};

use atomic::{Atomic, Ordering};
use font_features::FontVariations;

use super::text_properties::*;
use crate::core::{
    focus::{FocusInfoBuilder, FOCUS, FOCUS_CHANGED_EVENT},
    keyboard::{CHAR_INPUT_EVENT, KEYBOARD},
    task::parking_lot::Mutex,
    text::*,
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
    /// This variable is replaced often, the text resolver subscribes to it automatically.
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
/// This node setups the [`ResolvedText`] for all inner nodes, the `text!` widget includes this node in the [`NestGroup::EVENT`] group,
/// so all properties except [`NestGroup::CONTEXT`] have access using the [`ResolvedText::get`] function.
///
/// This node also subscribes to all the text context vars so other `text!` properties don't need to.
pub fn resolve_text(child: impl UiNode, text: impl IntoVar<Text>) -> impl UiNode {
    struct ResolveTextNode<C, T> {
        child: C,
        text: T,
        faces: Option<(VarHandle, ResponseVar<FontFaceList>)>,
        resolved: Mutex<Option<ResolvedText>>,
        event_handles: EventHandles,
        caret_opacity_handle: Option<VarHandle>,
    }
    impl<C: UiNode, T> ResolveTextNode<C, T> {
        fn with_mut<R>(&mut self, f: impl FnOnce(&mut C) -> R) -> R {
            RESOLVED_TEXT.with_context_opt(self.resolved.get_mut(), || f(&mut self.child))
        }
        fn with<R>(&self, f: impl FnOnce(&C) -> R) -> R {
            RESOLVED_TEXT.with_context_opt(&mut *self.resolved.lock(), || f(&self.child))
        }
    }
    impl<C: UiNode, T: Var<Text>> UiNode for ResolveTextNode<C, T> {
        fn init(&mut self) {
            WIDGET
                .sub_var(&self.text)
                .sub_var(&LANG_VAR)
                .sub_var(&DIRECTION_VAR)
                .sub_var(&FONT_FAMILY_VAR)
                .sub_var(&FONT_STYLE_VAR)
                .sub_var(&FONT_WEIGHT_VAR)
                .sub_var(&FONT_STRETCH_VAR)
                .sub_var(&TEXT_TRANSFORM_VAR)
                .sub_var(&WHITE_SPACE_VAR)
                .sub_var(&FONT_SIZE_VAR)
                .sub_var(&FONT_VARIATIONS_VAR)
                .sub_var(&LINE_HEIGHT_VAR)
                .sub_var(&LETTER_SPACING_VAR)
                .sub_var(&WORD_SPACING_VAR)
                .sub_var(&LINE_SPACING_VAR)
                .sub_var(&WORD_BREAK_VAR)
                .sub_var(&LINE_BREAK_VAR)
                .sub_var(&TAB_LENGTH_VAR)
                .sub_var(&FONT_FEATURES_VAR)
                .sub_var(&TEXT_ALIGN_VAR)
                .sub_var(&TEXT_COLOR_VAR)
                .sub_var(&FONT_SYNTHESIS_VAR)
                .sub_var(&FONT_AA_VAR)
                .sub_var(&OVERLINE_THICKNESS_VAR)
                .sub_var(&OVERLINE_STYLE_VAR)
                .sub_var(&OVERLINE_COLOR_VAR)
                .sub_var(&STRIKETHROUGH_THICKNESS_VAR)
                .sub_var(&STRIKETHROUGH_STYLE_VAR)
                .sub_var(&STRIKETHROUGH_COLOR_VAR)
                .sub_var(&UNDERLINE_THICKNESS_VAR)
                .sub_var(&UNDERLINE_COLOR_VAR)
                .sub_var(&UNDERLINE_SKIP_VAR)
                .sub_var(&UNDERLINE_POSITION_VAR)
                .sub_var(&CARET_COLOR_VAR);

            let style = FONT_STYLE_VAR.get();
            let weight = FONT_WEIGHT_VAR.get();

            let faces = FONT_FAMILY_VAR.with(|family| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

            let faces = if faces.is_done() {
                faces.into_rsp().unwrap()
            } else {
                self.faces = Some((faces.subscribe(WIDGET.id()), faces));
                FontFaceList::empty()
            };

            let text = self.text.get();
            let text = TEXT_TRANSFORM_VAR.with(|t| t.transform(text));
            let text = WHITE_SPACE_VAR.with(|t| t.transform(text));

            let editable = TEXT_EDITABLE_VAR.get();
            let caret_opacity = if editable && FOCUS.focused().get().map(|p| p.widget_id()) == Some(WIDGET.id()) {
                let v = KEYBOARD.caret_animation();
                self.caret_opacity_handle = Some(v.subscribe(WIDGET.id()));
                v
            } else {
                var(0.fct()).read_only()
            };

            *self.resolved.get_mut() = Some(ResolvedText {
                synthesis: FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight),
                faces,
                text: SegmentedText::new(text, DIRECTION_VAR.get()),
                reshape: false,
                baseline: Atomic::new(Px(0)),
                caret_opacity,
            });

            if editable {
                self.event_handles.push(CHAR_INPUT_EVENT.subscribe(WIDGET.id()));
                self.event_handles.push(FOCUS_CHANGED_EVENT.subscribe(WIDGET.id()));
            }

            self.with_mut(|c| c.init())
        }

        fn info(&self, info: &mut WidgetInfoBuilder) {
            if TEXT_EDITABLE_VAR.get() {
                FocusInfoBuilder::new(info).focusable(true);
            }
            self.with(|c| c.info(info))
        }

        fn deinit(&mut self) {
            self.event_handles.clear();
            self.caret_opacity_handle = None;
            self.faces = None;
            self.with_mut(|c| c.deinit());
            *self.resolved.get_mut() = None;
        }

        fn event(&mut self, update: &EventUpdate) {
            if let Some(args) = CHAR_INPUT_EVENT.on(update) {
                if !args.propagation().is_stopped()
                    && self.text.capabilities().contains(VarCapabilities::MODIFY)
                    && args.is_enabled(WIDGET.id())
                {
                    args.propagation().stop();

                    let new_animation = KEYBOARD.caret_animation();
                    self.caret_opacity_handle = Some(new_animation.subscribe(WIDGET.id()));
                    self.resolved.get_mut().as_mut().unwrap().caret_opacity = new_animation;

                    if args.is_backspace() {
                        let _ = self.text.modify(move |t| {
                            if !t.as_ref().is_empty() {
                                t.to_mut().to_mut().pop();
                            }
                        });
                    } else {
                        let c = args.character;
                        let _ = self.text.modify(move |t| {
                            t.to_mut().to_mut().push(c);
                        });
                    }
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                if TEXT_EDITABLE_VAR.get() {
                    if args.is_focused(WIDGET.id()) {
                        let new_animation = KEYBOARD.caret_animation();
                        self.caret_opacity_handle = Some(new_animation.subscribe(WIDGET.id()));
                        self.resolved.get_mut().as_mut().unwrap().caret_opacity = new_animation;
                    } else {
                        self.caret_opacity_handle = None;
                        self.resolved.get_mut().as_mut().unwrap().caret_opacity = var(0.fct()).read_only();
                    }
                }
            } else if let Some(_args) = FONT_CHANGED_EVENT.on(update) {
                // font query may return a different result.

                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR.with(|family| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), &LANG_VAR.get()));

                if faces.is_done() {
                    let faces = faces.rsp().unwrap();

                    let r = self.resolved.get_mut().as_mut().unwrap();

                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                        r.faces = faces;

                        r.reshape = true;
                        WIDGET.layout();
                    }
                } else {
                    self.faces = Some((faces.subscribe(WIDGET.id()), faces));
                }
            }
            self.with_mut(|c| c.event(update))
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            let r = self.resolved.get_mut().as_mut().unwrap();

            // update `r.text`, affects layout.
            if self.text.is_new() || TEXT_TRANSFORM_VAR.is_new() || WHITE_SPACE_VAR.is_new() || LANG_VAR.is_new() {
                let text = self.text.get();
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
                    self.faces = Some((faces.subscribe(WIDGET.id()), faces));
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
                if enabled && self.event_handles.0.is_empty() {
                    // actually enabled.

                    let id = WIDGET.id();
                    self.event_handles.push(CHAR_INPUT_EVENT.subscribe(id));
                    self.event_handles.push(FOCUS_CHANGED_EVENT.subscribe(id));

                    if FOCUS.focused().get().map(|p| p.widget_id()) == Some(id) {
                        let new_animation = KEYBOARD.caret_animation();
                        self.caret_opacity_handle = Some(new_animation.subscribe(id));
                        r.caret_opacity = new_animation;
                    }
                } else {
                    self.event_handles.clear();
                    self.caret_opacity_handle = None;
                    r.caret_opacity = var(0.fct()).read_only();
                }
            }

            if let Some((_, faces)) = &self.faces {
                if faces.is_done() {
                    let faces = faces.rsp().unwrap();
                    self.faces = None;

                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                        r.faces = faces;

                        r.reshape = true;
                        WIDGET.layout();
                    }
                }
            }

            self.with_mut(|c| c.update(updates))
        }

        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            self.with(|c| c.measure(wm))
        }
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let size = self.with_mut(|c| c.layout(wl));
            self.resolved.get_mut().as_mut().unwrap().reshape = false;
            size
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.with(|c| c.render(frame))
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            self.with(|c| c.render_update(update))
        }
    }
    ResolveTextNode {
        child: child.cfg_boxed(),
        text: text.into_var(),
        faces: None,
        resolved: Mutex::new(None),
        event_handles: EventHandles::default(),
        caret_opacity_handle: None,
    }
    .cfg_boxed()
}

/// An UI node that layouts the parent [`ResolvedText`] defined by the text context vars.
///
/// This node setups the [`LayoutText`] for all inner nodes in the layout and render methods, the `text!` widget includes this
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
        last_layout: (LayoutMetrics, Option<InlineConstrainsMeasure>),
    }
    impl FinalText {
        fn measure(&mut self, metrics: &LayoutMetrics) -> Option<PxSize> {
            if metrics.inline_constrains().is_some() {
                return None;
            }

            metrics.constrains().fill_or_exact()
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

            if TEXT_WRAP_VAR.get() && !metrics.constrains().x.is_unbounded() {
                let max_width = metrics.constrains().x.max().unwrap();
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

            if let Some(inline) = metrics.inline_constrains() {
                match inline {
                    InlineConstrains::Measure(m) => {
                        if self.shaping_args.inline_constrains != Some(m) {
                            self.shaping_args.inline_constrains = Some(m);
                            self.pending.insert(Layout::RESHAPE);
                        }
                    }
                    InlineConstrains::Layout(l) => {
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
            } else if self.shaping_args.inline_constrains.is_some() {
                self.shaping_args.inline_constrains = None;
                self.pending.insert(Layout::RESHAPE);
            }

            if !self.pending.contains(Layout::RESHAPE_LINES) {
                let size = r.shaped_text.size();
                if metrics.constrains().fill_size_or(size) != r.shaped_text.align_size() {
                    self.pending.insert(Layout::RESHAPE_LINES);
                }
            }

            let font = r.fonts.best();

            let space_len = font.space_x_advance();
            let dft_tab_len = space_len * 3;

            let (letter_spacing, word_spacing, tab_length) = {
                LAYOUT.with_constrains(
                    |_| PxConstrains2d::new_exact(space_len, space_len),
                    || {
                        (
                            LETTER_SPACING_VAR.layout_x(),
                            WORD_SPACING_VAR.layout_x(),
                            TAB_LENGTH_VAR.layout_dft_x(dft_tab_len),
                        )
                    },
                )
            };

            let dft_line_height = font.metrics().line_height();
            let line_height = {
                LAYOUT.with_constrains(
                    |_| PxConstrains2d::new_exact(dft_line_height, dft_line_height),
                    || LINE_HEIGHT_VAR.layout_dft_y(dft_line_height),
                )
            };
            let line_spacing = {
                LAYOUT.with_constrains(
                    |_| PxConstrains2d::new_exact(line_height, line_height),
                    || LINE_SPACING_VAR.layout_y(),
                )
            };

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
                LAYOUT.with_constrains(
                    |_| PxConstrains2d::new_exact(line_height, line_height),
                    || {
                        (
                            OVERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                            STRIKETHROUGH_THICKNESS_VAR.layout_dft_y(dft_thickness),
                            UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        )
                    },
                )
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

            if !self.pending.contains(Layout::RESHAPE_LINES) && prev_final_size != metrics.constrains().fill_size_or(r.shaped_text.size()) {
                self.pending.insert(Layout::RESHAPE_LINES);
            }

            if !is_measure {
                self.last_layout = (metrics.clone(), self.shaping_args.inline_constrains);

                if self.pending.contains(Layout::RESHAPE_LINES) {
                    r.shaped_text.reshape_lines(
                        metrics.constrains(),
                        metrics.inline_constrains().map(|c| c.layout()),
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

            metrics.constrains().fill_size_or(r.shaped_text.size())
        }

        fn ensure_layout_for_render(&mut self) {
            if self.txt_is_measured {
                let metrics = self.last_layout.0.clone();
                self.shaping_args.inline_constrains = self.last_layout.1;
                LAYOUT.with_context(metrics.clone(), || {
                    self.layout(&metrics, &RESOLVED_TEXT.get(), false);
                });

                debug_assert!(!self.txt_is_measured);
            }
        }
    }

    #[ui_node(struct LayoutTextNode {
        child: impl UiNode,
        txt: Mutex<FinalText>,
    })]
    impl LayoutTextNode {
        fn with_mut<R>(&mut self, f: impl FnOnce(&mut T_child) -> R) -> R {
            LAYOUT_TEXT.with_context_opt(&mut self.txt.get_mut().txt, || f(&mut self.child))
        }
        fn with(&self, f: impl FnOnce(&T_child)) {
            LAYOUT_TEXT.with_context_opt(&mut self.txt.lock().txt, || f(&self.child))
        }

        #[UiNode]
        fn init(&mut self) {
            // other subscriptions are handled by the `resolve_text` node.
            let txt = self.txt.get_mut();
            txt.shaping_args.lang = LANG_VAR.get();
            txt.shaping_args.direction = txt.shaping_args.lang.character_direction().into();
            txt.shaping_args.line_break = LINE_BREAK_VAR.get();
            txt.shaping_args.word_break = WORD_BREAK_VAR.get();
            txt.shaping_args.hyphens = HYPHENS_VAR.get();
            txt.shaping_args.hyphen_char = HYPHEN_CHAR_VAR.get();
            txt.shaping_args.font_features = FONT_FEATURES_VAR.with(|f| f.finalize());

            self.child.init();
        }

        #[UiNode]
        fn deinit(&mut self) {
            self.child.deinit();
            self.txt.get_mut().txt = None;
        }

        #[UiNode]
        fn update(&mut self, updates: &WidgetUpdates) {
            if FONT_SIZE_VAR.is_new() || FONT_VARIATIONS_VAR.is_new() {
                self.txt.get_mut().pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            }

            if LETTER_SPACING_VAR.is_new()
                || WORD_SPACING_VAR.is_new()
                || LINE_SPACING_VAR.is_new()
                || LINE_HEIGHT_VAR.is_new()
                || TAB_LENGTH_VAR.is_new()
                || LANG_VAR.is_new()
            {
                let txt = self.txt.get_mut();
                txt.shaping_args.lang = LANG_VAR.get();
                txt.shaping_args.direction = txt.shaping_args.lang.character_direction().into(); // will be set in layout too.
                txt.pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            }

            if UNDERLINE_POSITION_VAR.is_new() || UNDERLINE_SKIP_VAR.is_new() {
                self.txt.get_mut().pending.insert(Layout::UNDERLINE);
                WIDGET.layout();
            }

            if OVERLINE_THICKNESS_VAR.is_new() || STRIKETHROUGH_THICKNESS_VAR.is_new() || UNDERLINE_THICKNESS_VAR.is_new() {
                WIDGET.layout();
            }

            if let Some(lb) = LINE_BREAK_VAR.get_new() {
                let txt = self.txt.get_mut();
                if txt.shaping_args.line_break != lb {
                    txt.shaping_args.line_break = lb;
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }

            if let Some(wb) = WORD_BREAK_VAR.get_new() {
                let txt = self.txt.get_mut();
                if txt.shaping_args.word_break != wb {
                    txt.shaping_args.word_break = wb;
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }

            if let Some(h) = HYPHENS_VAR.get_new() {
                let txt = self.txt.get_mut();
                if txt.shaping_args.hyphens != h {
                    txt.shaping_args.hyphens = h;
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }

            if let Some(c) = HYPHEN_CHAR_VAR.get_new() {
                let txt = self.txt.get_mut();
                txt.shaping_args.hyphen_char = c;
                if Hyphens::None != txt.shaping_args.hyphens {
                    txt.pending.insert(Layout::RESHAPE);
                    WIDGET.layout();
                }
            }

            if TEXT_WRAP_VAR.is_new() {
                self.txt.get_mut().pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            }

            FONT_FEATURES_VAR.with_new(|f| {
                let txt = self.txt.get_mut();
                txt.shaping_args.font_features = f.finalize();
                txt.pending.insert(Layout::RESHAPE);
                WIDGET.layout();
            });

            self.child.update(updates);
        }

        #[UiNode]
        fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
            let mut txt = self.txt.lock();
            let metrics = LAYOUT.metrics();

            if let Some(size) = txt.measure(&metrics) {
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
            }
        }
        #[UiNode]
        fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
            let txt = self.txt.get_mut();

            let metrics = LAYOUT.metrics();
            let resolved_txt = RESOLVED_TEXT.get();
            let size = txt.layout(&metrics, &resolved_txt, false);

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

            LAYOUT.with_constrains(
                |_| PxConstrains2d::new_fill_size(size),
                || {
                    self.with_mut(|c| c.layout(wl));
                },
            );

            size
        }

        #[UiNode]
        fn render(&self, frame: &mut FrameBuilder) {
            self.txt.lock().ensure_layout_for_render();
            self.with(|c| c.render(frame))
        }
        #[UiNode]
        fn render_update(&self, update: &mut FrameUpdate) {
            self.txt.lock().ensure_layout_for_render();
            self.with(|c| c.render_update(update))
        }
    }
    LayoutTextNode {
        child: child.cfg_boxed(),
        txt: Mutex::new(FinalText {
            txt: None,
            shaping_args: TextShapingArgs::default(),
            pending: Layout::empty(),
            txt_is_measured: false,
            last_layout: (LayoutMetrics::new(1.fct(), PxSize::zero(), Px(0)), None),
        }),
    }
    .cfg_boxed()
}

/// An Ui node that renders the default underline visual using the parent [`LayoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `text!` widgets introduces this node in `new_child`, around the [`render_strikethroughs`] node.
pub fn render_underlines(child: impl UiNode) -> impl UiNode {
    #[ui_node(struct RenderUnderlineNode {
        child: impl UiNode,
    })]
    impl UiNode for RenderUnderlineNode {
        // subscriptions are handled by the `resolve_text` node.

        fn update(&mut self, updates: &WidgetUpdates) {
            if UNDERLINE_STYLE_VAR.is_new() || UNDERLINE_COLOR_VAR.is_new() {
                WIDGET.render();
            }

            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            {
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

            self.child.render(frame);
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
    #[ui_node(struct RenderStrikethroughsNode {
        child: impl UiNode,
    })]
    impl UiNode for RenderStrikethroughsNode {
        // subscriptions are handled by the `resolve_text` node.
        fn update(&mut self, updates: &WidgetUpdates) {
            if STRIKETHROUGH_STYLE_VAR.is_new() || STRIKETHROUGH_COLOR_VAR.is_new() {
                WIDGET.render();
            }

            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            {
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

            self.child.render(frame);
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
    #[ui_node(struct RenderOverlineNode {
        child: impl UiNode,
    })]
    impl UiNode for RenderOverlineNode {
        // subscriptions are handled by the `resolve_text` node.
        fn update(&mut self, updates: &WidgetUpdates) {
            if OVERLINE_STYLE_VAR.is_new() || OVERLINE_COLOR_VAR.is_new() {
                WIDGET.render();
            }

            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            {
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

            self.child.render(frame);
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
    #[ui_node(struct RenderCaretNode {
        child: impl UiNode,
        color: Rgba,
        color_key: FrameValueKey<RenderColor>,
    })]
    impl UiNode for RenderCaretNode {
        // subscriptions are handled by the text resolver node.
        fn init(&mut self) {
            self.color = if TEXT_EDITABLE_VAR.get() {
                let mut c = CARET_COLOR_VAR.get();
                c.alpha *= ResolvedText::get().caret_opacity.get().0;
                c
            } else {
                rgba(0, 0, 0, 0)
            };

            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            let color = if TEXT_EDITABLE_VAR.get() {
                let mut c = CARET_COLOR_VAR.get();
                c.alpha *= ResolvedText::get().caret_opacity.get().0;
                c
            } else {
                rgba(0, 0, 0, 0)
            };

            if self.color != color {
                self.color = color;
                WIDGET.render_update();
            }

            self.child.update(updates);
        }

        fn render(&self, frame: &mut FrameBuilder) {
            self.child.render(frame);

            if TEXT_EDITABLE_VAR.get() {
                let t = LayoutText::get();

                let mut clip_rect = PxRect::from_size(t.shaped_text.align_size());
                clip_rect.size.width = Dip::new(1).to_px(frame.scale_factor().0);
                clip_rect.size.height = t.shaped_text.line_height();

                frame.push_color(clip_rect, self.color_key.bind(self.color.into(), true));
            }
        }

        fn render_update(&self, update: &mut FrameUpdate) {
            self.child.render_update(update);

            if TEXT_EDITABLE_VAR.get() {
                update.update_color(self.color_key.update(self.color.into(), true))
            }
        }
    }
    RenderCaretNode {
        child,
        color: rgba(0, 0, 0, 0),
        color_key: FrameValueKey::new_unique(),
    }
}

/// An UI node that renders the parent [`LayoutText`].
///
/// This node renders the text only, decorators are rendered by other nodes.
///
/// This is the `text!` widget inner most child node.
pub fn render_text() -> impl UiNode {
    #[derive(Clone, Copy, PartialEq)]
    struct RenderedText {
        version: u32,
        synthesis: FontSynthesis,
        color: Rgba,
        aa: FontAntiAliasing,
    }
    #[ui_node(struct RenderTextNode {
        reuse: RefCell<Option<ReuseRange>>,
        rendered: Cell<Option<RenderedText>>,
        color_key: Option<FrameValueKey<RenderColor>>,
    })]
    impl UiNode for RenderTextNode {
        fn init(&mut self) {
            if TEXT_COLOR_VAR.capabilities().contains(VarCapabilities::NEW) {
                self.color_key = Some(FrameValueKey::new_unique());
            }
        }

        fn deinit(&mut self) {
            *self.reuse.get_mut() = None;
            self.color_key = None;
        }

        // subscriptions are handled by the `resolve_text` node.
        fn update(&mut self, _: &WidgetUpdates) {
            if FONT_AA_VAR.is_new() {
                WIDGET.render();
            } else if TEXT_COLOR_VAR.is_new() {
                WIDGET.render_update();
            }
        }

        fn render(&self, frame: &mut FrameBuilder) {
            let r = ResolvedText::get();
            let t = LayoutText::get();

            let lh = t.shaped_text.line_height();
            let clip = PxRect::from_size(t.shaped_text.align_size()).inflate(lh, lh); // clip inflated to allow some weird glyphs
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
        }

        fn render_update(&self, update: &mut FrameUpdate) {
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
