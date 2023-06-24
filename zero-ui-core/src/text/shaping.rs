use std::{
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    mem, ops,
};

use super::{
    font_features::RFontFeatures, lang, Font, FontList, GlyphIndex, GlyphInstance, Hyphenation, Hyphens, Lang, LineBreak, SegmentedText,
    TextSegment, TextSegmentKind, Txt, WordBreak,
};
use crate::{
    context::{InlineConstraintsLayout, InlineConstraintsMeasure, LayoutDirection},
    crate_util::{f32_cmp, IndexRange},
    text::BidiLevel,
    units::*,
    widget_info::{InlineSegmentInfo, InlineSegmentPos},
};

pub use font_kit::error::GlyphLoadingError;
use zero_ui_view_api::webrender_api::units::LayoutVector2D;

/// Extra configuration for [`shape_text`](Font::shape_text).
#[derive(Debug, Clone)]
pub struct TextShapingArgs {
    /// Extra spacing to add after each character.
    pub letter_spacing: Px,

    /// Extra spacing to add after each space (U+0020 SPACE).
    pub word_spacing: Px,

    /// Height of each line.
    ///
    /// Default can be computed using [`FontMetrics::line_height`].
    ///
    /// [`FontMetrics::line_height`]: crate::text::FontMetrics::line_height
    pub line_height: Px,

    /// Extra spacing added in between lines.
    pub line_spacing: Px,

    /// Primary language of the text.
    pub lang: Lang,

    /// Text flow direction.
    pub direction: LayoutDirection,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

    /// Width of the TAB character.
    pub tab_x_advance: Px,

    /// Inline constraints for initial text shaping and wrap.
    pub inline_constraints: Option<InlineConstraintsMeasure>,

    /// Finalized font features.
    pub font_features: RFontFeatures,

    /// Maximum line width.
    ///
    /// Is [`Px::MAX`] when text wrap is disabled.
    pub max_width: Px,

    /// Line break config for Chinese, Japanese, or Korean text.
    pub line_break: LineBreak,

    /// World break config.
    ///
    /// This value is only considered if it is impossible to fit the a word to a line.
    pub word_break: WordBreak,

    /// Hyphen breaks config.
    pub hyphens: Hyphens,

    /// Character rendered when text is auto-hyphenated.
    pub hyphen_char: Txt,
}
impl Default for TextShapingArgs {
    fn default() -> Self {
        TextShapingArgs {
            letter_spacing: Px(0),
            word_spacing: Px(0),
            line_height: Px(0),
            line_spacing: Px(0),
            lang: lang!(und),
            direction: LayoutDirection::LTR,
            ignore_ligatures: false,
            disable_kerning: false,
            tab_x_advance: Px(0),
            inline_constraints: None,
            font_features: RFontFeatures::default(),
            max_width: Px::MAX,
            line_break: Default::default(),
            word_break: Default::default(),
            hyphens: Default::default(),
            hyphen_char: Txt::from_char('-'),
        }
    }
}

/// Defines a range of segments in a [`ShapedText`] that form a line.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LineRange {
    /// Exclusive segment index, is the `segments.len()` for the last line and the index of the first
    /// segment after the line break for other lines.
    end: usize,
    /// Pixel width of the line.
    width: f32,
    /// Applied align offset to the right.
    x_offset: f32,
}

/// Defines the font of a range of glyphs in a [`ShapedText`].
#[derive(Clone)]
struct FontRange {
    font: Font,
    /// Exclusive glyph range end.
    end: usize,
}
impl PartialEq for FontRange {
    fn eq(&self, other: &Self) -> bool {
        self.font == other.font && self.end == other.end
    }
}
impl fmt::Debug for FontRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontInfo")
            .field("font", &self.font.face().display_name().name())
            .field("end", &self.end)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GlyphSegment {
    pub text: TextSegment,
    /// glyph exclusive end.
    pub end: usize,
    /// Segment offset in the line.
    pub x: f32,
    /// Advance/width of segment.
    pub advance: f32,
}

/// `Vec<GlyphSegment>` with helper methods.
#[derive(Debug, Default, Clone, PartialEq)]
struct GlyphSegmentVec(Vec<GlyphSegment>);
impl GlyphSegmentVec {
    /// Exclusive glyphs range of the segment.
    fn glyphs(&self, index: usize) -> IndexRange {
        let start = if index == 0 { 0 } else { self.0[index - 1].end };
        let end = self.0[index].end;
        IndexRange(start, end)
    }

    /// Exclusive glyphs range from an exclusive range of segments.
    fn glyphs_range(&self, range: IndexRange) -> IndexRange {
        let IndexRange(start, end) = range;

        if end == 0 {
            return IndexRange(0, 0);
        }

        let start = if start == 0 { 0 } else { self.0[start - 1].end };
        let end = self.0[end - 1].end;

        IndexRange(start, end)
    }
}

/// `Vec<LineRange>` with helper methods.
#[derive(Debug, Default, Clone, PartialEq)]
struct LineRangeVec(Vec<LineRange>);
impl LineRangeVec {
    /// Exclusive segments range of the line.
    fn segs(&self, index: usize) -> IndexRange {
        let end = self.0[index].end;
        let start = if index == 0 { 0 } else { self.0[index - 1].end };
        IndexRange(start, end)
    }

    /// Line width.
    fn width(&self, index: usize) -> f32 {
        self.0[index].width
    }

    /// Line x offset.
    fn x_offset(&self, index: usize) -> f32 {
        self.0[index].x_offset
    }

    /// Iter segment ranges.
    fn iter_segs(&self) -> impl Iterator<Item = (f32, IndexRange)> + '_ {
        self.iter_segs_skip(0)
    }

    /// Iter segment ranges starting at a line.
    fn iter_segs_skip(&self, start_line: usize) -> impl Iterator<Item = (f32, IndexRange)> + '_ {
        let mut start = self.segs(start_line).start();
        self.0[start_line..].iter().map(move |l| {
            let r = IndexRange(start, l.end);
            start = l.end;
            (l.width, r)
        })
    }

    /// Returns `true` if there is more then one line.
    fn is_multi(&self) -> bool {
        self.0.len() > 1
    }

    fn first_mut(&mut self) -> &mut LineRange {
        &mut self.0[0]
    }

    fn last(&self) -> LineRange {
        self.0[self.0.len() - 1]
    }

    fn last_mut(&mut self) -> &mut LineRange {
        let l = self.0.len() - 1;
        &mut self.0[l]
    }
}

/// `Vec<FontRange>` with helper methods.
#[derive(Debug, Default, Clone, PartialEq)]
struct FontRangeVec(Vec<FontRange>);
impl FontRangeVec {
    /// Iter glyph ranges.
    fn iter_glyphs(&self) -> impl Iterator<Item = (&Font, IndexRange)> + '_ {
        let mut start = 0;
        self.0.iter().map(move |f| {
            let r = IndexRange(start, f.end);
            start = f.end;
            (&f.font, r)
        })
    }

    /// Iter glyph ranges clipped by `glyphs_range`.
    fn iter_glyphs_clip(&self, glyphs_range: IndexRange) -> impl Iterator<Item = (&Font, IndexRange)> + '_ {
        let mut start = glyphs_range.start();
        let end = glyphs_range.end();
        let first_font = self.0.iter().position(|f| f.end > start).unwrap_or(self.0.len().saturating_sub(1));

        self.0[first_font..].iter().map_while(move |f| {
            let i = f.end.min(end);

            if i > start {
                let r = IndexRange(start, i);
                start = i;
                Some((&f.font, r))
            } else {
                None
            }
        })
    }

    /// Returns a reference to the font.
    fn font(&self, index: usize) -> &Font {
        &self.0[index].font
    }
}

/// Output of [text layout].
///
/// [text layout]: Font::shape_text
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText {
    glyphs: Vec<GlyphInstance>,
    clusters: Vec<u32>, // char byte index of each glyph in the segment that covers it.
    // segments of `glyphs` and `clusters`.
    segments: GlyphSegmentVec,
    lines: LineRangeVec,
    fonts: FontRangeVec,

    line_height: Px,
    line_spacing: Px,

    orig_line_height: Px,
    orig_line_spacing: Px,
    orig_first_line: PxSize,
    orig_last_line: PxSize,

    // offsets from the line_height bottom
    baseline: Px,
    overline: Px,
    strikethrough: Px,
    underline: Px,
    underline_descent: Px,

    /// vertical align offset applied.
    mid_offset: f32,
    align_size: PxSize,
    align: Align,
    direction: LayoutDirection,

    // inline layout values
    is_inlined: bool,
    first_wrapped: bool,
    first_line: PxRect,
    mid_clear: Px,
    mid_size: PxSize,
    last_line: PxRect,

    has_colored_glyphs: bool,
}

/// Represents normal and colored glyphs in [`ShapedText::colored_glyphs`].
pub enum ShapedColoredGlyphs<'a> {
    /// Sequence of not colored glyphs, use the base color to fill.
    Normal(&'a [GlyphInstance]),
    /// Colored glyph.
    Colored {
        /// Point that must be used for all `glyphs`.
        point: zero_ui_view_api::webrender_api::units::LayoutPoint,
        /// The glyph that is replaced by `glyphs`.
        ///
        /// Must be used as fallback if any `glyphs` cannot be rendered.
        base_glyph: GlyphIndex,

        /// The colored glyph components.
        glyphs: super::ColorGlyph<'a>,
    },
}

impl ShapedText {
    /// New empty text.
    pub fn new(font: &Font) -> Self {
        font.shape_text(&SegmentedText::new("", LayoutDirection::LTR), &TextShapingArgs::default())
    }

    /// Glyphs by font.
    pub fn glyphs(&self) -> impl Iterator<Item = (&Font, &[GlyphInstance])> {
        self.fonts.iter_glyphs().map(move |(f, r)| (f, &self.glyphs[r.iter()]))
    }

    /// If the shaped text has any Emoji glyph associated with a font that has color palettes.
    pub fn has_colored_glyphs(&self) -> bool {
        self.has_colored_glyphs
    }

    /// Glyphs by font and palette color.
    pub fn colored_glyphs(&self) -> impl Iterator<Item = (&Font, ShapedColoredGlyphs)> {
        struct Iter<'a, G>
        where
            G: Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a,
        {
            glyphs: G,
            maybe_colored: Option<(&'a Font, &'a [GlyphInstance])>,
        }
        impl<'a, G> Iterator for Iter<'a, G>
        where
            G: Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a,
        {
            type Item = (&'a Font, ShapedColoredGlyphs<'a>);

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    if let Some((font, glyphs)) = self.maybe_colored {
                        // maybe-colored iter

                        let color_glyphs = font.face().color_glyphs();

                        for (i, g) in glyphs.iter().enumerate() {
                            if let Some(c_glyphs) = color_glyphs.glyph(g.index) {
                                // colored yield

                                let next_start = i + 1;
                                if next_start < glyphs.len() {
                                    // continue maybe-colored iter
                                    self.maybe_colored = Some((font, &glyphs[next_start..]));
                                } else {
                                    // continue normal iter
                                    self.maybe_colored = None;
                                }

                                return Some((
                                    font,
                                    ShapedColoredGlyphs::Colored {
                                        point: g.point,
                                        base_glyph: g.index,
                                        glyphs: c_glyphs,
                                    },
                                ));
                            }
                        }
                        // enter normal iter
                        self.maybe_colored = None;

                        // last normal in maybe-colored yield
                        debug_assert!(!glyphs.is_empty());
                        return Some((font, ShapedColoredGlyphs::Normal(glyphs)));
                    } else if let Some((font, glyphs)) = self.glyphs.next() {
                        // normal iter

                        let color_glyphs = font.face().color_glyphs();
                        if color_glyphs.is_empty() {
                            return Some((font, ShapedColoredGlyphs::Normal(glyphs)));
                        } else {
                            // enter maybe-colored iter
                            self.maybe_colored = Some((font, glyphs));
                            continue;
                        }
                    } else {
                        return None;
                    }
                }
            }
        }
        Iter {
            glyphs: self.glyphs(),
            maybe_colored: None,
        }
    }

    /// Glyphs by font in the range.
    fn glyphs_range(&self, range: IndexRange) -> impl Iterator<Item = (&Font, &[GlyphInstance])> {
        self.fonts.iter_glyphs_clip(range).map(|(f, r)| (f, &self.glyphs[r.iter()]))
    }

    /// Index of each char byte in the segment range.
    /// The first char in the segment is 0.
    fn clusters_range(&self, range: IndexRange) -> &[u32] {
        &self.clusters[range.iter()]
    }

    /// Glyphs by font in the range, each glyph instance is paired with the *x-advance* to the next glyph or line end.
    fn glyphs_with_x_advance_range(
        &self,
        line_index: usize,
        glyph_range: IndexRange,
    ) -> impl Iterator<Item = (&Font, impl Iterator<Item = (GlyphInstance, f32)> + '_)> + '_ {
        let mut start = glyph_range.start();
        let segs_range = self.lines.segs(line_index);
        let line_end = self.segments.glyphs_range(segs_range).end();
        let line_width = self.lines.width(line_index);
        self.glyphs_range(glyph_range).map(move |(font, glyphs)| {
            let glyphs_with_adv = glyphs.iter().enumerate().map(move |(i, g)| {
                let gi = start + i + 1;

                let adv = if gi == line_end {
                    line_width - g.point.x
                } else {
                    self.glyphs[gi].point.x - g.point.x
                };

                (*g, adv)
            });

            start += glyphs.len();

            (font, glyphs_with_adv)
        })
    }

    /// Bounding box size, the width is the longest line or the right-most point of first and
    /// last line, the height is the bottom-most point of the last line.
    pub fn size(&self) -> PxSize {
        self.mid_size().max(PxSize::new(
            self.first_line.max_x().max(self.last_line.max_x()),
            self.last_line.max_y(),
        ))
    }

    /// Size of the text, if it is not inlined.
    pub fn block_size(&self) -> PxSize {
        if self.lines.0.is_empty() {
            PxSize::zero()
        } else if self.lines.0.len() == 1 {
            self.first_line.size
        } else {
            let mut s = PxSize::new(
                self.first_line.size.width.max(self.last_line.size.width),
                self.first_line.size.height + self.line_spacing + self.last_line.size.height,
            );
            if self.lines.0.len() > 2 {
                s.width = s.width.max(self.mid_size.width);
                s.height += self.mid_size.height + self.line_spacing;
            }
            s
        }
    }

    fn update_mid_size(&mut self) {
        self.mid_size = if self.lines.0.len() <= 2 {
            PxSize::zero()
        } else {
            let mid_lines = &self.lines.0[1..self.lines.0.len() - 1];
            PxSize::new(
                Px(mid_lines.iter().map(|l| l.width).max_by(f32_cmp).unwrap_or_default().ceil() as i32),
                Px(mid_lines.len() as i32) * self.line_height + Px((mid_lines.len() - 1) as i32) * self.line_spacing,
            )
        };
    }

    fn update_first_last_lines(&mut self) {
        if self.lines.0.is_empty() {
            self.first_line = PxRect::zero();
            self.last_line = PxRect::zero();
            self.align_size = PxSize::zero();
        } else {
            self.first_line = PxRect::from_size(PxSize::new(Px(self.lines.first_mut().width.ceil() as i32), self.line_height));

            if self.lines.0.len() > 1 {
                self.last_line.size = PxSize::new(Px(self.lines.last().width.ceil() as i32), self.line_height);
                self.last_line.origin = PxPoint::new(Px(0), self.first_line.max_y() + self.line_spacing);
                if self.lines.0.len() > 2 {
                    self.last_line.origin.y += self.mid_size.height + self.line_spacing;
                }
            } else {
                self.last_line = self.first_line;
            }
            self.align_size = self.block_size();
        }
    }

    /// Bounding box of the mid-lines, that is the lines except the first and last.
    pub fn mid_size(&self) -> PxSize {
        self.mid_size
    }

    /// If the text first and last lines is defined externally by the inline layout.
    ///
    /// When this is `true` the shaped text only defines aligns horizontally and only the mid-lines. The vertical
    /// offset is defined by the the first line rectangle plus the [`mid_clear`].
    ///
    /// [`mid_clear`]: Self::mid_clear
    pub fn is_inlined(&self) -> bool {
        self.is_inlined
    }

    /// Last applied alignment.
    ///
    /// If the text is inlined only the mid-lines are aligned, and only horizontally.
    pub fn align(&self) -> Align {
        self.align
    }

    /// Last applied alignment area.
    ///
    /// The lines are aligned inside this size. If the text is inlined only the mid-lines are aligned and only horizontally.
    pub fn align_size(&self) -> PxSize {
        self.align_size
    }

    /// Last applied alignment direction.
    ///
    /// Note that the glyph and word directions is defined by the [`TextShapingArgs::lang`].
    pub fn direction(&self) -> LayoutDirection {
        self.direction
    }

    /// Last applied extra spacing between the first and second lines to clear the full width of the second line in the
    /// parent inline layout.
    pub fn mid_clear(&self) -> Px {
        self.mid_clear
    }

    /// Reshape text lines.
    ///
    /// Reshape text lines without re-wrapping, this is more efficient then fully reshaping every glyph, but may
    /// cause overflow if called with constraints incompatible with the ones used during the full text shaping.
    ///
    /// The general process of shaping text is to generate a shaped-text without align during *measure*, and then reuse
    /// this shaped text every layout that does not invalidate any property that affects the text wrap.
    #[allow(clippy::too_many_arguments)]
    pub fn reshape_lines(
        &mut self,
        constraints: PxConstraints2d,
        inline_constraints: Option<InlineConstraintsLayout>,
        align: Align,
        line_height: Px,
        line_spacing: Px,
        direction: LayoutDirection,
    ) {
        self.reshape_line_height_and_spacing(line_height, line_spacing);

        let is_inlined = inline_constraints.is_some();

        let align_x = align.x(direction);
        let align_y = if is_inlined { 0.fct() } else { align.y() };

        let (first, mid, last, first_segs, last_segs) = if let Some(l) = &inline_constraints {
            (l.first, l.mid_clear, l.last, &*l.first_segs, &*l.last_segs)
        } else {
            // calculate our own first & last
            let block_size = self.block_size();
            let align_size = constraints.fill_size_or(block_size);

            let mut first = PxRect::from_size(self.line(0).map(|l| l.rect().size).unwrap_or_default());
            let mut last = PxRect::from_size(
                self.line(self.lines_len().saturating_sub(1))
                    .map(|l| l.rect().size)
                    .unwrap_or_default(),
            );
            last.origin.y = block_size.height - last.size.height;

            first.origin.x = (align_size.width - first.size.width) * align_x;
            last.origin.x = (align_size.width - last.size.width) * align_x;

            let align_y = (align_size.height - block_size.height) * align_y;
            first.origin.y += align_y;
            last.origin.y += align_y;

            static EMPTY: Vec<InlineSegmentPos> = vec![];
            (first, Px(0), last, &EMPTY, &EMPTY)
        };

        if !self.lines.0.is_empty() {
            if self.first_line != first {
                let first_offset = (first.origin - self.first_line.origin).cast::<f32>().cast_unit();

                let first_range = self.lines.segs(0);
                let first_glyphs = self.segments.glyphs_range(first_range);

                for g in &mut self.glyphs[first_glyphs.iter()] {
                    g.point += first_offset;
                }

                let first_line = self.lines.first_mut();
                first_line.x_offset = first.origin.x.0 as f32;
                first_line.width = first.size.width.0 as f32;
            }
            if !first_segs.is_empty() {
                // parent set first_segs.
                let first_range = self.lines.segs(0);
                if first_range.len() == first_segs.len() {
                    for i in first_range.iter() {
                        let seg_offset = first_segs[i].x - self.segments.0[i].x;
                        let glyphs = self.segments.glyphs(i);
                        for g in &mut self.glyphs[glyphs.iter()] {
                            g.point.x += seg_offset;
                        }
                        self.segments.0[i].x = first_segs[i].x;
                    }
                } else {
                    #[cfg(debug_assertions)]
                    {
                        tracing::error!("expected {} segments in `first_segs`, was {}", first_range.len(), first_segs.len());
                    }
                }
            }
        }

        if self.lines.0.len() > 1 {
            if self.last_line != last {
                // last changed and it is not first

                let last_offset = (last.origin - self.last_line.origin).cast::<f32>().cast_unit();

                let last_range = self.lines.segs(self.lines.0.len() - 1);
                let last_glyphs = self.segments.glyphs_range(last_range);

                for g in &mut self.glyphs[last_glyphs.iter()] {
                    g.point += last_offset;
                }

                let last_line = self.lines.last_mut();
                last_line.x_offset = last.origin.x.0 as f32;
                last_line.width = last.size.width.0 as f32;
            }
            if !last_segs.is_empty() {
                // parent set last_segs.
                let last_range = self.lines.segs(self.lines.0.len() - 1);

                if last_range.len() == last_segs.len() {
                    for i in last_range.iter() {
                        let li = i - last_range.start();

                        let seg_offset = last_segs[li].x - self.segments.0[i].x;
                        let glyphs = self.segments.glyphs(i);
                        for g in &mut self.glyphs[glyphs.iter()] {
                            g.point.x += seg_offset;
                        }
                        self.segments.0[i].x = last_segs[li].x;
                    }
                } else {
                    #[cfg(debug_assertions)]
                    {
                        tracing::error!("expected {} segments in `last_segs`, was {}", last_range.len(), last_segs.len());
                    }
                }
            }
        }

        self.first_line = first;
        self.last_line = last;

        let block_size = self.block_size();
        let align_size = constraints.fill_size_or(block_size);

        if self.lines.0.len() > 2 {
            // has mid-lines

            let mid_offset = euclid::vec2::<f32, zero_ui_view_api::webrender_api::units::LayoutPixel>(
                0.0,
                (align_size.height - block_size.height).0 as f32 * align_y + mid.0 as f32,
            );
            let y_transform = mid_offset.y - self.mid_offset;
            let align_width = align_size.width.0 as f32;

            let skip_last = self.lines.0.len() - 2;
            let mut line_start = self.lines.0[0].end;
            for line in &mut self.lines.0[1..=skip_last] {
                let x_offset = (align_width - line.width) * align_x;
                let x_transform = x_offset - line.x_offset;

                let glyphs = self.segments.glyphs_range(IndexRange(line_start, line.end));
                for g in &mut self.glyphs[glyphs.iter()] {
                    g.point.x += x_transform;
                    g.point.y += y_transform;
                }
                line.x_offset = x_offset;

                line_start = line.end;
            }

            let y_transform_px = Px(y_transform as i32);
            self.underline -= y_transform_px;
            self.baseline -= y_transform_px;
            self.overline -= y_transform_px;
            self.strikethrough -= y_transform_px;
            self.underline_descent -= y_transform_px;
            self.mid_offset = mid_offset.y;
        }

        // apply baseline to the content only,
        let baseline_offset =
            if self.align.is_baseline() { -self.baseline } else { Px(0) } + if align.is_baseline() { self.baseline } else { Px(0) };
        if baseline_offset != Px(0) {
            let baseline_offset = baseline_offset.0 as f32;
            for g in &mut self.glyphs {
                g.point.y += baseline_offset;
            }
        }

        self.align_size = align_size;
        self.align = align;
        self.is_inlined = is_inlined;

        self.debug_assert_ranges();
    }
    fn reshape_line_height_and_spacing(&mut self, line_height: Px, line_spacing: Px) {
        let mut update_height = false;

        if self.line_height != line_height {
            let offset_y = (line_height - self.line_height).0 as f32;
            let mut offset = 0.0;
            let center = offset_y / 2.0;

            self.first_line.origin.y += Px(center as i32);

            for (_, r) in self.lines.iter_segs() {
                let r = self.segments.glyphs_range(r);
                for g in &mut self.glyphs[r.iter()] {
                    g.point.y += offset + center;
                }

                offset += offset_y;
            }

            self.line_height = line_height;
            update_height = true;
        }

        if self.line_spacing != line_spacing {
            if self.lines.is_multi() {
                let offset_y = (line_spacing - self.line_spacing).0 as f32;
                let mut offset = offset_y;

                for (_, r) in self.lines.iter_segs_skip(1) {
                    let r = self.segments.glyphs_range(r);

                    for g in &mut self.glyphs[r.iter()] {
                        g.point.y += offset;
                    }

                    offset += offset_y;
                }
                offset -= offset_y;

                self.last_line.origin.y += Px(offset as i32);

                update_height = true;
            }
            self.line_spacing = line_spacing;
        }

        if update_height {
            self.update_mid_size();

            if !self.is_inlined {
                self.update_first_last_lines();
            }
        }
    }

    /// Restore text to initial shape.
    pub fn clear_reshape(&mut self) {
        self.reshape_lines(
            PxConstraints2d::new_fill_size(self.align_size()),
            None,
            Align::TOP_LEFT,
            self.orig_line_height,
            self.orig_line_spacing,
            LayoutDirection::LTR,
        );
    }

    /// Height of a single line.
    pub fn line_height(&self) -> Px {
        self.line_height
    }

    /// Vertical spacing in between lines.
    pub fn line_spacing(&self) -> Px {
        self.line_spacing
    }

    /// Vertical offset from the line bottom up that is the text baseline.
    ///
    /// The *line bottom* is the [`line_height`].
    ///
    /// [`line_height`]: Self::line_height
    pub fn baseline(&self) -> Px {
        self.baseline
    }

    /// Vertical offset from the line bottom up that is the overline placement.
    pub fn overline(&self) -> Px {
        self.overline
    }

    /// Vertical offset from the line bottom up that is the strikethrough placement.
    pub fn strikethrough(&self) -> Px {
        self.strikethrough
    }

    /// Vertical offset from the line bottom up that is the font defined underline placement.
    pub fn underline(&self) -> Px {
        self.underline
    }

    /// Vertical offset from the line bottom up that is the underline placement when the option for
    /// clearing all glyph descents is selected.
    pub fn underline_descent(&self) -> Px {
        self.underline_descent
    }

    /// No segments.
    pub fn is_empty(&self) -> bool {
        self.segments.0.is_empty()
    }

    /// Iterate over [`ShapedLine`] selections split by [`LineBreak`] or wrap.
    ///
    /// [`LineBreak`]: TextSegmentKind::LineBreak
    pub fn lines(&self) -> impl Iterator<Item = ShapedLine> {
        self.lines.iter_segs().enumerate().map(move |(i, (w, r))| ShapedLine {
            text: self,
            seg_range: r,
            index: i,
            width: Px(w.round() as i32),
        })
    }

    /// Returns the number of text lines.
    pub fn lines_len(&self) -> usize {
        self.lines.0.len()
    }

    /// If the first line starts in a new inline row because it could not fit in the leftover inline space.
    pub fn first_wrapped(&self) -> bool {
        self.first_wrapped
    }

    /// Gets the line by index.
    pub fn line(&self, line_idx: usize) -> Option<ShapedLine> {
        if self.lines.0.len() <= line_idx {
            None
        } else {
            self.lines.iter_segs_skip(line_idx).next().map(move |(w, r)| ShapedLine {
                text: self,
                seg_range: r,
                index: line_idx,
                width: Px(w.round() as i32),
            })
        }
    }

    /// Create an empty [`ShapedText`] with the same metrics as `self`.
    pub fn empty(&self) -> ShapedText {
        ShapedText {
            glyphs: vec![],
            clusters: vec![],
            segments: GlyphSegmentVec(vec![]),
            lines: LineRangeVec(vec![LineRange {
                end: 0,
                width: 0.0,
                x_offset: 0.0,
            }]),
            fonts: FontRangeVec(vec![FontRange {
                font: self.fonts.font(0).clone(),
                end: 0,
            }]),
            orig_line_height: self.orig_line_height,
            orig_line_spacing: self.orig_line_spacing,
            orig_first_line: PxSize::zero(),
            orig_last_line: PxSize::zero(),
            line_height: self.orig_line_height,
            line_spacing: self.orig_line_spacing,
            baseline: self.baseline,
            overline: self.overline,
            strikethrough: self.strikethrough,
            underline: self.underline,
            underline_descent: self.underline_descent,
            mid_offset: 0.0,
            align_size: PxSize::zero(),
            align: Align::TOP_LEFT,
            direction: LayoutDirection::LTR,
            first_wrapped: false,
            first_line: PxRect::zero(),
            mid_clear: Px(0),
            is_inlined: false,
            mid_size: PxSize::zero(),
            last_line: PxRect::zero(),
            has_colored_glyphs: false,
        }
    }

    /// Check if any line can be better wrapped given the new wrap config.
    ///
    /// Note that a new [`ShapedText`] must be generated to *rewrap*.
    pub fn can_rewrap(&self, max_width: Px) -> bool {
        for line in self.lines() {
            if line.width > max_width || line.started_by_wrap() {
                return true;
            }
        }
        false
    }

    #[track_caller]
    fn debug_assert_ranges(&self) {
        #[cfg(debug_assertions)]
        {
            let mut prev_seg_end = 0;
            for seg in &self.segments.0 {
                assert!(seg.end >= prev_seg_end);
                prev_seg_end = seg.end;
            }
            trace_assert!(self.segments.0.last().map(|s| s.end == self.glyphs.len()).unwrap_or(true));

            let mut prev_line_end = 0;
            for (i, line) in self.lines.0.iter().enumerate() {
                trace_assert!(line.end >= prev_line_end);
                trace_assert!(line.width >= 0.0);

                let line_max = line.x_offset + line.width;
                let glyphs = self.segments.glyphs_range(IndexRange(prev_line_end, line.end));
                for g in &self.glyphs[glyphs.iter()] {
                    trace_assert!(
                        g.point.x <= line_max,
                        "glyph.x({:?}) > line[{i}].x+width({:?})",
                        g.point.x,
                        line_max
                    );
                }

                let seg_width = self.segments.0[prev_line_end..line.end].iter().map(|s| s.advance).sum::<f32>();
                trace_assert!(
                    seg_width <= line.width,
                    "seg_width({:?}) > line[{i}].width({:?})",
                    seg_width,
                    line.width,
                );

                prev_line_end = line.end;
            }
            trace_assert!(self.lines.0.last().map(|l| l.end == self.segments.0.len()).unwrap_or(true));

            let mut prev_font_end = 0;
            for font in &self.fonts.0 {
                trace_assert!(font.end >= prev_font_end);
                prev_font_end = font.end;
            }
            trace_assert!(self.fonts.0.last().map(|f| f.end == self.glyphs.len()).unwrap_or(true));
        }
    }

    /// Gets the top-left origin for a caret visual that marks the insert `index` in the string.
    pub fn caret_origin(&self, index: usize) -> PxPoint {
        for line in self.lines() {
            for seg in line.segs() {
                let txt_range = seg.text_range();
                if txt_range.contains(index) {
                    let text_start = txt_range.start();

                    let clusters = seg.clusters();
                    let cluster_idx = index - text_start;
                    let mut closest_cluster = 0;
                    let mut search_lig_caret = true;
                    for (i, c) in clusters.iter().enumerate() {
                        match (*c as usize).cmp(&cluster_idx) {
                            std::cmp::Ordering::Less => closest_cluster = i,
                            std::cmp::Ordering::Equal => {
                                closest_cluster = i;
                                search_lig_caret = false;
                                break;
                            }
                            std::cmp::Ordering::Greater => break,
                        }
                    }
                    let closest_cluster = closest_cluster as u32;

                    let mut p = seg.rect().origin;
                    let mut x = p.x.0 as f32;

                    let mut cluster_count = closest_cluster;
                    'outer: for (font, glyph_adv) in seg.glyphs_with_x_advance() {
                        for (glyph, advance) in glyph_adv {
                            if cluster_count == 0 {
                                if search_lig_caret {
                                    let mut font_caret = None;
                                    let mut caret_count = cluster_idx;
                                    for caret in font.ligature_caret_offsets(glyph.index) {
                                        font_caret = Some(caret);
                                        if caret_count == 0 {
                                            break;
                                        }
                                        caret_count -= 1;
                                    }

                                    if let Some(caret) = font_caret {
                                        x += caret;
                                    } else {
                                        let next_cluster = closest_cluster + 1;
                                        let cluster_len = if next_cluster < clusters.len() as u32 {
                                            clusters[next_cluster as usize] - closest_cluster
                                        } else {
                                            txt_range.end() as u32 - closest_cluster
                                        };

                                        let caret = advance * ((cluster_idx as u32 - closest_cluster) as f32 / cluster_len as f32);
                                        x += caret;
                                    }
                                }

                                break 'outer;
                            }
                            x += advance;
                            cluster_count -= 1;
                        }
                    }
                    p.x.0 = x.round() as i32;

                    return p;
                }
            }
        }

        if let Some(line) = self.line(self.lines_len().saturating_sub(1)) {
            // top-right of last line
            let rect = line.rect();
            PxPoint::new(rect.max_x(), rect.min_y())
        } else {
            PxPoint::zero()
        }
    }

    /// Gets the line that contains the `y` offset or is nearest to it.
    pub fn nearest_line(&self, y: Px) -> Option<ShapedLine> {
        let first_line_max_y = self.first_line.max_y();
        if first_line_max_y >= y {
            self.line(0)
        } else if self.last_line.min_y() <= y {
            self.line(self.lines_len().saturating_sub(1))
        } else {
            let y = y - first_line_max_y;
            let line = (y / self.line_height()).0 as usize + 1;
            self.lines.iter_segs_skip(line).next().map(move |(w, r)| ShapedLine {
                text: self,
                seg_range: r,
                index: line,
                width: Px(w.round() as i32),
            })
        }
    }
}

trait FontListRef {
    /// Shape segment, try fallback fonts if a glyph in the segment is not resolved.
    fn shape_segment<R>(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        features: &[harfbuzz_rs::Feature],
        out: impl FnOnce(&ShapedSegmentData, &Font) -> R,
    ) -> R;
}
impl FontListRef for [Font] {
    fn shape_segment<R>(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        features: &[harfbuzz_rs::Feature],
        out: impl FnOnce(&ShapedSegmentData, &Font) -> R,
    ) -> R {
        let mut out = Some(out);
        let last = self.len() - 1;
        for font in &self[..last] {
            let r = font.shape_segment(seg, word_ctx_key, features, |seg| {
                if seg.glyphs.iter().all(|g| g.index != 0) {
                    Some(out.take().unwrap()(seg, font))
                } else {
                    None
                }
            });
            if let Some(r) = r {
                return r;
            }
        }
        self[last].shape_segment(seg, word_ctx_key, features, move |seg| out.unwrap()(seg, &self[last]))
    }
}

struct ShapedTextBuilder {
    out: ShapedText,

    line_height: f32,
    line_spacing: f32,
    word_spacing: f32,
    letter_spacing: f32,
    max_width: f32,
    break_words: bool,
    hyphen_glyphs: (ShapedSegmentData, Font),
    tab_x_advance: f32,
    tab_index: u32,
    hyphens: Hyphens,

    origin: euclid::Point2D<f32, ()>,
    allow_first_wrap: bool,
    first_line_max: f32,
    mid_clear_min: f32,
    max_line_x: f32,
    text_seg_end: usize,
    line_has_ltr: bool,
    line_has_rtl: bool,
}
impl ShapedTextBuilder {
    fn actual_max_width(&self) -> f32 {
        if self.out.lines.0.is_empty() && !self.out.first_wrapped {
            self.first_line_max.min(self.max_width)
        } else {
            self.max_width
        }
    }

    fn shape_text(fonts: &[Font], text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        let mut t = Self {
            out: ShapedText {
                glyphs: Default::default(),
                clusters: Default::default(),
                segments: Default::default(),
                lines: Default::default(),
                fonts: Default::default(),
                line_height: Default::default(),
                line_spacing: Default::default(),
                orig_line_height: Default::default(),
                orig_line_spacing: Default::default(),
                orig_first_line: Default::default(),
                orig_last_line: Default::default(),
                baseline: Default::default(),
                overline: Default::default(),
                strikethrough: Default::default(),
                underline: Default::default(),
                underline_descent: Default::default(),
                mid_offset: 0.0,
                align_size: PxSize::zero(),
                align: Align::TOP_LEFT,
                direction: LayoutDirection::LTR,
                first_wrapped: false,
                is_inlined: config.inline_constraints.is_some(),
                first_line: PxRect::zero(),
                mid_clear: Px(0),
                mid_size: PxSize::zero(),
                last_line: PxRect::zero(),
                has_colored_glyphs: false,
            },

            line_height: 0.0,
            line_spacing: 0.0,
            word_spacing: 0.0,
            letter_spacing: 0.0,
            max_width: 0.0,
            break_words: false,
            hyphen_glyphs: (ShapedSegmentData::default(), fonts[0].clone()),
            tab_x_advance: 0.0,
            tab_index: 0,
            hyphens: config.hyphens,
            allow_first_wrap: false,

            origin: euclid::point2(0.0, 0.0),
            first_line_max: f32::INFINITY,
            mid_clear_min: 0.0,
            max_line_x: 0.0,
            text_seg_end: 0,
            line_has_ltr: false,
            line_has_rtl: false,
        };

        let mut word_ctx_key = WordContextKey::new(&config.lang, config.direction, &config.font_features);

        let metrics = fonts[0].metrics();

        t.out.orig_line_height = config.line_height;
        t.out.orig_line_spacing = config.line_spacing;
        t.out.line_height = config.line_height;
        t.out.line_spacing = config.line_spacing;

        t.line_height = config.line_height.0 as f32;
        t.line_spacing = config.line_spacing.0 as f32;
        let baseline = metrics.ascent + metrics.line_gap / 2.0;

        t.out.baseline = t.out.line_height - baseline;
        t.out.underline = t.out.baseline + metrics.underline_position;
        t.out.underline_descent = t.out.baseline + metrics.descent + Px(1);
        t.out.strikethrough = t.out.baseline + metrics.ascent / 3.0;
        t.out.overline = t.out.baseline + metrics.ascent;

        let dft_line_height = metrics.line_height().0 as f32;
        let center_height = (t.line_height - dft_line_height) / 2.0;

        t.origin = euclid::point2::<_, ()>(0.0, baseline.0 as f32 + center_height);
        t.max_line_x = 0.0;
        if let Some(inline) = config.inline_constraints {
            t.first_line_max = inline.first_max.0 as f32;
            t.mid_clear_min = inline.mid_clear_min.0 as f32;
            t.allow_first_wrap = true;
        } else {
            t.first_line_max = f32::INFINITY;
            t.mid_clear_min = 0.0;
            t.allow_first_wrap = false;
        }

        t.letter_spacing = config.letter_spacing.0 as f32;
        t.word_spacing = config.word_spacing.0 as f32;
        t.tab_x_advance = config.tab_x_advance.0 as f32;
        t.tab_index = fonts[0].space_index();

        t.max_width = if config.max_width == Px::MAX {
            f32::INFINITY
        } else {
            config.max_width.0 as f32
        };

        t.break_words = match config.word_break {
            WordBreak::Normal => {
                lang!("ch").matches(&config.lang, true, false)
                    || lang!("jp").matches(&config.lang, true, false)
                    || lang!("ko").matches(&config.lang, true, false)
            }
            WordBreak::BreakAll => true,
            WordBreak::KeepAll => false,
        };

        if !matches!(config.hyphens, Hyphens::None) && t.max_width.is_finite() {
            // "hyphen" can be any char and we need the x-advance for the wrap algorithm.
            t.hyphen_glyphs = fonts.shape_segment(config.hyphen_char.as_str(), &word_ctx_key, &config.font_features, |s, f| {
                (s.clone(), f.clone())
            });
        }

        t.push_text(fonts, &config.font_features, &mut word_ctx_key, text);

        t.out.debug_assert_ranges();
        t.out
    }

    fn push_text(&mut self, fonts: &[Font], features: &RFontFeatures, word_ctx_key: &mut WordContextKey, text: &SegmentedText) {
        for (seg, info) in text.iter() {
            word_ctx_key.direction = info.direction();
            let max_width = self.actual_max_width();
            if info.kind.is_word() {
                fonts.shape_segment(seg, word_ctx_key, features, |shaped_seg, font| {
                    if self.origin.x + shaped_seg.x_advance > max_width {
                        // need wrap
                        if shaped_seg.x_advance > max_width {
                            // need segment split

                            // try to hyphenate
                            let hyphenated = self.push_hyphenate(word_ctx_key, seg, font, shaped_seg, info, text);

                            if !hyphenated && self.break_words {
                                // break word
                                self.push_split_seg(shaped_seg, seg, info, self.letter_spacing, text);
                            } else if !hyphenated {
                                // normal wrap, glyphs overflow
                                self.push_line_break(true, text);

                                // try to hyphenate with full width available
                                let hyphenaded = self.push_hyphenate(word_ctx_key, seg, font, shaped_seg, info, text);

                                if !hyphenaded {
                                    self.push_glyphs(shaped_seg, self.letter_spacing);
                                    self.push_text_seg(seg, info);
                                }
                            }
                        } else {
                            self.push_line_break(true, text);
                            self.push_glyphs(shaped_seg, self.letter_spacing);
                            self.push_text_seg(seg, info);
                        }
                    } else {
                        // don't need wrap
                        self.push_glyphs(shaped_seg, self.letter_spacing);
                        self.push_text_seg(seg, info);
                    }

                    if matches!(info.kind, TextSegmentKind::Emoji) && !font.face().color_glyphs().is_empty() {
                        self.out.has_colored_glyphs = true;
                    }

                    self.push_font(font);
                });
            } else if info.kind.is_space() {
                if matches!(info.kind, TextSegmentKind::Tab) {
                    if self.origin.x + self.tab_x_advance > max_width {
                        self.push_line_break(true, text);
                    }
                    let point = euclid::point2(self.origin.x, self.origin.y);
                    self.origin.x += self.tab_x_advance;
                    self.out.glyphs.push(GlyphInstance {
                        index: self.tab_index,
                        point,
                    });
                    self.out.clusters.push(0);

                    self.push_text_seg(seg, info);

                    self.push_font(&fonts[0]);
                } else {
                    fonts.shape_segment(seg, word_ctx_key, features, |shaped_seg, font| {
                        if self.origin.x + shaped_seg.x_advance > max_width {
                            // need wrap
                            if seg.len() > 2 {
                                // split spaces
                                self.push_split_seg(shaped_seg, seg, info, self.word_spacing, text);
                            } else {
                                self.push_line_break(true, text);
                                self.push_glyphs(shaped_seg, self.word_spacing);
                                self.push_text_seg(seg, info);
                            }
                        } else {
                            self.push_glyphs(shaped_seg, self.word_spacing);
                            self.push_text_seg(seg, info);
                        }

                        self.push_font(font);
                    });
                }
            } else if info.kind.is_line_break() {
                self.push_text_seg(seg, info);
                self.push_line_break(false, text);
            } else {
                self.push_text_seg(seg, info)
            }
        }

        self.finish_current_line_bidi(text);
        self.out.lines.0.push(LineRange {
            end: self.out.segments.0.len(),
            width: self.origin.x,
            x_offset: 0.0,
        });

        self.out.update_mid_size();
        self.out.update_first_last_lines();
        self.out.orig_first_line = self.out.first_line.size;
        self.out.orig_last_line = self.out.last_line.size;
        if self.out.is_inlined && self.out.lines.0.len() > 1 {
            self.out.last_line.origin.y += self.out.mid_clear;
        }

        self.push_font(&fonts[0]);
    }

    fn push_hyphenate(
        &mut self,
        word_ctx_key: &WordContextKey,
        seg: &str,
        font: &Font,
        shaped_seg: &ShapedSegmentData,
        info: TextSegment,
        text: &SegmentedText,
    ) -> bool {
        if !matches!(self.hyphens, Hyphens::Auto) {
            return false;
        }

        let split_points = Hyphenation::hyphenate(&word_ctx_key.lang(), seg);
        self.push_hyphenate_pt(&split_points, font, shaped_seg, seg, info, text)
    }

    fn push_hyphenate_pt(
        &mut self,
        split_points: &[usize],
        font: &Font,
        shaped_seg: &ShapedSegmentData,
        seg: &str,
        info: TextSegment,
        text: &SegmentedText,
    ) -> bool {
        if split_points.is_empty() {
            return false;
        }

        // find the split that fits more letters and hyphen
        let mut end_glyph = 0;
        let mut end_point_i = 0;
        let max_width = self.actual_max_width();
        for (i, point) in split_points.iter().enumerate() {
            let mut point = *point;
            let mut width = 0.0;
            let mut c = u32::MAX;
            let mut gi = 0;
            for (i, g) in shaped_seg.glyphs.iter().enumerate() {
                width = g.point.0;
                if g.cluster != c {
                    if point == 0 {
                        break;
                    }
                    c = g.cluster;
                    point -= 1;
                }
                gi = i;
            }

            if self.origin.x + width + self.hyphen_glyphs.0.x_advance > max_width {
                break;
            } else {
                end_glyph = gi;
                end_point_i = i;
            }
        }

        // split and push the the first half + hyphen
        let end_glyph_x = shaped_seg.glyphs[end_glyph].point.0;
        let (glyphs_a, glyphs_b) = shaped_seg.glyphs.split_at(end_glyph);
        if glyphs_a.is_empty() || glyphs_b.is_empty() {
            return false;
        }
        let end_cluster = glyphs_b[0].cluster;
        let (seg_a, seg_b) = seg.split_at(end_cluster as usize);

        self.push_glyphs(
            &ShapedSegmentData {
                glyphs: glyphs_a.to_vec(),
                x_advance: end_glyph_x,
                y_advance: glyphs_a.iter().map(|g| g.point.1).sum(),
            },
            self.word_spacing,
        );
        self.push_font(font);

        self.push_glyphs(&self.hyphen_glyphs.0.clone(), 0.0);
        self.push_font(&self.hyphen_glyphs.1.clone());

        self.push_text_seg(seg_a, info);

        self.push_line_break(true, text);

        // adjust the second half to a new line
        let mut shaped_seg_b = ShapedSegmentData {
            glyphs: glyphs_b.to_vec(),
            x_advance: shaped_seg.x_advance - end_glyph_x,
            y_advance: glyphs_b.iter().map(|g| g.point.1).sum(),
        };
        for g in &mut shaped_seg_b.glyphs {
            g.point.0 -= end_glyph_x;
            g.cluster -= seg_a.len() as u32;
        }

        if shaped_seg_b.x_advance > self.actual_max_width() {
            // second half still does not fit, try to hyphenate again.
            if self.push_hyphenate_pt(&split_points[end_point_i..], font, &shaped_seg_b, seg_b, info, text) {
                return true;
            }
        }

        // push second half
        self.push_glyphs(&shaped_seg_b, self.word_spacing);
        self.push_text_seg(seg_b, info);
        true
    }

    fn push_glyphs(&mut self, shaped_seg: &ShapedSegmentData, spacing: f32) {
        self.out.glyphs.extend(shaped_seg.glyphs.iter().map(|gi| {
            let r = GlyphInstance {
                index: gi.index,
                point: euclid::point2(gi.point.0 + self.origin.x, gi.point.1 + self.origin.y),
            };
            self.origin.x += spacing;
            r
        }));
        self.out.clusters.extend(shaped_seg.glyphs.iter().map(|gi| gi.cluster));

        self.origin.x += shaped_seg.x_advance;
        self.origin.y += shaped_seg.y_advance;
    }

    fn push_line_break(&mut self, soft: bool, text: &SegmentedText) {
        if self.out.glyphs.is_empty() && self.allow_first_wrap && soft {
            self.out.first_wrapped = true;
        } else {
            self.finish_current_line_bidi(text);

            self.out.lines.0.push(LineRange {
                end: self.out.segments.0.len(),
                width: self.origin.x,
                x_offset: 0.0,
            });

            if self.out.lines.0.len() == 1 {
                self.out.first_line = PxRect::from_size(PxSize::new(Px(self.origin.x as i32), Px(self.line_height as i32)));

                if !self.out.first_wrapped {
                    let mid_clear = (self.mid_clear_min - self.line_height).max(0.0).round();
                    self.origin.y += mid_clear;
                    self.out.mid_clear = Px(mid_clear as i32);
                    self.out.mid_offset = mid_clear;
                }
            }

            self.max_line_x = self.origin.x.max(self.max_line_x);
            self.origin.x = 0.0;
            self.origin.y += self.line_height + self.line_spacing;
        }
    }

    fn finish_current_line_bidi(&mut self, text: &SegmentedText) {
        if self.line_has_rtl {
            let seg_start = if self.out.lines.0.is_empty() {
                0
            } else {
                self.out.lines.last().end
            };

            if self.line_has_ltr {
                // mixed direction

                let line_segs = seg_start..self.out.segments.0.len();

                // compute visual order and offset segments.
                let mut x = 0.0;
                for i in text.reorder_line_to_ltr(line_segs) {
                    let g_range = self.out.segments.glyphs(i);
                    if g_range.iter().is_empty() {
                        continue;
                    }

                    let glyphs = &mut self.out.glyphs[g_range.iter()];
                    let offset = x - self.out.segments.0[i].x;
                    self.out.segments.0[i].x = x;
                    for g in glyphs {
                        g.point.x += offset;
                    }
                    x += self.out.segments.0[i].advance;
                }
            } else {
                // entire line RTL
                let line_width = self.origin.x;

                let mut x = line_width;
                for i in seg_start..self.out.segments.0.len() {
                    x -= self.out.segments.0[i].advance;

                    let g_range = self.out.segments.glyphs(i);

                    let glyphs = &mut self.out.glyphs[g_range.iter()];
                    let offset = x - self.out.segments.0[i].x;
                    self.out.segments.0[i].x = x;
                    for g in glyphs {
                        g.point.x += offset;
                    }
                }
            }
        }

        self.line_has_ltr = false;
        self.line_has_rtl = false;
    }

    pub fn push_text_seg(&mut self, seg: &str, info: TextSegment) {
        let g_len = if let Some(l) = self.out.segments.0.last() {
            self.out.glyphs.len() - l.end
        } else {
            self.out.glyphs.len()
        };
        if g_len > 0 {
            self.line_has_ltr |= info.level.is_ltr();
            self.line_has_rtl |= info.level.is_rtl();
        }

        self.text_seg_end += seg.len();

        let is_first_of_line =
            (!self.out.lines.0.is_empty() && self.out.lines.last().end == self.out.segments.0.len()) || self.out.segments.0.is_empty();
        let x = if is_first_of_line {
            0.0
        } else {
            // not first segment of line
            self.out.segments.0.last().map(|s| s.x + s.advance).unwrap_or(0.0)
        };
        self.out.segments.0.push(GlyphSegment {
            text: TextSegment {
                end: self.text_seg_end,
                ..info
            },
            end: self.out.glyphs.len(),
            x,
            advance: self.origin.x - x,
        });
    }

    pub fn push_split_seg(&mut self, shaped_seg: &ShapedSegmentData, seg: &str, info: TextSegment, spacing: f32, text: &SegmentedText) {
        let mut end_glyph = 0;
        let mut end_glyph_x = 0.0;
        let max_width = self.actual_max_width();
        for (i, g) in shaped_seg.glyphs.iter().enumerate() {
            if self.origin.x + g.point.0 > max_width {
                break;
            }
            end_glyph = i;
            end_glyph_x = g.point.0;
        }

        let (glyphs_a, glyphs_b) = shaped_seg.glyphs.split_at(end_glyph);

        if glyphs_a.is_empty() || glyphs_b.is_empty() {
            // failed split
            self.push_line_break(true, text);
            self.push_glyphs(shaped_seg, spacing);
            self.push_text_seg(seg, info);
        } else {
            let (seg_a, seg_b) = seg.split_at(glyphs_b[0].cluster as usize);

            let shaped_seg_a = ShapedSegmentData {
                glyphs: glyphs_a.to_vec(),
                x_advance: end_glyph_x,
                y_advance: glyphs_a.iter().map(|g| g.point.1).sum(),
            };
            self.push_glyphs(&shaped_seg_a, spacing);
            self.push_text_seg(seg_a, info);
            self.push_line_break(true, text);

            let mut shaped_seg_b = ShapedSegmentData {
                glyphs: glyphs_b.to_vec(),
                x_advance: shaped_seg.x_advance - end_glyph_x,
                y_advance: glyphs_b.iter().map(|g| g.point.1).sum(),
            };
            for g in &mut shaped_seg_b.glyphs {
                g.point.0 -= shaped_seg_a.x_advance;
                g.cluster -= seg_a.len() as u32;
            }

            if shaped_seg_b.x_advance <= max_width {
                self.push_glyphs(&shaped_seg_b, spacing);
                self.push_text_seg(seg_b, info);
            } else {
                self.push_split_seg(&shaped_seg_b, seg_b, info, spacing, text);
            }
        }
    }

    fn push_font(&mut self, font: &Font) {
        if let Some(last) = self.out.fonts.0.last_mut() {
            if &last.font == font {
                last.end = self.out.glyphs.len();
                return;
            } else if last.end == self.out.glyphs.len() {
                return;
            }
        }
        self.out.fonts.0.push(FontRange {
            font: font.clone(),
            end: self.out.glyphs.len(),
        })
    }
}

/// Represents a line selection of a [`ShapedText`].
#[derive(Clone, Copy)]
pub struct ShapedLine<'a> {
    text: &'a ShapedText,
    // range of segments of this line.
    seg_range: IndexRange,
    index: usize,
    width: Px,
}
impl<'a> fmt::Debug for ShapedLine<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedLine")
            .field("seg_range", &self.seg_range)
            .field("index", &self.index)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedLine<'a> {
    /// Bounds of the line.
    pub fn rect(&self) -> PxRect {
        if self.index == 0 {
            return self.text.first_line;
        }
        if self.index == self.text.lines.0.len() - 1 {
            return self.text.last_line;
        }

        let size = PxSize::new(self.width, self.text.line_height);
        let origin = PxPoint::new(
            Px(self.text.lines.0[self.index].x_offset as i32),
            self.text.line_height * Px((self.index - 1) as i32) + self.text.first_line.max_y() + self.text.mid_clear,
        );
        PxRect::new(origin, size)
    }

    /// Initial size of the line, before any line reshaping.
    ///
    /// This can be different then the current [`rect`] size if the parent inline changed the size, usually to inject
    /// blank spaces to justify the text or to visually insert a bidirectional fragment of another widget.
    ///
    /// [`rect`]: Self::rect
    pub fn original_size(&self) -> PxSize {
        if self.index == 0 {
            return self.text.orig_first_line;
        }
        if self.index == self.text.lines.0.len() - 1 {
            return self.text.orig_last_line;
        }
        PxSize::new(self.width, self.text.line_height)
    }

    /// Full overline, start point + width.
    pub fn overline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.overline)
    }

    /// Full strikethrough line, start point + width.
    pub fn strikethrough(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.strikethrough)
    }

    /// Full underline, not skipping.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns start point + width.
    pub fn underline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline)
    }

    /// Full underline, not skipping.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns start point + width.
    pub fn underline_descent(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline_descent)
    }

    /// Underline, skipping spaces.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns and iterator of start point + width for each word.
    pub fn underline_skip_spaces(&self) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.segs().filter(|s| s.kind().is_word()).map(|s| s.underline()))
    }

    /// Underline, skipping spaces.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns and iterator of start point + width for each word.
    pub fn underline_descent_skip_spaces(&self) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.segs().filter(|s| s.kind().is_word()).map(|s| s.underline_descent()))
    }

    /// Underline, skipping glyph descends that intersect the underline.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns an iterator of start point + width for continuous underline.
    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.segs().flat_map(move |s| s.underline_skip_glyphs(thickness)))
    }

    /// Underline, skipping spaces and glyph descends that intersect the underline
    ///
    /// The *y* is defined by font metrics.
    ///
    /// Returns an iterator of start point + width for continuous underline.
    pub fn underline_skip_glyphs_and_spaces(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(
            self.segs()
                .filter(|s| s.kind().is_word())
                .flat_map(move |s| s.underline_skip_glyphs(thickness)),
        )
    }

    fn decoration_line(&self, bottom_up_offset: Px) -> (PxPoint, Px) {
        let r = self.rect();
        let y = r.max_y() - bottom_up_offset;
        (PxPoint::new(r.origin.x, y), self.width)
    }

    fn segments(&self) -> &'a [GlyphSegment] {
        &self.text.segments.0[self.seg_range.iter()]
    }

    /// Glyphs in the line.
    pub fn glyphs(&self) -> impl Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_range(r)
    }

    /// Glyphs in the line paired with the *x-advance* to the next glyph or the end of the line.
    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a Font, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_with_x_advance_range(self.index, r)
    }

    fn glyphs_range(&self) -> IndexRange {
        self.text.segments.glyphs_range(self.seg_range)
    }

    /// Iterate over word and space segments in this line.
    pub fn segs(&self) -> impl Iterator<Item = ShapedSegment<'a>> + DoubleEndedIterator + ExactSizeIterator {
        let text = self.text;
        let line_index = self.index;
        self.seg_range.iter().map(move |i| ShapedSegment {
            text,
            line_index,
            index: i,
        })
    }

    /// Number of segments in this line.
    pub fn segs_len(&self) -> usize {
        self.seg_range.len()
    }

    /// Get the segment by index.
    ///
    /// The first segment of the line is `0`.
    pub fn seg(&self, seg_idx: usize) -> Option<ShapedSegment> {
        if self.seg_range.len() > seg_idx {
            Some(ShapedSegment {
                text: self.text,
                line_index: self.index,
                index: seg_idx + self.seg_range.start(),
            })
        } else {
            None
        }
    }

    /// Returns `true` if this line was started by the wrap algorithm.
    ///
    /// If this is `false` then the line is the first or the previous line ends in a [`LineBreak`].
    ///
    /// [`LineBreak`]: TextSegmentKind::LineBreak
    pub fn started_by_wrap(&self) -> bool {
        self.index > 0 && {
            let prev_line = self.text.lines.segs(self.index - 1);
            self.text.segments.0[prev_line.iter()]
                .last()
                .map(|s| !matches!(s.text.kind, TextSegmentKind::LineBreak))
                .unwrap_or(true)
        }
    }

    /// Returns `true` if this line was ended by the wrap algorithm.
    ///
    /// If this is `false` then the line is the last or ends in a [`LineBreak`].
    ///
    /// [`LineBreak`]: TextSegmentKind::LineBreak
    pub fn ended_by_wrap(&self) -> bool {
        // not last and not ended in line-break.
        self.index < self.text.lines.0.len() - 1
            && self
                .segments()
                .last()
                .map(|s| !matches!(s.text.kind, TextSegmentKind::LineBreak))
                .unwrap_or(false)
    }

    /// Get the text bytes range of this segment in the original text.
    pub fn text_range(&self) -> IndexRange {
        let start = self.seg_range.start();
        let start = if start == 0 { 0 } else { self.text.segments.0[start - 1].text.end };
        let end = self.text.segments.0[self.seg_range.end()].text.end;

        IndexRange(start, end)
    }

    /// Select the string represented by this line.
    ///
    /// The `full_text` must be equal to the original text that was used to generate the parent [`ShapedText`].
    pub fn text<'s>(&self, full_text: &'s str) -> &'s str {
        let IndexRange(start, end) = self.text_range();

        let start = start.min(full_text.len());
        let end = end.min(full_text.len());

        &full_text[start..end]
    }

    /// Gets the segment that contains `x` or is nearest to it;
    pub fn nearest_seg(&self, x: Px) -> Option<ShapedSegment<'a>> {
        let mut min = None;
        let mut min_dist = Px::MAX;
        for seg in self.segs() {
            let (seg_x, width) = seg.x_width();
            if x >= seg_x {
                let seg_max_x = seg_x + width;
                if x < seg_max_x {
                    return Some(seg);
                }
            }
            let dist = (x - seg_x).abs();
            if min_dist > dist {
                min = Some(seg);
                min_dist = dist;
            }
        }
        min
    }
}

/// Merges lines defined by `(PxPoint, Px)`, assuming the `y` is equal.
struct MergingLineIter<I> {
    iter: I,
    line: Option<(PxPoint, Px)>,
}
impl<I> MergingLineIter<I> {
    pub fn new(iter: I) -> Self {
        MergingLineIter { iter, line: None }
    }
}
impl<I: Iterator<Item = (PxPoint, Px)>> Iterator for MergingLineIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some((point, width)) => {
                    if let Some((lp, lw)) = &mut self.line {
                        // merge line if touching or only skipping 1px, the lines are rounded to snap-to-pixels
                        // this can cause 1px errors.
                        let diff = point.x - (lp.x + *lw);
                        if diff <= Px(1) {
                            *lw += width + diff;
                            continue;
                        } else {
                            let r = (*lp, *lw);

                            *lp = point;
                            *lw = width;

                            return Some(r);
                        }
                    } else {
                        self.line = Some((point, width));
                        continue;
                    }
                }
                None => return self.line.take(),
            }
        }
    }
}

/// Represents a word or space selection of a [`ShapedText`].
#[derive(Clone, Copy)]
pub struct ShapedSegment<'a> {
    text: &'a ShapedText,
    line_index: usize,
    index: usize,
}
impl<'a> fmt::Debug for ShapedSegment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedSegment")
            .field("line_index", &self.line_index)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedSegment<'a> {
    /// Segment kind.
    pub fn kind(&self) -> TextSegmentKind {
        self.text.segments.0[self.index].text.kind
    }

    /// Segment bidi level.
    pub fn level(&self) -> BidiLevel {
        self.text.segments.0[self.index].text.level
    }

    /// Layout direction of glyphs in the segment.
    pub fn direction(&self) -> LayoutDirection {
        self.text.segments.0[self.index].text.direction()
    }

    /// If the segment contains the last glyph of the line.
    pub fn has_last_glyph(&self) -> bool {
        let seg_glyphs = self.text.segments.glyphs(self.index);
        let s = self.text.lines.segs(self.line_index);
        let line_glyphs = self.text.segments.glyphs_range(s);
        seg_glyphs.end() == line_glyphs.end()
    }

    fn glyphs_range(&self) -> IndexRange {
        self.text.segments.glyphs(self.index)
    }

    /// Glyphs in the word or space.
    pub fn glyphs(&self) -> impl Iterator<Item = (&'a Font, &'a [GlyphInstance])> {
        let r = self.glyphs_range();
        self.text.glyphs_range(r)
    }

    fn clusters(&self) -> &[u32] {
        let r = self.glyphs_range();
        self.text.clusters_range(r)
    }

    /// Glyphs in the word or space, paired with the *x-advance* to then next glyph or line end.
    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a Font, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_with_x_advance_range(self.line_index, r)
    }

    fn x_width(&self) -> (Px, Px) {
        let IndexRange(start, end) = self.glyphs_range();

        let is_line_break = start == end && matches!(self.kind(), TextSegmentKind::LineBreak);

        let start_x = match self.direction() {
            LayoutDirection::LTR => {
                if is_line_break {
                    let x = self.text.lines.x_offset(self.line_index);
                    let w = self.text.lines.width(self.line_index);
                    return (Px((x + w) as i32), Px(0));
                }
                self.text.glyphs[start].point.x
            }
            LayoutDirection::RTL => {
                if is_line_break {
                    return (Px(0), Px(0));
                }

                self.text.glyphs[start..end]
                    .iter()
                    .map(|g| g.point.x)
                    .min_by(f32::total_cmp)
                    .unwrap_or(0.0)
            }
        };

        (Px(start_x.floor() as i32), Px(self.advance().ceil() as i32))
    }

    /// Segment exact *width* in pixels.
    pub fn advance(&self) -> f32 {
        self.text.segments.0[self.index].advance
    }

    /// Bounds of the word or spaces.
    pub fn rect(&self) -> PxRect {
        let (x, width) = self.x_width();
        let size = PxSize::new(width, self.text.line_height);

        let y = if self.line_index == 0 {
            self.text.first_line.origin.y
        } else if self.line_index == self.text.lines.0.len() - 1 {
            self.text.last_line.origin.y
        } else {
            self.text.line_height * Px((self.line_index - 1) as i32) + self.text.first_line.max_y() + self.text.mid_clear
        };
        PxRect::new(PxPoint::new(x, y), size)
    }

    /// Segment info for widget inline segments.
    pub fn inline_info(&self) -> InlineSegmentInfo {
        let (x, width) = self.x_width();
        InlineSegmentInfo { x, width }
    }

    fn decoration_line(&self, bottom_up_offset: Px) -> (PxPoint, Px) {
        let (x, width) = self.x_width();
        let y = (self.text.line_height * Px((self.line_index as i32) + 1)) - bottom_up_offset;
        (PxPoint::new(x, y), width)
    }

    /// Overline spanning the word or spaces, start point + width.
    pub fn overline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.overline)
    }

    /// Strikethrough spanning the word or spaces, start point + width.
    pub fn strikethrough(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.strikethrough)
    }

    /// Underline spanning the word or spaces, not skipping.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns start point + width.
    pub fn underline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline)
    }

    /// Underline spanning the word or spaces, skipping glyph descends that intercept the line.
    ///
    /// Returns an iterator of start point + width for underline segments.
    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        let y = (self.text.line_height * Px((self.line_index as i32) + 1)) - self.text.underline;
        let (x, _) = self.x_width();

        let line_y = -(self.text.baseline - self.text.underline).0 as f32;
        let line_y_range = (line_y, line_y - thickness.0 as f32);

        // space around glyph descends, thickness clamped to a minimum of 1px and a maximum of 0.2em (same as Firefox).
        let padding = (thickness.0 as f32).clamp(1.0, (self.text.fonts.font(0).size().0 as f32 * 0.2).max(1.0));

        // no yield, only sadness
        struct UnderlineSkipGlyphs<'a, I, J> {
            line_y_range: (f32, f32),
            y: Px,
            padding: f32,
            min_width: Px,

            iter: I,
            resume: Option<(&'a Font, J)>,
            x: f32,
            width: f32,
        }
        impl<'a, I, J> UnderlineSkipGlyphs<'a, I, J> {
            fn line(&self) -> Option<(PxPoint, Px)> {
                fn f32_to_px(px: f32) -> Px {
                    Px(px.round() as i32)
                }
                let r = (PxPoint::new(f32_to_px(self.x), self.y), f32_to_px(self.width));
                if r.1 >= self.min_width {
                    Some(r)
                } else {
                    None
                }
            }
        }
        impl<'a, I, J> Iterator for UnderlineSkipGlyphs<'a, I, J>
        where
            I: Iterator<Item = (&'a Font, J)>,
            J: Iterator<Item = (GlyphInstance, f32)>,
        {
            type Item = (PxPoint, Px);

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    let continuation = self.resume.take().or_else(|| self.iter.next());
                    if let Some((font, mut glyphs_with_adv)) = continuation {
                        for (g, a) in &mut glyphs_with_adv {
                            if let Ok(Some((ex_start, ex_end))) = font.h_line_hits(g.index, self.line_y_range) {
                                self.width += ex_start - self.padding;
                                let r = self.line();
                                self.x += self.width + self.padding + ex_end + self.padding;
                                self.width = a - (ex_start + ex_end) - self.padding;

                                if r.is_some() {
                                    self.resume = Some((font, glyphs_with_adv));
                                    return r;
                                }
                            } else {
                                self.width += a;
                                // continue
                            }
                        }
                    } else {
                        let r = self.line();
                        self.width = 0.0;
                        return r;
                    }
                }
            }
        }
        UnderlineSkipGlyphs {
            line_y_range,
            y,
            padding,
            min_width: Px((padding / 2.0).max(1.0).ceil() as i32),

            iter: self.glyphs_with_x_advance(),
            resume: None,
            x: x.0 as f32,
            width: 0.0,
        }
    }

    /// Underline spanning the word or spaces, not skipping.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns start point + width.
    pub fn underline_descent(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline_descent)
    }

    /// Get the text bytes range of this segment in the original text.
    pub fn text_range(&self) -> IndexRange {
        let start = if self.index == 0 {
            0
        } else {
            self.text.segments.0[self.index - 1].text.end
        };
        let end = self.text.segments.0[self.index].text.end;

        IndexRange(start, end)
    }

    /// Get the text bytes range of the `glyph_range` in this segment's [`text`].
    ///
    /// [`text`]: Self::text
    pub fn text_glyph_range(&self, glyph_range: impl ops::RangeBounds<usize>) -> IndexRange {
        let included_start = match glyph_range.start_bound() {
            ops::Bound::Included(i) => Some(*i),
            ops::Bound::Excluded(i) => Some(*i + 1),
            ops::Bound::Unbounded => None,
        };
        let excluded_end = match glyph_range.end_bound() {
            ops::Bound::Included(i) => Some(*i - 1),
            ops::Bound::Excluded(i) => Some(*i),
            ops::Bound::Unbounded => None,
        };

        let glyph_range_start = self.glyphs_range().start();
        let glyph_to_char = |g| self.text.clusters[glyph_range_start + g] as usize;

        match (included_start, excluded_end) {
            (None, None) => IndexRange(0, self.text_range().len()),
            (None, Some(end)) => IndexRange(0, glyph_to_char(end)),
            (Some(start), None) => IndexRange(glyph_to_char(start), self.text_range().len()),
            (Some(start), Some(end)) => IndexRange(glyph_to_char(start), glyph_to_char(end)),
        }
    }

    /// Select the string represented by this segment.
    ///
    /// The `full_text` must be equal to the original text that was used to generate the parent [`ShapedText`].
    pub fn text<'s>(&self, full_text: &'s str) -> &'s str {
        let IndexRange(start, end) = self.text_range();
        let start = start.min(full_text.len());
        let end = end.min(full_text.len());
        &full_text[start..end]
    }

    /// Gets the insert index in the segment text that is nearest to `x`.
    pub fn nearest_char_index(&self, x: Px, full_text: &str) -> usize {
        let x = x.0 as f32;
        let q = self
            .glyphs_with_x_advance()
            .flat_map(|(_, g)| g)
            .enumerate()
            .min_by_key(|(_, (g, _))| {
                let key = (g.point.x - x).abs();
                (key * 5.0) as i32
            });
        let (i, (g, advance)) = match q {
            Some(r) => r,
            None => return self.text_range().start(), // no glyphs
        };

        let i = self.glyphs_range().start() + i;
        let mut i = self.text_range().start() + self.text.clusters[i] as usize;

        if x >= g.point.x + advance / 2.0 {
            i += full_text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        }

        i
    }
}

const WORD_CACHE_MAX_LEN: usize = 32;
const WORD_CACHE_MAX_ENTRIES: usize = 10_000;

#[derive(Hash, PartialEq, Eq)]
pub(super) struct WordCacheKey<S> {
    string: S,
    ctx_key: WordContextKey,
}
#[derive(Hash)]
struct WordCacheKeyRef<'a, S> {
    string: &'a S,
    ctx_key: &'a WordContextKey,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub(super) struct WordContextKey {
    lang: unic_langid::subtags::Language,
    script: Option<unic_langid::subtags::Script>,
    direction: LayoutDirection,
    features: Box<[usize]>,
}
impl WordContextKey {
    pub fn new(lang: &Lang, direction: LayoutDirection, font_features: &RFontFeatures) -> Self {
        let is_64 = mem::size_of::<usize>() == mem::size_of::<u64>();

        let mut features = vec![];

        if !font_features.is_empty() {
            features.reserve(font_features.len() * if is_64 { 3 } else { 4 });
            for feature in font_features {
                if is_64 {
                    let mut h = feature.tag().0 as usize;
                    h |= (feature.value() as usize) << 32;
                    features.push(h);
                } else {
                    features.push(feature.tag().0 as usize);
                    features.push(feature.value() as usize);
                }

                features.push(feature.start());
                features.push(feature.end());
            }
        }

        WordContextKey {
            lang: lang.language,
            script: lang.script,
            direction,
            features: features.into_boxed_slice(),
        }
    }

    pub fn harfbuzz_lang(&self) -> Option<harfbuzz_rs::Language> {
        self.lang.as_str().parse().ok()
    }

    pub fn harfbuzz_script(&self) -> Option<harfbuzz_rs::Tag> {
        let t: u32 = self.script?.into();
        let t = t.to_le_bytes(); // Script is a TinyStr4 that uses LE
        Some(harfbuzz_rs::Tag::from(&[t[0], t[1], t[2], t[3]]))
    }

    pub fn harfbuzz_direction(&self) -> harfbuzz_rs::Direction {
        self.direction.into()
    }

    pub fn lang(&self) -> Lang {
        Lang::from_parts(self.lang, self.script, None, &[])
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct ShapedSegmentData {
    glyphs: Vec<ShapedGlyph>,
    x_advance: f32,
    y_advance: f32,
}
#[derive(Debug, Clone, Copy)]
struct ShapedGlyph {
    /// glyph index
    index: u32,
    /// char index
    cluster: u32,
    point: (f32, f32),
}

impl Font {
    fn buffer_segment(&self, segment: &str, key: &WordContextKey) -> harfbuzz_rs::UnicodeBuffer {
        let mut buffer = harfbuzz_rs::UnicodeBuffer::new()
            .set_direction(key.harfbuzz_direction())
            .set_cluster_level(harfbuzz_rs::ClusterLevel::MonotoneCharacters);

        if let Some(lang) = key.harfbuzz_lang() {
            buffer = buffer.set_language(lang);
        }
        if let Some(script) = key.harfbuzz_script() {
            buffer = buffer.set_script(script);
        }

        buffer.add_str(segment)
    }

    fn shape_segment_no_cache(&self, seg: &str, key: &WordContextKey, features: &[harfbuzz_rs::Feature]) -> ShapedSegmentData {
        let size_scale = self.metrics().size_scale;
        let to_layout = |p: i32| p as f32 * size_scale;

        let buffer = self.buffer_segment(seg, key);
        let buffer = harfbuzz_rs::shape(self.harfbuzz_font(), buffer, features);

        let mut w_x_advance = 0.0;
        let mut w_y_advance = 0.0;

        let glyphs: Vec<_> = if self.face().is_empty() {
            let advance = self.metrics().bounds.width().0 as f32;
            buffer
                .get_glyph_infos()
                .iter()
                .map(|i| {
                    let g = ShapedGlyph {
                        index: 0,
                        cluster: i.cluster,
                        point: (w_x_advance, 0.0),
                    };
                    w_x_advance += advance;
                    g
                })
                .collect()
        } else {
            buffer
                .get_glyph_infos()
                .iter()
                .zip(buffer.get_glyph_positions())
                .map(|(i, p)| {
                    let x_offset = to_layout(p.x_offset);
                    let y_offset = -to_layout(p.y_offset);
                    let x_advance = to_layout(p.x_advance);
                    let y_advance = to_layout(p.y_advance);

                    let point = (w_x_advance + x_offset, w_y_advance + y_offset);
                    w_x_advance += x_advance;
                    w_y_advance += y_advance;

                    ShapedGlyph {
                        index: i.codepoint,
                        cluster: i.cluster,
                        point,
                    }
                })
                .collect()
        };

        ShapedSegmentData {
            glyphs,
            x_advance: w_x_advance,
            y_advance: w_y_advance,
        }
    }

    fn shape_segment<R>(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        features: &[harfbuzz_rs::Feature],
        out: impl FnOnce(&ShapedSegmentData) -> R,
    ) -> R {
        if !(1..=WORD_CACHE_MAX_LEN).contains(&seg.len()) || self.face().is_empty() {
            let seg = self.shape_segment_no_cache(seg, word_ctx_key, features);
            out(&seg)
        } else if let Some(small) = Self::to_small_word(seg) {
            // try cached
            let cache = self.0.small_word_cache.read();
            let mut hasher = cache.hasher().build_hasher();
            WordCacheKeyRef {
                string: &small,
                ctx_key: word_ctx_key,
            }
            .hash(&mut hasher);
            let hash = hasher.finish();

            if let Some((_, seg)) = cache
                .raw_entry()
                .from_hash(hash, |e| e.string == small && &e.ctx_key == word_ctx_key)
            {
                return out(seg);
            }
            drop(cache);

            // shape and cache, can end-up shaping the same word here, but that is better then write locking
            let seg = self.shape_segment_no_cache(seg, word_ctx_key, features);
            let key = WordCacheKey {
                string: small,
                ctx_key: word_ctx_key.clone(),
            };
            let r = out(&seg);
            let mut cache = self.0.small_word_cache.write();
            if cache.len() > WORD_CACHE_MAX_ENTRIES {
                cache.clear();
            }
            cache.insert(key, seg);
            r
        } else {
            // try cached
            let cache = self.0.word_cache.read();
            let mut hasher = cache.hasher().build_hasher();
            WordCacheKeyRef {
                string: &seg,
                ctx_key: word_ctx_key,
            }
            .hash(&mut hasher);
            let hash = hasher.finish();

            if let Some((_, seg)) = cache
                .raw_entry()
                .from_hash(hash, |e| e.string.as_str() == seg && &e.ctx_key == word_ctx_key)
            {
                return out(seg);
            }
            drop(cache);

            // shape and cache, can end-up shaping the same word here, but that is better then write locking
            let string = seg.to_owned();
            let seg = self.shape_segment_no_cache(seg, word_ctx_key, features);
            let key = WordCacheKey {
                string,
                ctx_key: word_ctx_key.clone(),
            };
            let r = out(&seg);
            let mut cache = self.0.word_cache.write();
            if cache.len() > WORD_CACHE_MAX_ENTRIES {
                cache.clear();
            }
            cache.insert(key, seg);
            r
        }
    }

    /// Glyph index for the space `' '` character.
    pub fn space_index(&self) -> GlyphIndex {
        self.0.font.get_nominal_glyph(' ').unwrap_or(0)
    }

    /// Returns the horizontal advance of the space `' '` character.
    pub fn space_x_advance(&self) -> Px {
        let mut adv = 0.0;
        self.shape_segment(
            " ",
            &WordContextKey {
                lang: unic_langid::subtags::Language::from_bytes(b"und").unwrap(),
                script: None,
                direction: LayoutDirection::LTR,
                features: Box::new([]),
            },
            &[],
            |r| adv = r.x_advance,
        );

        Px(adv as i32)
    }

    /// Gets the distance from the origin of the glyph with the given ID to the next.
    pub fn advance(&self, index: GlyphIndex) -> Result<LayoutVector2D, GlyphLoadingError> {
        self.face()
            .font_kit()
            .ok_or(GlyphLoadingError::NoSuchGlyph)?
            .advance(index)
            .map(|v| LayoutVector2D::new(v.x(), v.y()) * self.metrics().size_scale)
    }

    /// Gets the amount that the given glyph should be displaced from the origin.
    pub fn origin(&self, index: GlyphIndex) -> Result<LayoutVector2D, GlyphLoadingError> {
        self.face()
            .font_kit()
            .ok_or(GlyphLoadingError::NoSuchGlyph)?
            .origin(index)
            .map(|v| LayoutVector2D::new(v.x(), v.y()) * self.metrics().size_scale)
    }

    /// Calculates a [`ShapedText`].
    pub fn shape_text(self: &Font, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        ShapedTextBuilder::shape_text(&[self.clone()], text, config)
    }

    /// Sends the sized vector path for a glyph to `sink`.
    pub fn outline(
        &self,
        glyph_id: GlyphIndex,
        hinting_options: OutlineHintingOptions,
        sink: &mut impl OutlineSink,
    ) -> Result<(), GlyphLoadingError> {
        struct AdapterSink<'a, S> {
            sink: &'a mut S,
            scale: f32,
        }
        impl<'a, S> AdapterSink<'a, S> {
            fn scale(&self, p: pathfinder_geometry::vector::Vector2F) -> euclid::Point2D<f32, Px> {
                euclid::point2(p.x() * self.scale, p.y() * self.scale)
            }
        }
        impl<'a, S: OutlineSink> font_kit::outline::OutlineSink for AdapterSink<'a, S> {
            fn move_to(&mut self, to: pathfinder_geometry::vector::Vector2F) {
                let to = self.scale(to);
                self.sink.move_to(to)
            }

            fn line_to(&mut self, to: pathfinder_geometry::vector::Vector2F) {
                let to = self.scale(to);
                self.sink.line_to(to)
            }

            fn quadratic_curve_to(&mut self, ctrl: pathfinder_geometry::vector::Vector2F, to: pathfinder_geometry::vector::Vector2F) {
                let ctrl = self.scale(ctrl);
                let to = self.scale(to);
                self.sink.quadratic_curve_to(ctrl, to)
            }

            fn cubic_curve_to(
                &mut self,
                ctrl: pathfinder_geometry::line_segment::LineSegment2F,
                to: pathfinder_geometry::vector::Vector2F,
            ) {
                let l_from = self.scale(ctrl.from());
                let l_to = self.scale(ctrl.to());
                let to = self.scale(to);
                self.sink.cubic_curve_to((l_from, l_to), to)
            }

            fn close(&mut self) {
                self.sink.close()
            }
        }

        let scale = self.metrics().size_scale;

        self.face()
            .font_kit()
            .ok_or(GlyphLoadingError::NoSuchGlyph)?
            .outline(glyph_id, hinting_options, &mut AdapterSink { sink, scale })
    }

    /// Returns the boundaries of a glyph in pixel units.
    ///
    /// The rectangle origin is the bottom-left of the bounds relative to the baseline.
    pub fn typographic_bounds(&self, glyph_id: GlyphIndex) -> Result<euclid::Rect<f32, Px>, GlyphLoadingError> {
        let rect = self
            .face()
            .font_kit()
            .ok_or(GlyphLoadingError::NoSuchGlyph)?
            .typographic_bounds(glyph_id)?;

        let scale = self.metrics().size_scale;
        let bounds = euclid::rect::<f32, Px>(
            rect.origin_x() * scale,
            rect.origin_y() * scale,
            rect.width() * scale,
            rect.height() * scale,
        );

        Ok(bounds)
    }

    /// Ray cast an horizontal line across the glyph and returns the entry and exit hits.
    ///
    /// The `line_y_range` are two vertical offsets relative to the baseline, the offsets define
    /// the start and inclusive end of the horizontal line, that is, `(underline, underline + thickness)`, note
    /// that positions under the baseline are negative so a 2px underline set 1px under the baseline becomes `(-1.0, -3.0)`.
    ///
    /// Returns `Ok(Some(x_enter, x_exit))` where the two values are x-advances, returns `None` if there is not hit, returns
    /// an error if the glyph is not found. The first x-advance is from the left typographic border to the first hit on the outline,
    /// the second x-advance is from the first across the outline to the exit hit.
    pub fn h_line_hits(&self, glyph_id: GlyphIndex, line_y_range: (f32, f32)) -> Result<Option<(f32, f32)>, GlyphLoadingError> {
        // Algorithm:
        //
        //  - Ignore curves, everything is direct line.
        //  - If a line-y crosses `line_y_range` register the min-x and max-x from the two points.
        //  - Same if a line is inside `line_y_range`.
        struct InterseptsSink {
            start: Option<euclid::Point2D<f32, Px>>,
            curr: euclid::Point2D<f32, Px>,
            under: (bool, bool),

            line_y_range: (f32, f32),
            hit: Option<(f32, f32)>,
        }
        impl OutlineSink for InterseptsSink {
            fn move_to(&mut self, to: euclid::Point2D<f32, Px>) {
                self.start = Some(to);
                self.curr = to;
                self.under = (to.y < self.line_y_range.0, to.y < self.line_y_range.1);
            }

            fn line_to(&mut self, to: euclid::Point2D<f32, Px>) {
                let under = (to.y < self.line_y_range.0, to.y < self.line_y_range.1);

                if self.under != under || under == (true, false) {
                    // crossed one or two y-range boundaries or both points are inside
                    self.under = under;

                    let (x0, x1) = if self.curr.x < to.x {
                        (self.curr.x, to.x)
                    } else {
                        (to.x, self.curr.x)
                    };
                    if let Some((min, max)) = &mut self.hit {
                        *min = min.min(x0);
                        *max = max.max(x1);
                    } else {
                        self.hit = Some((x0, x1));
                    }
                }

                self.curr = to;
                self.under = under;
            }

            fn quadratic_curve_to(&mut self, _: euclid::Point2D<f32, Px>, to: euclid::Point2D<f32, Px>) {
                self.line_to(to);
            }

            fn cubic_curve_to(&mut self, _: (euclid::Point2D<f32, Px>, euclid::Point2D<f32, Px>), to: euclid::Point2D<f32, Px>) {
                self.line_to(to);
            }

            fn close(&mut self) {
                if let Some(s) = self.start.take() {
                    if s != self.curr {
                        self.line_to(s);
                    }
                }
            }
        }
        let mut sink = InterseptsSink {
            start: None,
            curr: euclid::point2(0.0, 0.0),
            under: (false, false),

            line_y_range,
            hit: None,
        };
        self.outline(glyph_id, OutlineHintingOptions::None, &mut sink)?;

        Ok(sink.hit.map(|(a, b)| (a, b - a)))
    }
}

/// Hinting options for [`Font::outline`].
pub type OutlineHintingOptions = font_kit::hinting::HintingOptions;

/// Receives Bézier path rendering commands from [`Font::outline`].
///
/// The points are relative to the baseline, negative values under, positive over.
pub trait OutlineSink {
    /// Moves the pen to a point.
    fn move_to(&mut self, to: euclid::Point2D<f32, Px>);
    /// Draws a line to a point.
    fn line_to(&mut self, to: euclid::Point2D<f32, Px>);
    /// Draws a quadratic Bézier curve to a point.
    fn quadratic_curve_to(&mut self, ctrl: euclid::Point2D<f32, Px>, to: euclid::Point2D<f32, Px>);
    /// Draws a cubic Bézier curve to a point.
    ///
    /// The `ctrl` is a line (from, to).
    fn cubic_curve_to(&mut self, ctrl: (euclid::Point2D<f32, Px>, euclid::Point2D<f32, Px>), to: euclid::Point2D<f32, Px>);
    /// Closes the path, returning to the first point in it.
    fn close(&mut self);
}

impl FontList {
    /// Calculates a [`ShapedText`] using the [best](FontList::best) font in this list and the other fonts as fallback.
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        ShapedTextBuilder::shape_text(self, text, config)
    }
}

#[cfg(test)]
mod tests {
    use crate::{app::App, context::LayoutDirection, text::*};

    fn test_font() -> Font {
        let mut app = App::default().run_headless(false);
        let font = app
            .block_on_fut(
                async {
                    FONTS
                        .normal(&FontName::sans_serif(), &lang!(und))
                        .wait_rsp()
                        .await
                        .unwrap()
                        .sized(Px(20), vec![])
                },
                10.secs(),
            )
            .unwrap();
        drop(app);
        font
    }

    #[test]
    fn set_line_spacing() {
        let text = "0\n1\n2\n3\n4";
        test_line_spacing(text, Px(20), Px(0));
        test_line_spacing(text, Px(0), Px(20));
        test_line_spacing(text, Px(4), Px(6));
        test_line_spacing(text, Px(4), Px(4));
        test_line_spacing("a line\nanother\nand another", Px(20), Px(0));
        test_line_spacing("", Px(20), Px(0));
        test_line_spacing("a line", Px(20), Px(0));
    }
    fn test_line_spacing(text: &'static str, from: Px, to: Px) {
        let font = test_font();
        let mut config = TextShapingArgs {
            line_height: Px(40),
            line_spacing: from,
            ..Default::default()
        };

        let text = SegmentedText::new(text, LayoutDirection::LTR);
        let mut test = font.shape_text(&text, &config);

        config.line_spacing = to;
        let expected = font.shape_text(&text, &config);

        assert_eq!(from, test.line_spacing());
        test.reshape_lines(
            PxConstraints2d::new_fill_size(test.align_size()),
            None,
            test.align(),
            test.line_height(),
            to,
            test.direction(),
        );
        assert_eq!(to, test.line_spacing());

        for (i, (g0, g1)) in test.glyphs.iter().zip(expected.glyphs.iter()).enumerate() {
            assert_eq!(g0, g1, "testing {from} to {to}, glyph {i} is not equal");
        }

        assert_eq!(test.size(), expected.size());
    }

    #[test]
    fn set_line_height() {
        let text = "0\n1\n2\n3\n4";
        test_line_height(text, Px(20), Px(20));
        test_line_height(text, Px(20), Px(10));
        test_line_height(text, Px(10), Px(20));
        test_line_height("a line\nanother\nand another", Px(20), Px(10));
        test_line_height("", Px(20), Px(10));
        test_line_height("a line", Px(20), Px(10));
    }
    fn test_line_height(text: &'static str, from: Px, to: Px) {
        let font = test_font();
        let mut config = TextShapingArgs {
            line_height: from,
            line_spacing: Px(20),
            ..Default::default()
        };

        let text = SegmentedText::new(text, LayoutDirection::LTR);
        let mut test = font.shape_text(&text, &config);

        config.line_height = to;
        let expected = font.shape_text(&text, &config);

        assert_eq!(from, test.line_height());
        test.reshape_lines(
            PxConstraints2d::new_fill_size(test.align_size()),
            None,
            test.align(),
            to,
            test.line_spacing(),
            test.direction(),
        );
        assert_eq!(to, test.line_height());

        for (i, (g0, g1)) in test.glyphs.iter().zip(expected.glyphs.iter()).enumerate() {
            assert_eq!(g0, g1, "testing {from} to {to}, glyph {i} is not equal");
        }

        assert_eq!(test.size(), expected.size());
    }

    #[test]
    fn font_fallback_issue() {
        let mut app = App::default().run_headless(false);
        app.block_on_fut(
            async {
                let font = FONTS
                    .list(
                        &[FontName::new("Consolas"), FontName::monospace()],
                        FontStyle::Normal,
                        FontWeight::NORMAL,
                        FontStretch::NORMAL,
                        &lang!(und),
                    )
                    .wait_rsp()
                    .await
                    .sized(Px(20), vec![]);

                let config = TextShapingArgs::default();

                let txt_seg = SegmentedText::new("النص ثنائي الاتجاه (بالإنجليزية:Bi", LayoutDirection::RTL);
                let txt_shape = font.shape_text(&txt_seg, &config);

                let _ok = (txt_seg, txt_shape);
            },
            5.secs(),
        )
        .unwrap()
    }

    #[test]
    fn cluster_is_byte() {
        let font = test_font();

        let data = font.shape_segment_no_cache("£a", &WordContextKey::new(&lang!("en-US"), LayoutDirection::LTR, &vec![]), &[]);

        for ((i, _), g) in "£a".char_indices().zip(&data.glyphs) {
            assert_eq!(i as u32, g.cluster);
        }
    }
}
