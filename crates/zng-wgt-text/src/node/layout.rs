use std::{mem, sync::Arc};

use atomic::Atomic;
use parking_lot::RwLock;
use zng_app::{
    DInstant,
    event::{AnyEventArgs as _, CommandHandle, EventHandle, EventHandles},
    widget::{
        WIDGET,
        node::{UiNode, UiNodeOp, match_node},
    },
};
use zng_ext_font::{CaretIndex, FontFaceList, Hyphens, SegmentedText, ShapedText, TextShapingArgs, font_features::FontVariations};
use zng_ext_input::{
    focus::FOCUS,
    keyboard::{KEY_INPUT_EVENT, KEYBOARD},
    mouse::{MOUSE, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT},
    pointer_capture::{POINTER_CAPTURE, POINTER_CAPTURE_EVENT},
    touch::{TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_TAP_EVENT},
};
use zng_ext_l10n::LANG_VAR;
use zng_ext_undo::UNDO;
use zng_ext_window::WidgetInfoBuilderImeArea as _;
use zng_layout::{
    context::{InlineConstraints, InlineConstraintsMeasure, InlineSegment, LAYOUT, LayoutMetrics},
    unit::{DipPoint, FactorUnits as _, Px, PxBox, PxConstraints2d, PxRect, PxSize, PxTransform, Rect, Size},
};
use zng_view_api::keyboard::{Key, KeyState};
use zng_wgt::prelude::*;
use zng_wgt_scroll::{SCROLL, cmd::ScrollToMode};

use crate::{
    ACCEPTS_ENTER_VAR, AUTO_SELECTION_VAR, AutoSelection, FONT_FAMILY_VAR, FONT_FEATURES_VAR, FONT_SIZE_VAR,
    FONT_STRETCH_VAR, FONT_STYLE_VAR, FONT_VARIATIONS_VAR, FONT_WEIGHT_VAR, HYPHEN_CHAR_VAR, HYPHENS_VAR, IME_UNDERLINE_THICKNESS_VAR,
    JUSTIFY_MODE_VAR, LETTER_SPACING_VAR, LINE_BREAK_VAR, LINE_HEIGHT_VAR, LINE_SPACING_VAR, OBSCURE_TXT_VAR, OBSCURING_CHAR_VAR,
    OVERLINE_THICKNESS_VAR, STRIKETHROUGH_THICKNESS_VAR, TAB_LENGTH_VAR, TEXT_ALIGN_VAR, TEXT_EDITABLE_VAR, TEXT_OVERFLOW_ALIGN_VAR,
    TEXT_OVERFLOW_VAR, TEXT_SELECTABLE_ALT_ONLY_VAR, TEXT_SELECTABLE_VAR, TEXT_WRAP_VAR, TextOverflow, UNDERLINE_POSITION_VAR,
    UNDERLINE_SKIP_VAR, UNDERLINE_THICKNESS_VAR, UnderlinePosition, UnderlineSkip, WORD_BREAK_VAR, WORD_SPACING_VAR,
    cmd::{SELECT_ALL_CMD, SELECT_CMD, TextSelectOp},
    node::SelectionBy,
};

use super::{LAIDOUT_TEXT, LaidoutText, PendingLayout, RenderInfo, TEXT};

/// An UI node that layouts the parent [`ResolvedText`] defined by the text context vars.
///
/// This node setups the [`LaidoutText`] for all inner nodes, the `Text!` widget includes this
/// node in the `NestGroup::CHILD_LAYOUT + 100` nest group, so all properties in [`NestGroup::CHILD_LAYOUT`]
/// can affect the layout normally and custom properties can be created to be inside this group and have access
/// to the [`TEXT::laidout`] method.
///
/// [`ResolvedText`]: super::ResolvedText
///
/// [`NestGroup::CHILD_LAYOUT`]: zng_wgt::prelude::NestGroup::CHILD_LAYOUT
pub fn layout_text(child: impl UiNode) -> impl UiNode {
    let child = layout_text_edit(child);
    let child = layout_text_layout(child);
    layout_text_context(child)
}
fn layout_text_context(child: impl UiNode) -> impl UiNode {
    let mut laidout = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            let fonts = FontFaceList::empty().sized(Px(10), vec![]);
            laidout = Some(Arc::new(RwLock::new(LaidoutText {
                shaped_text: ShapedText::new(fonts.best()),
                fonts,
                overflow: None,
                overflow_suffix: None,
                shaped_text_version: 0,
                overlines: vec![],
                overline_thickness: Px(0),
                strikethroughs: vec![],
                strikethrough_thickness: Px(0),
                underlines: vec![],
                underline_thickness: Px(0),
                ime_underlines: vec![],
                ime_underline_thickness: Px(0),
                caret_origin: None,
                caret_selection_origin: None,
                caret_retained_x: Px(0),
                render_info: RenderInfo {
                    transform: PxTransform::identity(),
                    scale_factor: 1.fct(),
                },
                viewport: PxSize::zero(),
            })));

            LAIDOUT_TEXT.with_context(&mut laidout, || child.init());
        }
        UiNodeOp::Deinit => {
            LAIDOUT_TEXT.with_context(&mut laidout, || child.deinit());

            laidout = None;
        }
        op => LAIDOUT_TEXT.with_context(&mut laidout, || child.op(op)),
    })
}
fn layout_text_layout(child: impl UiNode) -> impl UiNode {
    let mut txt = LayoutTextFinal {
        shaping_args: TextShapingArgs::default(),
        pending: PendingLayout::empty(),
        txt_is_measured: false,
        last_layout: (LayoutMetrics::new(1.fct(), PxSize::zero(), Px(0)), None),
        baseline: Px(0),
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
                .sub_var(&TEXT_WRAP_VAR)
                .sub_var(&TEXT_OVERFLOW_VAR);
            WIDGET
                .sub_var_layout(&TEXT_ALIGN_VAR)
                .sub_var_layout(&JUSTIFY_MODE_VAR)
                .sub_var_layout(&TEXT_OVERFLOW_ALIGN_VAR);

            WIDGET.sub_var(&FONT_FEATURES_VAR);

            WIDGET.sub_var(&OBSCURE_TXT_VAR).sub_var(&OBSCURING_CHAR_VAR);

            // LANG_VAR already subscribed by resolve_text

            txt.shaping_args.lang = LANG_VAR.with(|l| l.best().clone());
            txt.shaping_args.direction = txt.shaping_args.lang.direction();
            txt.shaping_args.line_break = LINE_BREAK_VAR.get();
            txt.shaping_args.word_break = WORD_BREAK_VAR.get();
            txt.shaping_args.hyphens = HYPHENS_VAR.get();
            txt.shaping_args.hyphen_char = HYPHEN_CHAR_VAR.get();
            txt.shaping_args.font_features = FONT_FEATURES_VAR.with(|f| f.finalize());

            if OBSCURE_TXT_VAR.get() {
                txt.shaping_args.obscuring_char = Some(OBSCURING_CHAR_VAR.get());
            }
        }
        UiNodeOp::Deinit => {
            txt.shaping_args = TextShapingArgs::default();
        }
        UiNodeOp::Update { .. } => {
            if FONT_SIZE_VAR.is_new() || FONT_VARIATIONS_VAR.is_new() {
                txt.pending.insert(PendingLayout::RESHAPE);
                TEXT.layout().overflow_suffix = None;
                WIDGET.layout();
            }

            if LETTER_SPACING_VAR.is_new()
                || WORD_SPACING_VAR.is_new()
                || LINE_SPACING_VAR.is_new()
                || LINE_HEIGHT_VAR.is_new()
                || TAB_LENGTH_VAR.is_new()
                || LANG_VAR.is_new()
            {
                txt.shaping_args.lang = LANG_VAR.with(|l| l.best().clone());
                txt.shaping_args.direction = txt.shaping_args.lang.direction(); // will be set in layout too.
                TEXT.layout().overflow_suffix = None;
                txt.pending.insert(PendingLayout::RESHAPE);
                WIDGET.layout();
            }

            if UNDERLINE_POSITION_VAR.is_new() || UNDERLINE_SKIP_VAR.is_new() {
                txt.pending.insert(PendingLayout::UNDERLINE);
                WIDGET.layout();
            }

            if let Some(lb) = LINE_BREAK_VAR.get_new() {
                if txt.shaping_args.line_break != lb {
                    txt.shaping_args.line_break = lb;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(wb) = WORD_BREAK_VAR.get_new() {
                if txt.shaping_args.word_break != wb {
                    txt.shaping_args.word_break = wb;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(h) = HYPHENS_VAR.get_new() {
                if txt.shaping_args.hyphens != h {
                    txt.shaping_args.hyphens = h;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(c) = HYPHEN_CHAR_VAR.get_new() {
                txt.shaping_args.hyphen_char = c;
                if Hyphens::None != txt.shaping_args.hyphens {
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }

            if OBSCURE_TXT_VAR.is_new() || OBSCURING_CHAR_VAR.is_new() {
                let c = if OBSCURE_TXT_VAR.get() {
                    Some(OBSCURING_CHAR_VAR.get())
                } else {
                    None
                };
                if txt.shaping_args.obscuring_char != c {
                    txt.shaping_args.obscuring_char = c;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }

            if TEXT_WRAP_VAR.is_new() {
                txt.pending.insert(PendingLayout::RESHAPE);
                WIDGET.layout();
            }
            if TEXT_OVERFLOW_VAR.is_new() {
                TEXT.layout().overflow_suffix = None;
                txt.pending.insert(PendingLayout::OVERFLOW);
                WIDGET.layout();
            }

            FONT_FEATURES_VAR.with_new(|f| {
                txt.shaping_args.font_features = f.finalize();
                txt.pending.insert(PendingLayout::RESHAPE);
                WIDGET.layout();
            });

            if FONT_FAMILY_VAR.is_new()
                || FONT_STYLE_VAR.is_new()
                || FONT_STRETCH_VAR.is_new()
                || FONT_WEIGHT_VAR.is_new()
                || LANG_VAR.is_new()
            {
                // resolve_text already requests RESHAPE
                TEXT.layout().overflow_suffix = None;
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();

            let metrics = LAYOUT.metrics();

            *desired_size = if let Some(size) = txt.measure(&metrics) {
                size
            } else {
                let size = txt.layout(&metrics, true);

                if let Some(inline) = wm.inline() {
                    let ctx = TEXT.laidout();

                    if let Some(first_line) = ctx.shaped_text.line(0) {
                        inline.first = first_line.original_size();
                        inline.with_first_segs(|i| {
                            for seg in first_line.segs() {
                                i.push(InlineSegment::new(seg.advance(), seg.kind()));
                            }
                        });
                    } else {
                        inline.first = PxSize::zero();
                        inline.with_first_segs(|i| i.clear());
                    }

                    if ctx.shaped_text.lines_len() == 1 {
                        inline.last = inline.first;
                        inline.last_segs = inline.first_segs.clone();
                    } else if let Some(last_line) = ctx.shaped_text.line(ctx.shaped_text.lines_len().saturating_sub(1)) {
                        inline.last = last_line.original_size();
                        inline.with_last_segs(|i| {
                            for seg in last_line.segs() {
                                i.push(InlineSegment::new(seg.advance(), seg.kind()));
                            }
                        })
                    } else {
                        inline.last = PxSize::zero();
                        inline.with_last_segs(|i| i.clear());
                    }

                    inline.first_wrapped = ctx.shaped_text.first_wrapped();
                    inline.last_wrapped = ctx.shaped_text.lines_len() > 1;
                }
                size
            };

            LAYOUT.with_constraints(metrics.constraints().with_new_min_size(*desired_size), || {
                // foreign nodes in the CHILD_LAYOUT+100 ..= CHILD range may change the size
                *desired_size = child.measure(wm);
            });
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();

            let metrics = LAYOUT.metrics();

            TEXT.layout().viewport = metrics.viewport();

            *final_size = txt.layout(&metrics, false);

            if txt.pending != PendingLayout::empty() {
                WIDGET.render();
                txt.pending = PendingLayout::empty();
            }

            if let Some(inline) = wl.inline() {
                let ctx = TEXT.laidout();

                let last_line = ctx.shaped_text.lines_len().saturating_sub(1);

                inline.first_segs.clear();
                inline.last_segs.clear();

                for (i, line) in ctx.shaped_text.lines().enumerate() {
                    if i == 0 {
                        let info = ctx.shaped_text.line(0).unwrap().segs().map(|s| s.inline_info());
                        if LAYOUT.direction().is_rtl() {
                            // help sort
                            inline.set_first_segs(info.rev());
                        } else {
                            inline.set_first_segs(info);
                        }
                    } else if i == last_line {
                        let info = ctx
                            .shaped_text
                            .line(ctx.shaped_text.lines_len().saturating_sub(1))
                            .unwrap()
                            .segs()
                            .map(|s| s.inline_info());
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

            wl.set_baseline(txt.baseline);

            LAYOUT.with_constraints(metrics.constraints().with_new_min_size(*final_size), || {
                // foreign nodes in the CHILD_LAYOUT+100 ..= CHILD range may change the size
                *final_size = child.layout(wl);
            });
        }
        UiNodeOp::Render { .. } => {
            txt.ensure_layout_for_render();
        }
        UiNodeOp::RenderUpdate { .. } => {
            txt.ensure_layout_for_render();
        }
        _ => {}
    })
}
struct LayoutTextFinal {
    shaping_args: TextShapingArgs,
    pending: PendingLayout,

    txt_is_measured: bool,
    last_layout: (LayoutMetrics, Option<InlineConstraintsMeasure>),
    baseline: Px,
}
impl LayoutTextFinal {
    fn measure(&mut self, metrics: &LayoutMetrics) -> Option<PxSize> {
        if metrics.inline_constraints().is_some() {
            return None;
        }

        metrics.constraints().fill_or_exact()
    }

    fn layout(&mut self, metrics: &LayoutMetrics, is_measure: bool) -> PxSize {
        let mut resolved = TEXT.resolved();

        self.pending |= resolved.pending_layout;

        let font_size = metrics.font_size();

        let mut ctx = TEXT.layout();

        if font_size != ctx.fonts.requested_size() || !ctx.fonts.is_sized_from(&resolved.faces) {
            ctx.fonts = resolved.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
            self.pending.insert(PendingLayout::RESHAPE);
        }

        if TEXT_WRAP_VAR.get() && !metrics.constraints().x.is_unbounded() {
            let max_width = metrics.constraints().x.max().unwrap();
            if self.shaping_args.max_width != max_width {
                self.shaping_args.max_width = max_width;

                if !self.pending.contains(PendingLayout::RESHAPE) && ctx.shaped_text.can_rewrap(max_width) {
                    self.pending.insert(PendingLayout::RESHAPE);
                }
            }
        } else if self.shaping_args.max_width != Px::MAX {
            self.shaping_args.max_width = Px::MAX;
            if !self.pending.contains(PendingLayout::RESHAPE) && ctx.shaped_text.can_rewrap(Px::MAX) {
                self.pending.insert(PendingLayout::RESHAPE);
            }
        }

        if ctx.caret_origin.is_none() {
            self.pending.insert(PendingLayout::CARET);
        }

        if let Some(inline) = metrics.inline_constraints() {
            match inline {
                InlineConstraints::Measure(m) => {
                    if self.shaping_args.inline_constraints != Some(m) {
                        self.shaping_args.inline_constraints = Some(m);
                        self.pending.insert(PendingLayout::RESHAPE);
                    }
                }
                InlineConstraints::Layout(l) => {
                    if !self.pending.contains(PendingLayout::RESHAPE)
                        && (Some(l.first_segs.len()) != ctx.shaped_text.line(0).map(|l| l.segs_len())
                            || Some(l.last_segs.len())
                                != ctx
                                    .shaped_text
                                    .line(ctx.shaped_text.lines_len().saturating_sub(1))
                                    .map(|l| l.segs_len()))
                    {
                        self.pending.insert(PendingLayout::RESHAPE);
                    }

                    if !self.pending.contains(PendingLayout::RESHAPE_LINES)
                        && (ctx.shaped_text.mid_clear() != l.mid_clear
                            || ctx.shaped_text.line(0).map(|l| l.rect()) != Some(l.first)
                            || ctx
                                .shaped_text
                                .line(ctx.shaped_text.lines_len().saturating_sub(1))
                                .map(|l| l.rect())
                                != Some(l.last))
                    {
                        self.pending.insert(PendingLayout::RESHAPE_LINES);
                    }
                }
            }
        } else if self.shaping_args.inline_constraints.is_some() {
            self.shaping_args.inline_constraints = None;
            self.pending.insert(PendingLayout::RESHAPE);
        }

        if !self.pending.contains(PendingLayout::RESHAPE_LINES) {
            let size = ctx.shaped_text.size();
            if metrics.constraints().fill_size_or(size) != ctx.shaped_text.align_size() {
                self.pending.insert(PendingLayout::RESHAPE_LINES);
            }
        }

        let font = ctx.fonts.best();

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

        if !self.pending.contains(PendingLayout::RESHAPE)
            && (letter_spacing != self.shaping_args.letter_spacing
                || word_spacing != self.shaping_args.word_spacing
                || tab_length != self.shaping_args.tab_x_advance)
        {
            self.pending.insert(PendingLayout::RESHAPE);
        }
        if !self.pending.contains(PendingLayout::RESHAPE_LINES)
            && (line_spacing != self.shaping_args.line_spacing || line_height != self.shaping_args.line_height)
        {
            self.pending.insert(PendingLayout::RESHAPE_LINES);
        }

        self.shaping_args.letter_spacing = letter_spacing;
        self.shaping_args.word_spacing = word_spacing;
        self.shaping_args.tab_x_advance = tab_length;
        self.shaping_args.line_height = line_height;
        self.shaping_args.line_spacing = line_spacing;

        let dft_thickness = font.metrics().underline_thickness;
        let (overline, strikethrough, underline, ime_underline) = {
            LAYOUT.with_constraints(PxConstraints2d::new_exact(line_height, line_height), || {
                (
                    OVERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                    STRIKETHROUGH_THICKNESS_VAR.layout_dft_y(dft_thickness),
                    UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                    IME_UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                )
            })
        };

        if !self.pending.contains(PendingLayout::OVERLINE) && (ctx.overline_thickness == 0) != (overline == 0) {
            self.pending.insert(PendingLayout::OVERLINE);
        }
        if !self.pending.contains(PendingLayout::STRIKETHROUGH) && (ctx.strikethrough_thickness == 0) != (strikethrough == 0) {
            self.pending.insert(PendingLayout::STRIKETHROUGH);
        }
        if !self.pending.contains(PendingLayout::UNDERLINE)
            && ((ctx.underline_thickness == 0) != (underline == 0) || (ctx.ime_underline_thickness != 0) != (ime_underline != 0))
        {
            self.pending.insert(PendingLayout::UNDERLINE);
        }
        ctx.overline_thickness = overline;
        ctx.strikethrough_thickness = strikethrough;
        ctx.underline_thickness = underline;
        ctx.ime_underline_thickness = ime_underline;

        let align = TEXT_ALIGN_VAR.get();
        let justify = JUSTIFY_MODE_VAR.get();
        let overflow_align = TEXT_OVERFLOW_ALIGN_VAR.get();
        if !self.pending.contains(PendingLayout::RESHAPE_LINES)
            && (align != ctx.shaped_text.align()
                || justify != ctx.shaped_text.justify_mode().unwrap_or_default()
                || overflow_align != ctx.shaped_text.overflow_align())
        {
            self.pending.insert(PendingLayout::RESHAPE_LINES);
        }

        /*
            APPLY
        */

        if self.pending.contains(PendingLayout::RESHAPE) {
            ctx.shaped_text = ctx.fonts.shape_text(&resolved.segmented_text, &self.shaping_args);
            self.pending = self.pending.intersection(PendingLayout::RESHAPE_LINES);
        }

        if !self.pending.contains(PendingLayout::RESHAPE_LINES)
            && ctx.shaped_text.align_size() != metrics.constraints().fill_size_or(ctx.shaped_text.block_size())
        {
            self.pending.insert(PendingLayout::RESHAPE_LINES);
        }

        if self.pending.contains(PendingLayout::RESHAPE_LINES) && metrics.inline_constraints().is_none() {
            // Affects block size in measure too
            //
            // This breaks inline context, so it is avoided here and called later in the `if !is_measure` block.
            // This is needed to measure the same block size a layout call would output.
            //
            // Not sure if it is a bug that it does not work inlining, but it is not needed there anyway, so for now
            // this fix is sufficient.
            ctx.shaped_text.reshape_lines(
                metrics.constraints(),
                metrics.inline_constraints().map(|c| c.layout()),
                align,
                overflow_align,
                line_height,
                line_spacing,
                metrics.direction(),
            );
        }

        if !is_measure {
            self.last_layout = (metrics.clone(), self.shaping_args.inline_constraints);

            if self.pending.contains(PendingLayout::RESHAPE_LINES) {
                if metrics.inline_constraints().is_some() {
                    // when inlining, only reshape lines in layout passes
                    ctx.shaped_text.reshape_lines(
                        metrics.constraints(),
                        metrics.inline_constraints().map(|c| c.layout()),
                        align,
                        overflow_align,
                        line_height,
                        line_spacing,
                        metrics.direction(),
                    );
                }
                ctx.shaped_text.reshape_lines_justify(justify, &self.shaping_args.lang);

                ctx.shaped_text_version = ctx.shaped_text_version.wrapping_add(1);
                drop(resolved);
                self.baseline = ctx.shaped_text.baseline();
                resolved = TEXT.resolved();
                ctx.caret_origin = None;
                ctx.caret_selection_origin = None;
            }
            if self.pending.contains(PendingLayout::OVERFLOW) {
                let txt_size = ctx.shaped_text.size();
                let max_size = metrics.constraints().fill_size_or(txt_size);
                if txt_size.width > max_size.width || txt_size.height > max_size.height {
                    let suf_width = ctx.overflow_suffix.as_ref().map(|s| s.size().width).unwrap_or(Px(0));
                    ctx.overflow = ctx.shaped_text.overflow_info(max_size, suf_width);

                    if ctx.overflow.is_some() && ctx.overflow_suffix.is_none() && !TEXT_EDITABLE_VAR.get() {
                        match TEXT_OVERFLOW_VAR.get() {
                            TextOverflow::Truncate(suf) if !suf.is_empty() => {
                                let suf = SegmentedText::new(suf, self.shaping_args.direction);
                                let suf = ctx.fonts.shape_text(&suf, &self.shaping_args);

                                ctx.overflow = ctx.shaped_text.overflow_info(max_size, suf.size().width);
                                ctx.overflow_suffix = Some(suf);
                            }
                            _ => {}
                        }
                    }
                } else {
                    ctx.overflow = None;
                }
            }
            if self.pending.contains(PendingLayout::OVERLINE) {
                if ctx.overline_thickness > Px(0) {
                    ctx.overlines = ctx.shaped_text.lines().map(|l| l.overline()).collect();
                } else {
                    ctx.overlines = vec![];
                }
            }
            if self.pending.contains(PendingLayout::STRIKETHROUGH) {
                if ctx.strikethrough_thickness > Px(0) {
                    ctx.strikethroughs = ctx.shaped_text.lines().map(|l| l.strikethrough()).collect();
                } else {
                    ctx.strikethroughs = vec![];
                }
            }

            if self.pending.contains(PendingLayout::UNDERLINE) {
                let ime_range = if let Some(ime) = &resolved.ime_preview {
                    let start = ime.prev_selection.unwrap_or(ime.prev_caret).index.min(ime.prev_caret.index);
                    start..start + ime.txt.len()
                } else {
                    0..0
                };
                let caret_ime_range = if !ime_range.is_empty() && (ctx.underline_thickness > Px(0) || ctx.ime_underline_thickness > Px(0)) {
                    let start = ctx.shaped_text.snap_caret_line(CaretIndex {
                        index: ime_range.start,
                        line: 0,
                    });
                    let end = ctx.shaped_text.snap_caret_line(CaretIndex {
                        index: ime_range.end,
                        line: 0,
                    });

                    start..end
                } else {
                    CaretIndex::ZERO..CaretIndex::ZERO
                };

                if ctx.underline_thickness > Px(0) {
                    let mut underlines = vec![];

                    let skip = UNDERLINE_SKIP_VAR.get();
                    match UNDERLINE_POSITION_VAR.get() {
                        UnderlinePosition::Font => {
                            if skip == UnderlineSkip::GLYPHS | UnderlineSkip::SPACES {
                                for line in ctx.shaped_text.lines() {
                                    for und in line.underline_skip_glyphs_and_spaces(ctx.underline_thickness) {
                                        underlines.push(und);
                                    }
                                }
                            } else if skip.contains(UnderlineSkip::GLYPHS) {
                                for line in ctx.shaped_text.lines() {
                                    for und in line.underline_skip_glyphs(ctx.underline_thickness) {
                                        underlines.push(und);
                                    }
                                }
                            } else if skip.contains(UnderlineSkip::SPACES) {
                                for line in ctx.shaped_text.lines() {
                                    for und in line.underline_skip_spaces() {
                                        underlines.push(und);
                                    }
                                }
                            } else {
                                for line in ctx.shaped_text.lines() {
                                    let und = line.underline();
                                    underlines.push(und);
                                }
                            }
                        }
                        UnderlinePosition::Descent => {
                            // descent clears all glyphs, so we only need to care about spaces
                            if skip.contains(UnderlineSkip::SPACES) {
                                for line in ctx.shaped_text.lines() {
                                    for und in line.underline_descent_skip_spaces() {
                                        underlines.push(und);
                                    }
                                }
                            } else {
                                for line in ctx.shaped_text.lines() {
                                    let und = line.underline_descent();
                                    underlines.push(und);
                                }
                            }
                        }
                    }

                    if !ime_range.is_empty() {
                        underlines = ctx.shaped_text.clip_lines(
                            caret_ime_range.clone(),
                            true,
                            resolved.segmented_text.text(),
                            underlines.into_iter(),
                        );
                    }

                    ctx.underlines = underlines;
                } else {
                    ctx.underlines = vec![];
                }

                if ctx.ime_underline_thickness > Px(0) && !ime_range.is_empty() {
                    let mut ime_underlines = vec![];

                    // collects underlines for all segments that intersect with the IME text.
                    for line in ctx.shaped_text.lines() {
                        let line_range = line.text_range();
                        if line_range.start < ime_range.end && line_range.end > ime_range.start {
                            for seg in line.segs() {
                                let seg_range = seg.text_range();
                                if seg_range.start < ime_range.end && seg_range.end > ime_range.start {
                                    for und in seg.underline_skip_glyphs(ctx.ime_underline_thickness) {
                                        ime_underlines.push(und);
                                    }
                                }
                            }
                        }
                    }

                    ctx.ime_underlines =
                        ctx.shaped_text
                            .clip_lines(caret_ime_range, false, resolved.segmented_text.text(), ime_underlines.into_iter());
                } else {
                    ctx.ime_underlines = vec![];
                }
            }

            if self.pending.contains(PendingLayout::CARET) {
                drop(resolved);
                let mut resolved_mut = TEXT.resolve();
                let resolved_mut = &mut *resolved_mut;
                let caret = &mut resolved_mut.caret;
                if let Some(index) = &mut caret.index {
                    *index = ctx.shaped_text.snap_caret_line(*index);

                    let p = ctx.shaped_text.caret_origin(*index, resolved_mut.segmented_text.text());
                    if !caret.used_retained_x {
                        ctx.caret_retained_x = p.x;
                    }
                    ctx.caret_origin = Some(p);

                    if let Some(sel) = &mut caret.selection_index {
                        *sel = ctx.shaped_text.snap_caret_line(*sel);
                        ctx.caret_selection_origin = Some(ctx.shaped_text.caret_origin(*sel, resolved_mut.segmented_text.text()));
                    }

                    if !mem::take(&mut caret.skip_next_scroll)
                        && SCROLL.try_id().is_some()
                        && let Some(focused) = FOCUS.focused().get()
                        && focused.contains(TEXT.try_rich().map(|r| r.root_id).unwrap_or_else(|| WIDGET.id()))
                    {
                        let line_height = ctx
                            .shaped_text
                            .line(index.line)
                            .map(|l| l.rect().height())
                            .unwrap_or_else(|| ctx.shaped_text.line_height());

                        if let Some(p) = ctx.render_info.transform.transform_point(p) {
                            let p = p - WIDGET.info().inner_bounds().origin;
                            let min_rect = Rect::new(p.to_point(), Size::new(Px(1), line_height * 2 + ctx.shaped_text.line_spacing()));
                            SCROLL.scroll_to(ScrollToMode::minimal_rect(min_rect));
                        }
                    }
                }
            }

            // self.pending is cleared in the node layout, after this method call
        }
        self.txt_is_measured = is_measure;

        metrics.constraints().fill_size_or(ctx.shaped_text.size())
    }

    fn ensure_layout_for_render(&mut self) {
        if self.txt_is_measured {
            let metrics = self.last_layout.0.clone();
            self.shaping_args.inline_constraints = self.last_layout.1;
            LAYOUT.with_context(metrics.clone(), || {
                self.layout(&metrics, false);
            });

            debug_assert!(!self.txt_is_measured);
        }
    }
}

fn layout_text_edit(child: impl UiNode) -> impl UiNode {
    // Use `LayoutTextEdit::get` to access.
    let mut edit = None::<Box<LayoutTextEdit>>;

    match_node(child, move |child, op| {
        let mut enable = false;
        match op {
            UiNodeOp::Init => {
                child.init(); // let other nodes subscribe to events first (needed for `only_alt_...`)
                enable = TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get();
            }
            UiNodeOp::Deinit => {
                edit = None;
            }
            UiNodeOp::Info { info } => {
                if let Some(e) = &edit {
                    info.set_ime_area(e.ime_area.clone());
                }
            }
            UiNodeOp::Event { update } => {
                child.event(update);

                if let Some(e) = &mut edit {
                    layout_text_edit_events(update, e);
                }
            }
            UiNodeOp::Update { .. } => {
                if TEXT_EDITABLE_VAR.is_new() || TEXT_SELECTABLE_VAR.is_new() {
                    enable = TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get();
                    if !enable && edit.is_some() {
                        edit = None;
                    }
                } else if let Some(edit) = &edit {
                    TEXT.resolved().txt.with_new(|t| {
                        edit.select_all.set_enabled(!t.is_empty());
                    });
                }

                if OBSCURE_TXT_VAR.is_new() || OBSCURING_CHAR_VAR.is_new() {
                    if let Some(obscure) = OBSCURE_TXT_VAR.get_new() {
                        if edit.is_none() && WINDOW.info().access_enabled().is_enabled() {
                            WIDGET.info();
                        }

                        if obscure {
                            UNDO.clear();
                        }
                    }
                }
            }
            UiNodeOp::Render { frame } => {
                child.render(frame);
                if let Some(e) = &edit {
                    e.update_ime(&mut TEXT.layout());
                }
            }
            UiNodeOp::RenderUpdate { update } => {
                child.render_update(update);
                if let Some(e) = &edit {
                    e.update_ime(&mut TEXT.layout());
                }
            }
            _ => {}
        }

        if enable {
            let edit = LayoutTextEdit::get(&mut edit);

            let editable = TEXT_EDITABLE_VAR.get();
            let selectable = TEXT_SELECTABLE_VAR.get();

            if selectable || editable {
                let id = WIDGET.id();

                edit.events[0] = MOUSE_INPUT_EVENT.subscribe(id);
                edit.events[1] = TOUCH_TAP_EVENT.subscribe(id);
                edit.events[2] = TOUCH_LONG_PRESS_EVENT.subscribe(id);
                edit.events[3] = TOUCH_INPUT_EVENT.subscribe(id);
                // KEY_INPUT_EVENT subscribed by `resolve_text`.
            } else {
                edit.events = Default::default();
            }

            if selectable {
                let id = WIDGET.id();

                edit.select = SELECT_CMD.scoped(id).subscribe(true);
                let is_empty = TEXT.resolved().txt.with(|t| t.is_empty());
                edit.select_all = SELECT_ALL_CMD.scoped(id).subscribe(!is_empty);
            } else {
                edit.select = Default::default();
                edit.select_all = Default::default();
            }
        }
    })
}
/// Data allocated only when `editable`.
#[derive(Default)]
struct LayoutTextEdit {
    events: [EventHandle; 4],
    caret_animation: VarHandle,
    select: CommandHandle,
    select_all: CommandHandle,
    ime_area: Arc<Atomic<PxRect>>,
    click_count: u8,
    selection_mouse_down: Option<SelectionMouseDown>,
    auto_select: bool,
    selection_move_handles: EventHandles,
    selection_started_by_alt: bool,
}
struct SelectionMouseDown {
    position: DipPoint,
    timestamp: DInstant,
    count: u8,
}
impl LayoutTextEdit {
    fn get(edit_data: &mut Option<Box<Self>>) -> &mut Self {
        &mut *edit_data.get_or_insert_with(Default::default)
    }

    fn update_ime(&self, txt: &mut LaidoutText) {
        let transform = txt.render_info.transform;
        let area;

        if let Some(a) = txt.caret_origin {
            let (ac, bc) = {
                let ctx = TEXT.resolved();
                let c = &ctx.caret;
                (c.index, c.selection_index)
            };
            let ac = ac.unwrap_or(CaretIndex::ZERO);
            let mut a_line = PxRect::new(a, PxSize::new(Px(1), txt.shaped_text.line(ac.line).unwrap().height())).to_box2d();

            if let Some(b) = txt.caret_selection_origin {
                let bc = bc.unwrap_or(CaretIndex::ZERO);
                let b_line = PxRect::new(b, PxSize::new(Px(1), txt.shaped_text.line(bc.line).unwrap().height())).to_box2d();

                a_line.min = a_line.min.min(b_line.min);
                a_line.max = a_line.max.min(b_line.max);
            }
            area = a_line;
        } else {
            area = PxBox::from_size(txt.shaped_text.size());
        }

        if let Some(area) = transform.outer_transformed(area) {
            self.ime_area.store(area.to_rect(), atomic::Ordering::Relaxed);
        }
    }
}

fn layout_text_edit_events(update: &EventUpdate, edit: &mut LayoutTextEdit) {
    let resolved = TEXT.resolved();
    let editable = TEXT_EDITABLE_VAR.get() && resolved.txt.capabilities().can_modify();
    let selectable = TEXT_SELECTABLE_VAR.get();

    if !editable && !selectable {
        return;
    }
    let widget = WIDGET.info();
    if !widget.interactivity().is_enabled() {
        return;
    }

    let selectable_alt_only = selectable && !editable && TEXT_SELECTABLE_ALT_ONLY_VAR.get();

    let prev_caret_index = {
        let caret = &resolved.caret;
        (caret.index, caret.index_version, caret.selection_index)
    };
    drop(resolved);

    if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
        if args.state == KeyState::Pressed {
            if args.target.widget_id() == widget.id() {
                match &args.key {
                    Key::ArrowRight => {
                        let mut modifiers = args.modifiers;
                        let select = selectable && modifiers.take_shift();
                        let word = modifiers.take_ctrl();
                        if modifiers.is_empty() && (editable || select) {
                            args.propagation().stop();

                            TEXT.resolve().selection_by = SelectionBy::Keyboard;

                            if select {
                                if word {
                                    TextSelectOp::select_next_word()
                                } else {
                                    TextSelectOp::select_next()
                                }
                            } else if word {
                                TextSelectOp::next_word()
                            } else {
                                TextSelectOp::next()
                            }
                            .call();
                        }
                    }
                    Key::ArrowLeft => {
                        let mut modifiers = args.modifiers;
                        let select = selectable && modifiers.take_shift();
                        let word = modifiers.take_ctrl();
                        if modifiers.is_empty() && (editable || select) {
                            args.propagation().stop();

                            TEXT.resolve().selection_by = SelectionBy::Keyboard;

                            if select {
                                if word {
                                    TextSelectOp::select_prev_word()
                                } else {
                                    TextSelectOp::select_prev()
                                }
                            } else if word {
                                TextSelectOp::prev_word()
                            } else {
                                TextSelectOp::prev()
                            }
                            .call();
                        }
                    }
                    Key::ArrowUp => {
                        if ACCEPTS_ENTER_VAR.get() || TEXT.laidout().shaped_text.lines_len() > 1 || TEXT.try_rich().is_some() {
                            let mut modifiers = args.modifiers;
                            let select = selectable && modifiers.take_shift();
                            if modifiers.is_empty() && (editable || select) {
                                args.propagation().stop();

                                TEXT.resolve().selection_by = SelectionBy::Keyboard;

                                if select {
                                    TextSelectOp::select_line_up()
                                } else {
                                    TextSelectOp::line_up()
                                }
                                .call();
                            }
                        }
                    }
                    Key::ArrowDown => {
                        if ACCEPTS_ENTER_VAR.get() || TEXT.laidout().shaped_text.lines_len() > 1 || TEXT.try_rich().is_some() {
                            let mut modifiers = args.modifiers;
                            let select = selectable && modifiers.take_shift();
                            if modifiers.is_empty() && (editable || select) {
                                args.propagation().stop();

                                TEXT.resolve().selection_by = SelectionBy::Keyboard;

                                if select {
                                    TextSelectOp::select_line_down()
                                } else {
                                    TextSelectOp::line_down()
                                }
                                .call();
                            }
                        }
                    }
                    Key::PageUp => {
                        if ACCEPTS_ENTER_VAR.get() || TEXT.laidout().shaped_text.lines_len() > 1 || TEXT.try_rich().is_some() {
                            let mut modifiers = args.modifiers;
                            let select = selectable && modifiers.take_shift();
                            if modifiers.is_empty() && (editable || select) {
                                args.propagation().stop();

                                TEXT.resolve().selection_by = SelectionBy::Keyboard;

                                if select {
                                    TextSelectOp::select_page_up()
                                } else {
                                    TextSelectOp::page_up()
                                }
                                .call();
                            }
                        }
                    }
                    Key::PageDown => {
                        if ACCEPTS_ENTER_VAR.get() || TEXT.laidout().shaped_text.lines_len() > 1 || TEXT.try_rich().is_some() {
                            let mut modifiers = args.modifiers;
                            let select = selectable && modifiers.take_shift();
                            if modifiers.is_empty() && (editable || select) {
                                args.propagation().stop();

                                TEXT.resolve().selection_by = SelectionBy::Keyboard;

                                if select {
                                    TextSelectOp::select_page_down()
                                } else {
                                    TextSelectOp::page_down()
                                }
                                .call();
                            }
                        }
                    }
                    Key::Home => {
                        let mut modifiers = args.modifiers;
                        let select = selectable && modifiers.take_shift();
                        let full_text = modifiers.take_ctrl();
                        if modifiers.is_empty() && (editable || select) {
                            args.propagation().stop();

                            TEXT.resolve().selection_by = SelectionBy::Keyboard;

                            if select {
                                if full_text {
                                    TextSelectOp::select_text_start()
                                } else {
                                    TextSelectOp::select_line_start()
                                }
                            } else if full_text {
                                TextSelectOp::text_start()
                            } else {
                                TextSelectOp::line_start()
                            }
                            .call();
                        }
                    }
                    Key::End => {
                        let mut modifiers = args.modifiers;
                        let select = selectable && modifiers.take_shift();
                        let full_text = modifiers.take_ctrl();
                        if modifiers.is_empty() && (editable || select) {
                            args.propagation().stop();

                            TEXT.resolve().selection_by = SelectionBy::Keyboard;

                            if select {
                                if full_text {
                                    TextSelectOp::select_text_end()
                                } else {
                                    TextSelectOp::select_line_end()
                                }
                            } else if full_text {
                                TextSelectOp::text_end()
                            } else {
                                TextSelectOp::line_end()
                            }
                            .call();
                        }
                    }
                    Key::Escape => {
                        if args.modifiers.is_empty() && (editable || selectable) {
                            args.propagation().stop();
                            TEXT.resolve().selection_by = SelectionBy::Keyboard;

                            TextSelectOp::clear_selection().call();
                        }
                    }
                    _ => {}
                }
            }
        } else if let Key::Alt | Key::AltGraph = &args.key {
            if TEXT.try_rich().is_some() {
                if TEXT.take_rich_selection_started_by_alt() {
                    args.propagation().stop();
                }
            } else if mem::take(&mut edit.selection_started_by_alt) {
                args.propagation().stop();
            }
        }
    } else if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update) {
        if args.is_primary() && args.is_mouse_down() && args.target.widget_id() == widget.id() {
            let mut modifiers = args.modifiers;
            let alt = modifiers.take_alt();
            let select = selectable && modifiers.take_shift();

            if modifiers.is_empty() && (!selectable_alt_only || alt) {
                args.propagation().stop();
                TEXT.resolve().selection_by = SelectionBy::Mouse;
                if alt {
                    if TEXT.try_rich().is_some() {
                        TEXT.flag_rich_selection_started_by_alt();
                    } else {
                        edit.selection_started_by_alt = true;
                    }
                }

                edit.click_count = if let Some(info) = &mut edit.selection_mouse_down {
                    let cfg = MOUSE.multi_click_config().get();

                    let double_allowed = args.timestamp.duration_since(info.timestamp) <= cfg.time && {
                        let dist = (info.position.to_vector() - args.position.to_vector()).abs();
                        let area = cfg.area;
                        dist.x <= area.width && dist.y <= area.height
                    };

                    if double_allowed {
                        info.timestamp = args.timestamp;
                        info.count += 1;
                        info.count = info.count.min(4);
                    } else {
                        *info = SelectionMouseDown {
                            position: args.position,
                            timestamp: args.timestamp,
                            count: 1,
                        };
                    }

                    info.count
                } else {
                    edit.selection_mouse_down = Some(SelectionMouseDown {
                        position: args.position,
                        timestamp: args.timestamp,
                        count: 1,
                    });
                    1
                };

                match edit.click_count {
                    1 => {
                        if select {
                            TextSelectOp::select_nearest_to(args.position).call()
                        } else {
                            TextSelectOp::nearest_to(args.position).call();

                            // select all on mouse-up if only acquire focus
                            edit.auto_select = selectable
                                && AUTO_SELECTION_VAR.get().contains(AutoSelection::ALL_ON_FOCUS_POINTER)
                                && !FOCUS.is_focused(widget.id()).get()
                                && TEXT.resolved().caret.selection_range().is_none();
                        }
                    }
                    2 => {
                        if selectable {
                            TextSelectOp::select_word_nearest_to(!select, args.position).call()
                        }
                    }
                    3 => {
                        if selectable {
                            TextSelectOp::select_line_nearest_to(!select, args.position).call()
                        }
                    }
                    4 => {
                        if selectable {
                            TextSelectOp::select_all().call()
                        }
                    }
                    _ => unreachable!(),
                };
                if selectable {
                    let id = widget.id();
                    edit.selection_move_handles.push(MOUSE_MOVE_EVENT.subscribe(id));
                    edit.selection_move_handles.push(POINTER_CAPTURE_EVENT.subscribe(id));
                    POINTER_CAPTURE.capture_widget(id);
                }
            }
        } else {
            if mem::take(&mut edit.auto_select)
                && selectable
                && AUTO_SELECTION_VAR.get().contains(AutoSelection::ALL_ON_FOCUS_POINTER)
                && args.is_primary()
                && args.is_mouse_up()
                && FOCUS.is_focused(widget.id()).get()
                && TEXT.resolved().caret.selection_range().is_none()
            {
                TextSelectOp::select_all().call()
            }
            edit.selection_move_handles.clear();
        }
    } else if let Some(args) = TOUCH_INPUT_EVENT.on_unhandled(update) {
        let mut modifiers = args.modifiers;
        let alt = modifiers.take_alt();
        if modifiers.is_empty() && (!selectable_alt_only || alt) && args.target.widget_id() == widget.id() {
            edit.auto_select = selectable
                && AUTO_SELECTION_VAR.get().contains(AutoSelection::ALL_ON_FOCUS_POINTER)
                && args.modifiers.is_empty()
                && args.is_touch_start()
                && !FOCUS.is_focused(widget.id()).get()
                && TEXT.resolved().caret.selection_range().is_none();
        }
    } else if let Some(args) = TOUCH_TAP_EVENT.on_unhandled(update) {
        let mut modifiers = args.modifiers;
        let alt = modifiers.take_alt();
        if modifiers.is_empty() && (!selectable_alt_only || alt) && args.target.widget_id() == widget.id() {
            args.propagation().stop();

            TEXT.resolve().selection_by = SelectionBy::Touch;
            if alt {
                if TEXT.try_rich().is_some() {
                    TEXT.flag_rich_selection_started_by_alt();
                } else {
                    edit.selection_started_by_alt = true;
                }
            }

            TextSelectOp::nearest_to(args.position).call();

            if mem::take(&mut edit.auto_select)
                && selectable
                && AUTO_SELECTION_VAR.get().contains(AutoSelection::ALL_ON_FOCUS_POINTER)
                && FOCUS.is_focused(WIDGET.id()).get()
                && TEXT.resolved().caret.selection_range().is_none()
            {
                TextSelectOp::select_all().call()
            }
        }
    } else if let Some(args) = TOUCH_LONG_PRESS_EVENT.on_unhandled(update) {
        let mut modifiers = args.modifiers;
        let alt = modifiers.take_alt();
        if modifiers.is_empty() && (!selectable_alt_only || alt) && selectable && args.target.widget_id() == widget.id() {
            args.propagation().stop();

            TEXT.resolve().selection_by = SelectionBy::Touch;
            if alt {
                if TEXT.try_rich().is_some() {
                    TEXT.flag_rich_selection_started_by_alt();
                } else {
                    edit.selection_started_by_alt = true;
                }
            }

            TextSelectOp::select_word_nearest_to(true, args.position).call();
        }
    } else if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
        if !edit.selection_move_handles.is_dummy() && selectable {
            let handle = if let Some(rich_root_id) = TEXT.try_rich().map(|r| r.root_id) {
                args.target.contains(rich_root_id)
            } else {
                args.target.widget_id() == widget.id()
            };

            if handle {
                args.propagation().stop();

                match edit.click_count {
                    1 => TextSelectOp::select_nearest_to(args.position).call(),
                    2 => TextSelectOp::select_word_nearest_to(false, args.position).call(),
                    3 => TextSelectOp::select_line_nearest_to(false, args.position).call(),
                    4 => {}
                    _ => unreachable!(),
                }
            }
        }
    } else if let Some(args) = POINTER_CAPTURE_EVENT.on(update) {
        if args.is_lost(widget.id()) {
            edit.selection_move_handles.clear();
            edit.auto_select = false;
        }
    } else if selectable {
        if let Some(args) = SELECT_CMD.scoped(widget.id()).on_unhandled(update) {
            if let Some(op) = args.param::<TextSelectOp>() {
                args.propagation().stop();
                op.clone().call();
            }
        } else if let Some(args) = SELECT_ALL_CMD.scoped(widget.id()).on_unhandled(update) {
            args.propagation().stop();
            TextSelectOp::select_all().call();
        }
    }

    let mut resolve = TEXT.resolve();
    let caret = &mut resolve.caret;
    if (caret.index, caret.index_version, caret.selection_index) != prev_caret_index {
        if !editable || caret.index.is_none() || !FOCUS.is_focused(widget.id()).get() {
            edit.caret_animation = VarHandle::dummy();
            caret.opacity = var(0.fct()).read_only();
        } else {
            caret.opacity = KEYBOARD.caret_animation();
            edit.caret_animation = caret.opacity.subscribe(UpdateOp::RenderUpdate, widget.id());
        }
        resolve.pending_layout |= PendingLayout::CARET;
        WIDGET.layout(); // update caret_origin
    }
}
