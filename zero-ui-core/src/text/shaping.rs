use std::{
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    mem,
};

use super::{
    font_features::RFontFeatures, lang, Font, FontList, FontRef, GlyphIndex, GlyphInstance, InternedStr, Lang, SegmentedText, TextSegment,
    TextSegmentKind,
};
use crate::{crate_util::IndexRange, units::*};

pub use font_kit::error::GlyphLoadingError;

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

    /// Language of the text, also identifies if RTL.
    pub lang: Lang,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

    /// Width of the TAB character.
    pub tab_x_advance: Px,

    /// Extra space before the start of the first line.
    pub text_indent: Px,

    /// Finalized font features.
    pub font_features: RFontFeatures,
}
impl Default for TextShapingArgs {
    fn default() -> Self {
        TextShapingArgs {
            letter_spacing: Px(0),
            word_spacing: Px(0),
            line_height: Px(0),
            line_spacing: Px(0),
            lang: lang!(und),
            ignore_ligatures: false,
            disable_kerning: false,
            tab_x_advance: Px(0),
            text_indent: Px(0),
            font_features: RFontFeatures::default(),
        }
    }
}

/// Output of [text layout].
///
/// [text layout]: Font::shape_text
#[derive(Clone, Default)]
pub struct ShapedText {
    glyphs: Vec<GlyphInstance>,
    /// segments of `glyphs`
    segments: Vec<TextSegment>,
    /// index of `LineBreak` segments , line x-advance and width, is `segments.len()` for the last line.
    lines: Vec<(usize, Px, Px)>,
    /// fonts and index after last glyph that uses the font.
    fonts: Vec<(FontRef, usize)>,

    padding: PxSideOffsets,
    size: PxSize,
    line_height: Px,
    line_spacing: Px,

    // offsets from the line_height bottom
    baseline: Px,
    overline: Px,
    strikethrough: Px,
    underline: Px,
    underline_descent: Px,
}
impl fmt::Debug for ShapedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugFonts<'a>(&'a Vec<(FontRef, usize)>);
        impl<'a> fmt::Debug for DebugFonts<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list()
                    .entries(self.0.iter().map(|(f, i)| (f.face().display_name().name(), i)))
                    .finish()
            }
        }

        f.debug_struct("ShapedText")
            .field("segments", &self.segments)
            .field("glyphs", &self.glyphs)
            .field("lines", &self.lines)
            .field("fonts", &DebugFonts(&self.fonts))
            .field("padding", &self.padding)
            .field("size", &self.size)
            .field("line_height", &self.line_height)
            .field("line_spacing", &self.line_spacing)
            .field("baseline", &self.baseline)
            .field("overline", &self.overline)
            .field("strikethrough", &self.strikethrough)
            .field("underline", &self.underline)
            .field("underline_descent", &self.underline_descent)
            .finish()
    }
}
impl ShapedText {
    /// Split the shaped text in two. All segments before `segment` are the first text, the `segment` and
    /// all other segments are the second text.
    ///
    /// Any padding set is removed before split.
    ///
    /// # Panics
    ///
    /// Panics if `segment` is out of the range.
    pub fn split(mut self, segment: usize) -> (ShapedText, ShapedText) {
        self.set_padding(self.padding * -Px(1));

        if segment == 0 {
            let a = ShapedText {
                glyphs: vec![],
                segments: vec![],
                lines: vec![(0, Px(0), Px(0))],
                fonts: vec![self.fonts[0].clone()],
                padding: PxSideOffsets::zero(),
                size: PxSize::zero(),
                line_height: self.line_height,
                line_spacing: self.line_spacing,
                baseline: self.baseline,
                overline: self.overline,
                strikethrough: self.strikethrough,
                underline: self.underline,
                underline_descent: self.underline_descent,
            };
            (a, self)
        } else {
            let g_end = self.segments[segment - 1].end;
            let l_end = self.lines.iter().position(|(s, _, _)| *s >= segment).unwrap();
            let f_end = self.fonts.iter().position(|(_, g)| *g >= g_end).unwrap();

            let mut b = ShapedText {
                glyphs: self.glyphs.drain(g_end..).collect(),
                segments: self.segments.drain(segment..).collect(),
                lines: self.lines.drain(l_end..).collect(),
                fonts: self.fonts.drain(f_end..).collect(),
                padding: PxSideOffsets::zero(),
                size: PxSize::zero(),
                line_height: self.line_height,
                line_spacing: self.line_spacing,
                baseline: self.baseline,
                overline: self.overline,
                strikethrough: self.strikethrough,
                underline: self.underline,
                underline_descent: self.underline_descent,
            };

            if b.segments[0].kind != TextSegmentKind::LineBreak {
                let mut a_last_line = b.lines[0];
                let l_width = a_last_line.2;
                let x_offset = b.glyphs[0].point.x;
                let a_width = l_width - Px(x_offset as i32);
                let b_width = l_width - a_width;
                a_last_line.2 = a_width;
                b.lines[0].2 = b_width;
                self.lines.push(a_last_line);

                for g in &mut b.glyphs[..(a_last_line.0 as usize - segment)] {
                    g.point.x -= x_offset;
                }
            }

            if self.fonts.is_empty() {
                self.fonts.push((b.fonts[0].0.clone(), self.glyphs.len()))
            }

            self.size.width = self.lines.iter().map(|(_, _, w)| *w).max().unwrap();

            let a_lines = Px(self.lines.len() as i32);
            let a_height = self.line_height * a_lines + self.line_spacing * (a_lines - Px(1));
            self.size.height = a_height;

            b.size.width = b.lines.iter().map(|(_, _, w)| *w).max().unwrap();

            let b_lines = Px(self.lines.len() as i32);
            b.size.height = b.line_height * b_lines + b.line_spacing * (b_lines - Px(1));

            let b_y_offset = a_height.0 as f32;
            for g in &mut b.glyphs {
                g.point.y -= b_y_offset;
            }

            for s in &mut b.segments {
                s.end -= g_end;
            }

            for (_, g) in &mut b.fonts {
                *g -= g_end;
            }

            for (s, _, _) in &mut b.lines {
                *s -= segment;
            }

            (self, b)
        }
    }

    /// Like [`split`] but the segment is not included in the result.
    ///
    /// [`split`]: Self::split
    pub fn split_remove(self, segment: usize) -> (ShapedText, ShapedText) {
        let (mut a, b) = self.split(segment + 1);

        let remove_seg = a.segments.pop().unwrap();
        a.glyphs.drain(remove_seg.end..);

        if a.lines[a.lines.len() - 1].0 == a.segments.len() {
            let (_, _, last_line_width) = a.lines.pop().unwrap();
            a.size.height -= a.line_height;

            if a.size.width == last_line_width {
                a.size.width = a.lines.iter().map(|(_, _, w)| *w).max().unwrap();
            }
        }

        if let Some(i) = a.fonts.iter().rposition(|(_, g)| *g < a.glyphs.len()) {
            let maybe_remove = i + 1;
            if maybe_remove < a.fonts.len() {
                if a.fonts[i].1 == a.glyphs.len() {
                    a.fonts.truncate(maybe_remove);
                } else {
                    a.fonts.truncate(maybe_remove + 1);
                    a.fonts[maybe_remove].1 = a.glyphs.len();
                }
            }
        } else {
            a.fonts[0].1 = a.glyphs.len();
        }

        (a, b)
    }

    /// Appends the `text` to the end of `self`.
    ///
    /// If `text` has padding it is removed before pushing, if `self` has padding it is used.
    pub fn extend(&mut self, mut text: ShapedText) {
        if text.is_empty() {
            return;
        }
        text.set_padding(text.padding * -Px(1));

        if self.is_empty() {
            text.set_padding(self.padding);
            *self = text;
            return;
        }

        let (fl_x_offset, y_offset) = if self.segments[self.segments.len() - 1].kind == TextSegmentKind::LineBreak {
            (0.0, (self.size.height + self.line_spacing).0 as f32)
        } else {
            let (_, ll_x, ll_width) = self.lines[self.lines.len() - 1];
            ((ll_x + ll_width).0 as f32, (self.size.height - self.line_height).0 as f32)
        };

        let g_offset = self.glyphs.len();
        let s_offset = self.segments.len();

        if fl_x_offset > 0.0 {
            let fl_end = text.lines[0].0;
            for g in &mut text.glyphs[..fl_end] {
                g.point.x += fl_x_offset;
            }
        }
        for g in &mut text.glyphs {
            g.point.y += y_offset;
        }
        self.glyphs.extend(text.glyphs);

        for s in &mut text.segments {
            s.end += s_offset;
        }
        self.segments.extend(text.segments);

        for (s, _, _) in &mut text.lines {
            *s += s_offset;
        }
        self.lines.extend(text.lines);

        for (_, g) in &mut text.fonts {
            *g += g_offset;
        }
        self.fonts.extend(text.fonts);

        self.size.width = self.size.width.max(text.size.width);
        self.size.height += text.size.height;

        // TODO combine line heights.
    }

    /// Glyphs by font.
    pub fn glyphs(&self) -> impl Iterator<Item = (&FontRef, &[GlyphInstance])> {
        let mut start = 0;
        self.fonts.iter().map(move |(font, i)| {
            let i = *i;
            let glyphs = &self.glyphs[start..i];
            start = i;
            (font, glyphs)
        })
    }

    /// Glyphs by font in the range.
    fn glyphs_range(&self, range: IndexRange) -> impl Iterator<Item = (&FontRef, &[GlyphInstance])> {
        let mut start = range.start();
        let end = range.end();
        let first_font = self.fonts.iter().position(|(_, i)| *i > start).unwrap().saturating_sub(1);

        self.fonts[first_font..].iter().map_while(move |(font, i)| {
            let i = *i;
            let i = i.min(end);

            if i > start {
                let glyphs = &self.glyphs[start..i];
                start = i;
                Some((font, glyphs))
            } else {
                None
            }
        })
    }

    /// Glyphs by font in the range, each glyph instance is paired with the *x-advance* to the next glyph or line end.
    fn glyphs_with_x_advance_range(
        &self,
        line_index: usize,
        glyph_range: IndexRange,
    ) -> impl Iterator<Item = (&FontRef, impl Iterator<Item = (GlyphInstance, f32)> + '_)> + '_ {
        let mut start = glyph_range.start();
        let (line_end, line_x, line_width) = self.lines[line_index];
        let line_end = if line_end == self.segments.len() {
            self.glyphs.len()
        } else {
            self.segments[line_end].end
        };
        self.glyphs_range(glyph_range).map(move |(font, glyphs)| {
            let glyphs_with_adv = glyphs.iter().enumerate().map(move |(i, g)| {
                let gi = start + i + 1;

                let adv = if gi == line_end {
                    let line_adv = (line_x + line_width).0 as f32;
                    line_adv - g.point.x
                } else {
                    self.glyphs[gi].point.x - g.point.x
                };

                (*g, adv)
            });

            start += glyphs.len();

            (font, glyphs_with_adv)
        })
    }

    /// Glyphs segments.
    #[inline]
    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
    }

    /// Bounding box size, the width is the longest line, the height is the sum of line heights + spacing in between,
    /// no spacing is added before the first line and after the last line.
    #[inline]
    pub fn size(&self) -> PxSize {
        self.size
    }

    /// Current applied offsets around the text block.
    ///
    /// Note this padding is already computed in all other values.
    #[inline]
    pub fn padding(&self) -> PxSideOffsets {
        self.padding
    }

    /// Reshape text to have the new `padding`.
    ///
    /// The padding
    pub fn set_padding(&mut self, padding: PxSideOffsets) {
        if self.padding == padding {
            return;
        }

        let p = padding + self.padding * Px(-1); // no Sub impl

        let offset = PxVector::new(p.left, p.top);
        let offset_f32 = euclid::vec2(offset.x.0 as f32, offset.y.0 as f32);
        for g in &mut self.glyphs {
            g.point += offset_f32;
        }
        for (_, x, _) in &mut self.lines {
            *x = p.left;
        }

        self.size.width += p.horizontal();
        self.size.height += p.vertical();
        self.padding = padding;
    }

    /// Height of a single line.
    #[inline]
    pub fn line_height(&self) -> Px {
        self.line_height
    }

    /// Vertical spacing in between lines.
    #[inline]
    pub fn line_spacing(&self) -> Px {
        self.line_spacing
    }

    /// Vertical offset from the line bottom up that is the text baseline.
    ///
    /// The *line bottom* is the [`line_height`].
    ///
    /// [`line_height`]: Self::line_height
    #[inline]
    pub fn baseline(&self) -> Px {
        self.baseline
    }

    /// Vertical offset from the bottom up that is the baseline of the last line considering the padding.
    ///
    /// The *bottom* is the [`size`] height.
    ///
    /// [`size`]: Self::size
    #[inline]
    pub fn box_baseline(&self) -> Px {
        self.baseline + self.padding.bottom
    }

    /// Vertical offset from the line bottom up that is the overline placement.
    #[inline]
    pub fn overline(&self) -> Px {
        self.overline
    }

    /// Vertical offset from the line bottom up that is the strikethrough placement.
    #[inline]
    pub fn strikethrough(&self) -> Px {
        self.strikethrough
    }

    /// Vertical offset from the line bottom up that is the font defined underline placement.
    #[inline]
    pub fn underline(&self) -> Px {
        self.underline
    }

    /// Vertical offset from the line bottom up that is the underline placement when the option for
    /// clearing all glyph descents is selected.
    #[inline]
    pub fn underline_descent(&self) -> Px {
        self.underline_descent
    }

    /// No glyphs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Iterate over [`ShapedLine`] selections split by [`LineBreak`].
    ///
    /// [`LineBreak`]: TextSegmentKind::LineBreak
    #[inline]
    pub fn lines(&self) -> impl Iterator<Item = ShapedLine> {
        let mut start = 0;
        self.lines.iter().copied().enumerate().map(move |(i, (s, x, w))| {
            let range = IndexRange(start, s);
            start = s;

            ShapedLine {
                text: self,
                seg_range: range,
                index: i,
                x,
                width: w,
            }
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
    x: Px,
    width: Px,
}
impl<'a> fmt::Debug for ShapedLine<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedLine")
            .field("seg_range", &self.seg_range)
            .field("index", &self.index)
            .field("x", &self.x)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedLine<'a> {
    /// Bounds of the line.
    pub fn rect(&self) -> PxRect {
        let size = PxSize::new(self.width, self.text.line_height);
        let origin = PxPoint::new(self.x, self.text.line_height * Px(self.index as i32));
        PxRect::new(origin, size)
    }

    /// Horizontal alignment advance applied to the entire line.
    #[inline]
    pub fn x(&self) -> Px {
        self.x
    }

    /// Full overline, start point + width.
    #[inline]
    pub fn overline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.overline)
    }

    /// Full strikethrough line, start point + width.
    #[inline]
    pub fn strikethrough(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.strikethrough)
    }

    /// Full underline, not skipping.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns start point + width.
    #[inline]
    pub fn underline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline)
    }

    /// Full underline, not skipping.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns start point + width.
    #[inline]
    pub fn underline_descent(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline_descent)
    }

    /// Underline, skipping spaces.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns and iterator of start point + width for each word.
    #[inline]
    pub fn underline_skip_spaces(&self) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.parts().filter(|s| s.is_word()).map(|s| s.underline()))
    }

    /// Underline, skipping spaces.
    ///
    /// The *y* is the baseline + descent + 1px.
    ///
    /// Returns and iterator of start point + width for each word.
    #[inline]
    pub fn underline_descent_skip_spaces(&self) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.parts().filter(|s| s.is_word()).map(|s| s.underline_descent()))
    }

    /// Underline, skipping glyph descends that intersect the underline.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns an iterator of start point + width for continuous underline.
    #[inline]
    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(self.parts().flat_map(move |s| s.underline_skip_glyphs(thickness)))
    }

    /// Underline, skipping spaces and glyph descends that intersect the underline
    ///
    /// The *y* is defined by font metrics.
    ///
    /// Returns an iterator of start point + width for continuous underline.
    #[inline]
    pub fn underline_skip_glyphs_and_spaces(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        MergingLineIter::new(
            self.parts()
                .filter(|s| s.is_word())
                .flat_map(move |s| s.underline_skip_glyphs(thickness)),
        )
    }

    #[inline]
    fn decoration_line(&self, bottom_up_offset: Px) -> (PxPoint, Px) {
        let y = (self.text.line_height * Px((self.index as i32) + 1)) - bottom_up_offset;
        (PxPoint::new(self.x, y + self.text.padding.top), self.width)
    }

    /// Text segments of the line, does not include the line-break that started the line, can include
    /// the line break that starts the next line.
    #[inline]
    pub fn segments(&self) -> &'a [TextSegment] {
        &self.text.segments[self.seg_range.iter()]
    }

    /// Glyphs in the line.
    #[inline]
    pub fn glyphs(&self) -> impl Iterator<Item = (&'a FontRef, &'a [GlyphInstance])> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_range(r)
    }

    /// Glyphs in the line paired with the *x-advance* to the next glyph or the end of the line.
    #[inline]
    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a FontRef, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_with_x_advance_range(self.index, r)
    }

    fn glyphs_range(&self) -> IndexRange {
        let start = if self.seg_range.start() == 0 {
            0
        } else {
            self.text.segments[self.seg_range.inclusive_end()].end
        };
        let end = self.text.segments[self.seg_range.inclusive_end()].end;

        IndexRange(start, end)
    }

    /// Iterate over word and space segments in this line.
    #[inline]
    pub fn parts(&self) -> impl Iterator<Item = ShapedSegment<'a>> {
        let text = self.text;
        let line_index = self.index;
        let last_i = self.seg_range.inclusive_end();
        self.seg_range.iter().map(move |i| ShapedSegment {
            text,
            line_index,
            index: i,
            is_last: i == last_i,
        })
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
    is_last: bool,
}
impl<'a> fmt::Debug for ShapedSegment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedSegment")
            .field("line_index", &self.line_index)
            .field("index", &self.index)
            .field("is_last", &self.is_last)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedSegment<'a> {
    /// Segment kind.
    #[inline]
    pub fn kind(&self) -> TextSegmentKind {
        self.text.segments[self.index].kind
    }

    /// If the segment kind is [`Word`].
    ///
    /// [`Word`]: TextSegmentKind::Word
    #[inline]
    pub fn is_word(&self) -> bool {
        matches!(self.kind(), TextSegmentKind::Word)
    }

    /// If the segment kind is [`Space`] or [`Tab`].
    ///
    /// [`Space`]: TextSegmentKind::Space
    /// [`Tab`]: TextSegmentKind::Tab
    #[inline]
    pub fn is_space(&self) -> bool {
        matches!(self.kind(), TextSegmentKind::Space | TextSegmentKind::Tab)
    }

    /// If this is the last segment of the line.
    #[inline]
    pub fn is_last(&self) -> bool {
        self.is_last
    }

    fn glyph_range(&self) -> IndexRange {
        let start = if self.index == 0 {
            0
        } else {
            self.text.segments[self.index - 1].end
        };
        let end = self.text.segments[self.index].end;

        IndexRange(start, end)
    }

    /// Glyphs in the word or space.
    #[inline]
    pub fn glyphs(&self) -> impl Iterator<Item = (&'a FontRef, &'a [GlyphInstance])> {
        let r = self.glyph_range();
        self.text.glyphs_range(r)
    }

    /// Glyphs in the word or space, paired with the *x-advance* to then next glyph or line end.
    #[inline]
    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a FontRef, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        let r = self.glyph_range();
        self.text.glyphs_with_x_advance_range(self.line_index, r)
    }

    fn x_width(&self) -> (Px, Px) {
        let IndexRange(start, end) = self.glyph_range();

        let start_x = self.text.glyphs[start].point.x;
        let end_x = if self.is_last {
            self.text.lines[self.line_index].2 .0 as f32
        } else {
            self.text.glyphs[end].point.x
        };

        (Px(start_x as i32), Px((end_x - start_x) as i32))
    }

    /// Bounds of the word or spaces.
    pub fn rect(&self) -> PxRect {
        let (x, width) = self.x_width();
        let size = PxSize::new(width, self.text.line_height);
        let origin = PxPoint::new(x, self.text.line_height * Px(self.line_index as i32));
        PxRect::new(origin, size)
    }

    #[inline]
    fn decoration_line(&self, bottom_up_offset: Px) -> (PxPoint, Px) {
        let (x, width) = self.x_width();
        let y = (self.text.line_height * Px((self.line_index as i32) + 1)) - bottom_up_offset;
        (PxPoint::new(x, y + self.text.padding.top), width)
    }

    /// Overline spanning the word or spaces, start point + width.
    #[inline]
    pub fn overline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.overline)
    }

    /// Strikethrough spanning the word or spaces, start point + width.
    #[inline]
    pub fn strikethrough(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.strikethrough)
    }

    /// Underline spanning the word or spaces, not skipping.
    ///
    /// The *y* is defined by the font metrics.
    ///
    /// Returns start point + width.
    #[inline]
    pub fn underline(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline)
    }

    /// Underline spanning the word or spaces, skipping glyph descends that intercept the line.
    ///
    /// Returns an iterator of start point + width for underline segments.
    #[inline]
    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + 'a {
        let y = (self.text.line_height * Px((self.line_index as i32) + 1)) - self.text.underline;
        let y = y + self.text.padding.top;
        let (x, _) = self.x_width();

        let line_y = -(self.text.baseline - self.text.underline).0 as f32;
        let line_y_range = (line_y, line_y - thickness.0 as f32);

        // space around glyph descends, thickness clamped to a minimum of 1px and a maximum of 0.2em (same as Firefox).
        let padding = (thickness.0 as f32).min(self.text.fonts[0].0.size().0 as f32 * 0.2).max(1.0);

        // no yield, only sadness
        struct UnderlineSkipGlyphs<'a, I, J> {
            line_y_range: (f32, f32),
            y: Px,
            padding: f32,
            min_width: Px,

            iter: I,
            resume: Option<(&'a FontRef, J)>,
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
            I: Iterator<Item = (&'a FontRef, J)>,
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
    #[inline]
    pub fn underline_descent(&self) -> (PxPoint, Px) {
        self.decoration_line(self.text.underline_descent)
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
    lang: Lang,
    font_features: Option<Box<[usize]>>,
}
impl WordContextKey {
    pub fn new(config: &TextShapingArgs) -> Self {
        let is_64 = mem::size_of::<usize>() == mem::size_of::<u64>();

        let mut font_features = None;

        if !config.font_features.is_empty() {
            let mut features: Vec<_> = Vec::with_capacity(config.font_features.len() * if is_64 { 3 } else { 4 });
            for feature in &config.font_features {
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

            font_features = Some(features.into_boxed_slice());
        }

        WordContextKey {
            lang: config.lang.clone(),
            font_features,
        }
    }
}

#[derive(Debug)]
pub(super) struct ShapedSegmentData {
    glyphs: Vec<ShapedGlyph>,
    x_advance: f32,
    y_advance: f32,
}
#[derive(Debug, Clone, Copy)]
struct ShapedGlyph {
    index: u32,
    //cluster: u32,
    point: (f32, f32),
}

impl Font {
    fn buffer_segment(&self, segment: &str, lang: &Lang) -> harfbuzz_rs::UnicodeBuffer {
        let mut buffer =
            harfbuzz_rs::UnicodeBuffer::new().set_direction(if lang.character_direction() == unic_langid::CharacterDirection::RTL {
                harfbuzz_rs::Direction::Rtl
            } else {
                harfbuzz_rs::Direction::Ltr
            });

        if let Some(lang) = to_buzz_lang(lang.language) {
            buffer = buffer.set_language(lang);
        }
        if let Some(script) = lang.script {
            buffer = buffer.set_script(to_buzz_script(script))
        }

        buffer.add_str(segment)
    }

    fn shape_segment_no_cache(&self, seg: &str, lang: &Lang, features: &[harfbuzz_rs::Feature]) -> ShapedSegmentData {
        let size_scale = self.metrics().size_scale;
        let to_layout = |p: i32| p as f32 * size_scale;

        let buffer = self.buffer_segment(seg, lang);
        let buffer = harfbuzz_rs::shape(self.harfbuzz_font(), buffer, features);

        let mut w_x_advance = 0.0;
        let mut w_y_advance = 0.0;
        let glyphs: Vec<_> = buffer
            .get_glyph_infos()
            .iter()
            .zip(buffer.get_glyph_positions())
            .map(|(i, p)| {
                let x_offset = to_layout(p.x_offset);
                let y_offset = to_layout(p.y_offset);
                let x_advance = to_layout(p.x_advance);
                let y_advance = to_layout(p.y_advance);

                let point = (w_x_advance + x_offset, w_y_advance + y_offset);
                w_x_advance += x_advance;
                w_y_advance += y_advance;

                ShapedGlyph {
                    index: i.codepoint,
                    // cluster: i.cluster,
                    point,
                }
            })
            .collect();

        ShapedSegmentData {
            glyphs,
            x_advance: w_x_advance,
            y_advance: w_y_advance,
        }
    }

    fn shape_segment(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        lang: &Lang,
        features: &[harfbuzz_rs::Feature],
        out: impl FnOnce(&ShapedSegmentData),
    ) {
        if !(1..=WORD_CACHE_MAX_LEN).contains(&seg.len()) {
            let seg = self.shape_segment_no_cache(seg, lang, features);
            out(&seg);
        } else if let Some(small) = Self::to_small_word(seg) {
            let mut cache = self.small_word_cache.borrow_mut();

            if cache.len() > WORD_CACHE_MAX_ENTRIES {
                cache.clear();
            }

            let mut hasher = cache.hasher().build_hasher();
            WordCacheKeyRef {
                string: &small,
                ctx_key: word_ctx_key,
            }
            .hash(&mut hasher);
            let hash = hasher.finish();

            let seg = cache
                .raw_entry_mut()
                .from_hash(hash, |e| e.string == small && &e.ctx_key == word_ctx_key)
                .or_insert_with(|| {
                    let key = WordCacheKey {
                        string: small,
                        ctx_key: word_ctx_key.clone(),
                    };
                    let value = self.shape_segment_no_cache(seg, lang, features);
                    (key, value)
                })
                .1;

            out(seg)
        } else {
            let mut cache = self.word_cache.borrow_mut();

            if cache.len() > WORD_CACHE_MAX_ENTRIES {
                cache.clear();
            }

            let mut hasher = cache.hasher().build_hasher();
            WordCacheKeyRef {
                string: &seg,
                ctx_key: word_ctx_key,
            }
            .hash(&mut hasher);
            let hash = hasher.finish();

            let seg = cache
                .raw_entry_mut()
                .from_hash(hash, |e| e.string.as_str() == seg && &e.ctx_key == word_ctx_key)
                .or_insert_with(|| {
                    let key = WordCacheKey {
                        string: InternedStr::get_or_insert(seg),
                        ctx_key: word_ctx_key.clone(),
                    };
                    let value = self.shape_segment_no_cache(seg, lang, features);
                    (key, value)
                })
                .1;

            out(seg)
        }
    }

    /// Glyph index for the space `' ' ` character.
    pub fn space_index(&self) -> GlyphIndex {
        self.font.get_nominal_glyph(' ').unwrap_or(0)
    }

    /// Returns the horizontal advance of the space `' '` character.
    pub fn space_x_advance(&self) -> Px {
        let mut adv = 0.0;
        self.shape_segment(
            " ",
            &WordContextKey {
                lang: Lang::default(),
                font_features: None,
            },
            &Lang::default(),
            &[],
            |r| adv = r.x_advance,
        );

        Px(adv as i32)
    }

    /// Calculates a [`ShapedText`].
    pub fn shape_text(self: &FontRef, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        // let _scope = tracing::trace_span!("shape_text").entered();

        let mut out = ShapedText::default();

        let metrics = self.metrics();

        out.line_height = config.line_height;
        out.line_spacing = config.line_spacing;

        let line_height = config.line_height.0 as f32;
        let line_spacing = config.line_spacing.0 as f32;
        let baseline = metrics.ascent + metrics.line_gap / 2.0;

        out.baseline = out.line_height - baseline;
        out.underline = out.baseline + metrics.underline_position;
        out.underline_descent = out.baseline + metrics.descent + Px(1);
        out.strikethrough = out.baseline + metrics.ascent / 3.0;
        out.overline = out.baseline + metrics.ascent;

        let dft_line_height = self.metrics().line_height().0 as f32;
        let center_height = (line_height - dft_line_height) / 2.0;

        let mut origin = euclid::point2::<_, ()>(0.0, baseline.0 as f32 + center_height);
        let mut max_line_x = 0.0;

        let word_ctx_key = WordContextKey::new(config);

        let letter_spacing = config.letter_spacing.0 as f32;
        let word_spacing = config.word_spacing.0 as f32;
        let tab_x_advance = config.tab_x_advance.0 as f32;
        let tab_index = self.space_index();

        for (seg, kind) in text.iter() {
            match kind {
                TextSegmentKind::Word => {
                    self.shape_segment(seg, &word_ctx_key, &config.lang, &config.font_features, |shaped_seg| {
                        out.glyphs.extend(shaped_seg.glyphs.iter().map(|gi| {
                            let r = GlyphInstance {
                                index: gi.index,
                                point: euclid::point2(gi.point.0 + origin.x, gi.point.1 + origin.y),
                            };
                            origin.x += letter_spacing;
                            r
                        }));
                        origin.x += shaped_seg.x_advance;
                        origin.y += shaped_seg.y_advance;
                    });
                }
                TextSegmentKind::Space => {
                    self.shape_segment(seg, &word_ctx_key, &config.lang, &config.font_features, |shaped_seg| {
                        out.glyphs.extend(shaped_seg.glyphs.iter().map(|gi| {
                            let r = GlyphInstance {
                                index: gi.index,
                                point: euclid::point2(gi.point.0 + origin.x, gi.point.1 + origin.y),
                            };
                            origin.x += word_spacing;
                            r
                        }));
                        origin.x += shaped_seg.x_advance;
                        origin.y += shaped_seg.y_advance;
                    });
                }
                TextSegmentKind::Tab => {
                    let point = euclid::point2(origin.x, origin.y);
                    origin.x += tab_x_advance;
                    out.glyphs.push(GlyphInstance { index: tab_index, point });
                }
                TextSegmentKind::LineBreak => {
                    out.lines.push((out.segments.len(), Px(0), Px(origin.x as i32)));

                    max_line_x = origin.x.max(max_line_x);
                    origin.x = 0.0;
                    origin.y += line_height + line_spacing;
                }
            }

            out.segments.push(TextSegment {
                kind,
                end: out.glyphs.len(),
            });
        }

        out.lines.push((out.segments.len(), Px(0), Px(origin.x as i32)));

        // longest line width X line heights.
        out.size = PxSize::new(
            Px(origin.x.max(max_line_x) as i32),
            Px((((line_height + line_spacing) * out.lines.len() as f32) - line_spacing) as i32),
        );

        out.fonts.push((self.clone(), out.glyphs.len()));

        out
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
            .outline(glyph_id, hinting_options, &mut AdapterSink { sink, scale })
    }

    /// Returns the boundaries of a glyph in pixel units.
    ///
    /// The rectangle origin is the bottom-left of the bounds relative to the baseline.
    pub fn typographic_bounds(&self, glyph_id: GlyphIndex) -> Result<euclid::Rect<f32, Px>, GlyphLoadingError> {
        let rect = self.face().font_kit().typographic_bounds(glyph_id)?;

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
    /// Calculates a [`ShapedText`] using the [best](FontList::best) font in this list.
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        let mut r = self.best().shape_text(text, config);

        if self.len() == 1 || r.is_empty() {
            return r;
        }

        // find segments that contain unresolved glyphs (`0`) and collect replacements:
        let mut replacement_segs = vec![];
        let mut start = 0;
        for (i, seg) in r.segments.iter().enumerate() {
            let glyphs = &r.glyphs[start..seg.end];
            if glyphs.iter().any(|g| g.index == 0) {
                // try fallbacks:
                for font in &self[1..] {
                    let text = text.get_clone(i).unwrap();
                    let replacement = font.shape_text(&text, config);

                    if replacement.glyphs.iter().all(|g| g.index != 0) {
                        replacement_segs.push((i, replacement));
                        break;
                    }
                }
            }
            start = seg.end;
        }

        if replacement_segs.is_empty() {
            r
        } else if r.segments.len() == replacement_segs.len() {
            // all segments replacement, concat replacements:
            let mut iter = replacement_segs.into_iter();
            let (_, mut r) = iter.next().unwrap();
            for (_, repl) in iter {
                r.extend(repl);
            }
            r
        } else {
            let mut i_correction = 0isize;
            for (i, repl) in replacement_segs {
                let i = (i as isize + i_correction) as usize;
                i_correction += (repl.segments.len() as isize) - 1;

                let (mut head, tail) = r.split_remove(i);
                head.extend(repl);
                head.extend(tail);

                r = head;
            }
            r
        }
    }
}

fn to_buzz_lang(lang: unic_langid::subtags::Language) -> Option<harfbuzz_rs::Language> {
    lang.as_str().parse().ok()
}

fn to_buzz_script(script: unic_langid::subtags::Script) -> harfbuzz_rs::Tag {
    let t: u32 = script.into();
    let t = t.to_le_bytes(); // Script is a TinyStr4 that uses LE
    harfbuzz_rs::Tag::from(&[t[0], t[1], t[2], t[3]])
}
