use std::{
    cmp, fmt,
    hash::{BuildHasher, Hash},
    mem, ops,
    sync::Arc,
};

use zng_app::widget::info::InlineSegmentInfo;
use zng_ext_image::{IMAGES, ImageDataFormat, ImageSource, ImageVar};
use zng_ext_l10n::{Lang, lang};
use zng_layout::{
    context::{InlineConstraintsLayout, InlineConstraintsMeasure, InlineSegmentPos, LayoutDirection, TextSegmentKind},
    unit::{Align, Factor2d, FactorUnits, Px, PxBox, PxConstraints2d, PxPoint, PxRect, PxSize, about_eq, euclid},
};
use zng_txt::Txt;
use zng_var::{AnyVar, Var as _};
use zng_view_api::font::{GlyphIndex, GlyphInstance};

use crate::{
    BidiLevel, CaretIndex, Font, FontList, HYPHENATION, Hyphens, Justify, LineBreak, SegmentedText, TextSegment, WordBreak,
    font_features::RFontFeatures,
};

/// Reasons why a font might fail to load a glyph.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GlyphLoadingError {
    /// The font didn't contain a glyph with that ID.
    NoSuchGlyph,
    /// A platform function returned an error.
    PlatformError,
}
impl std::error::Error for GlyphLoadingError {}
impl fmt::Display for GlyphLoadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use GlyphLoadingError::*;
        match self {
            NoSuchGlyph => write!(f, "no such glyph"),
            PlatformError => write!(f, "platform error"),
        }
    }
}

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
    /// [`FontMetrics::line_height`]: crate::FontMetrics::line_height
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
    /// Is `Px::MAX` when text wrap is disabled.
    pub max_width: Px,

    /// Line break config for Chinese, Japanese, or Korean text.
    pub line_break: LineBreak,

    /// World break config.
    ///
    /// This value is only considered if it is impossible to fit the word to a line.
    pub word_break: WordBreak,

    /// Hyphen breaks config.
    pub hyphens: Hyphens,

    /// Character rendered when text is hyphenated by break.
    pub hyphen_char: Txt,

    /// Obscure the text with the replacement char.
    pub obscuring_char: Option<char>,
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
            obscuring_char: None,
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
    directions: LayoutDirections,
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

#[derive(Clone)]
struct GlyphImage(ImageVar);
impl PartialEq for GlyphImage {
    fn eq(&self, other: &Self) -> bool {
        self.0.var_ptr() == other.0.var_ptr()
    }
}
impl fmt::Debug for GlyphImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlyphImage(_)")
    }
}

/// Output of [text layout].
///
/// [text layout]: Font::shape_text
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText {
    // glyphs are in text order by segments and in visual (LTR) order within segments.
    glyphs: Vec<GlyphInstance>,
    // char byte index of each glyph in the segment that covers it.
    clusters: Vec<u32>,
    // segments of `glyphs` and `clusters`.
    segments: GlyphSegmentVec,
    lines: LineRangeVec,
    fonts: FontRangeVec,
    // sorted map of `glyphs` index -> image.
    images: Vec<(u32, GlyphImage)>,

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
    justify: Justify,    // applied justify if `align` is FILL_X
    justified: Vec<f32>, // each line has up to 3 values here, depending on first/last segs are trimmed
    overflow_align: Align,
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
        point: euclid::Point2D<f32, Px>,
        /// The glyph that is replaced by `glyphs`.
        ///
        /// Must be used as fallback if any `glyphs` cannot be rendered.
        base_glyph: GlyphIndex,

        /// The colored glyph components.
        glyphs: super::ColorGlyph<'a>,
    },
}

/// Represents normal and image glyphs in [`ShapedText::image_glyphs`].
pub enum ShapedImageGlyphs<'a> {
    /// Sequence of not image glyphs.
    Normal(&'a [GlyphInstance]),
    /// Image glyph.
    Image {
        /// Origin and size of the image in the shaped text.
        ///
        /// The size is empty is the image has not loaded yet.
        rect: euclid::Rect<f32, Px>,
        /// The glyph that is replaced by `img`.
        ///
        /// Must be used as fallback if the `img` cannot be rendered.
        base_glyph: GlyphIndex,
        /// The image.
        img: &'a ImageVar,
    },
}

impl ShapedText {
    /// New empty text.
    pub fn new(font: &Font) -> Self {
        font.shape_text(&SegmentedText::new("", LayoutDirection::LTR), &TextShapingArgs::default())
    }

    /// Glyphs by font.
    ///
    /// The glyphs are in text order by segments and in visual order (LTR) within segments, so
    /// the RTL text "لما " will have the space glyph first, then "’álif", "miim", "láam".
    ///
    /// All glyph points are set as offsets to the top-left of the text full text.
    ///
    /// Note that multiple glyphs can map to the same char and multiple chars can map to the same glyph.
    pub fn glyphs(&self) -> impl Iterator<Item = (&Font, &[GlyphInstance])> {
        self.fonts.iter_glyphs().map(move |(f, r)| (f, &self.glyphs[r.iter()]))
    }

    /// Glyphs in a range by font.
    ///
    /// Similar output to [`glyphs`], but only glyphs in the `range`.
    ///
    /// [`glyphs`]: Self::glyphs
    pub fn glyphs_slice(&self, range: impl ops::RangeBounds<usize>) -> impl Iterator<Item = (&Font, &[GlyphInstance])> {
        self.glyphs_slice_impl(IndexRange::from_bounds(range))
    }
    fn glyphs_slice_impl(&self, range: IndexRange) -> impl Iterator<Item = (&Font, &[GlyphInstance])> {
        self.fonts.iter_glyphs_clip(range).map(move |(f, r)| (f, &self.glyphs[r.iter()]))
    }

    /// If the shaped text has any Emoji glyph associated with a font that has color palettes.
    pub fn has_colored_glyphs(&self) -> bool {
        self.has_colored_glyphs
    }

    /// If the shaped text has any Emoji glyph associated with a pixel image.
    pub fn has_images(&self) -> bool {
        !self.images.is_empty()
    }

    /// Glyphs by font and palette color.
    pub fn colored_glyphs(&self) -> impl Iterator<Item = (&Font, ShapedColoredGlyphs)> {
        ColoredGlyphsIter {
            glyphs: self.glyphs(),
            maybe_colored: None,
        }
    }

    /// Glyphs in a range by font and palette color.
    pub fn colored_glyphs_slice(&self, range: impl ops::RangeBounds<usize>) -> impl Iterator<Item = (&Font, ShapedColoredGlyphs)> {
        ColoredGlyphsIter {
            glyphs: self.glyphs_slice_impl(IndexRange::from_bounds(range)),
            maybe_colored: None,
        }
    }

    /// Glyphs by font and associated image.
    pub fn image_glyphs(&self) -> impl Iterator<Item = (&Font, ShapedImageGlyphs)> {
        ImageGlyphsIter {
            glyphs: self.glyphs(),
            glyphs_i: 0,
            images: &self.images,
            maybe_img: None,
        }
    }

    /// Glyphs in a range by font and associated image.
    pub fn image_glyphs_slice(&self, range: impl ops::RangeBounds<usize>) -> impl Iterator<Item = (&Font, ShapedImageGlyphs)> {
        let range = IndexRange::from_bounds(range);
        ImageGlyphsIter {
            glyphs_i: range.start() as _,
            glyphs: self.glyphs_slice_impl(range),
            images: &self.images,
            maybe_img: None,
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

    fn seg_glyphs_with_x_advance(
        &self,
        seg_idx: usize,
        glyphs_range: IndexRange,
    ) -> impl Iterator<Item = (&Font, impl Iterator<Item = (GlyphInstance, f32)> + '_)> + '_ {
        let mut gi = glyphs_range.start();
        let seg_x = if gi < self.glyphs.len() { self.glyphs[gi].point.x } else { 0.0 };
        let seg_advance = self.segments.0[seg_idx].advance;
        self.glyphs_range(glyphs_range).map(move |(font, glyphs)| {
            let g_adv = glyphs.iter().map(move |g| {
                gi += 1;

                let adv = if gi == glyphs_range.end() {
                    (seg_x + seg_advance) - g.point.x
                } else {
                    self.glyphs[gi].point.x - g.point.x
                };
                (*g, adv)
            });

            (font, g_adv)
        })
    }

    fn seg_cluster_glyphs_with_x_advance(
        &self,
        seg_idx: usize,
        glyphs_range: IndexRange,
    ) -> impl Iterator<Item = (&Font, impl Iterator<Item = (u32, &[GlyphInstance], f32)>)> {
        let mut gi = glyphs_range.start();
        let seg_x = if gi < self.glyphs.len() { self.glyphs[gi].point.x } else { 0.0 };
        let seg_advance = self.segments.0[seg_idx].advance;
        let seg_clusters = self.clusters_range(glyphs_range);
        let mut cluster_i = 0;

        self.glyphs_range(glyphs_range).map(move |(font, glyphs)| {
            let clusters = &seg_clusters[cluster_i..cluster_i + glyphs.len()];
            cluster_i += glyphs.len();

            struct Iter<'a> {
                clusters: &'a [u32],
                glyphs: &'a [GlyphInstance],
            }
            impl<'a> Iterator for Iter<'a> {
                type Item = (u32, &'a [GlyphInstance]);

                fn next(&mut self) -> Option<Self::Item> {
                    if let Some(c) = self.clusters.first() {
                        let end = self.clusters.iter().rposition(|rc| rc == c).unwrap();
                        let glyphs = &self.glyphs[..=end];
                        self.clusters = &self.clusters[end + 1..];
                        self.glyphs = &self.glyphs[end + 1..];
                        Some((*c, glyphs))
                    } else {
                        None
                    }
                }
            }

            let g_adv = Iter { clusters, glyphs }.map(move |(c, gs)| {
                gi += gs.len();

                let adv = if gi == glyphs_range.end() {
                    (seg_x + seg_advance) - gs[0].point.x
                } else {
                    self.glyphs[gi].point.x - gs[0].point.x
                };
                (c, gs, adv)
            });

            (font, g_adv)
        })
    }

    /// Bounding box size, the width is the longest line or the first or
    /// last line width + absolute offset, the height is the bottom-most point of the last line.
    pub fn size(&self) -> PxSize {
        let first_width = self.first_line.origin.x.abs() + self.first_line.size.width;
        let last_width = self.last_line.origin.x.abs() + self.last_line.size.width;
        self.mid_size()
            .max(PxSize::new(first_width.max(last_width), self.last_line.max_y()))
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

    /// Gets the first line that overflows the `max_height`. A line overflows when the line `PxRect::max_y`
    /// is greater than `max_height`.
    pub fn overflow_line(&self, max_height: Px) -> Option<ShapedLine> {
        let mut y = self.first_line.max_y();
        if y > max_height {
            self.line(0)
        } else if self.lines.0.len() > 1 {
            let mid_lines = self.lines.0.len() - 2;
            for i in 0..=mid_lines {
                y += self.line_spacing;
                y += self.line_height;
                if y > max_height {
                    return self.line(i + 1);
                }
            }

            if self.last_line.max_y() > max_height {
                self.line(self.lines.0.len() - 1)
            } else {
                None
            }
        } else {
            None
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
    /// offset is defined by the first line rectangle plus the [`mid_clear`].
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

    /// Last applied justify.
    ///
    /// This is the resolved mode, it is never `Auto`.
    ///
    /// [`align`]: Self::align
    pub fn justify_mode(&self) -> Option<Justify> {
        match self.justify {
            Justify::Auto => None,
            m => {
                debug_assert!(self.align.is_fill_x());
                Some(m)
            }
        }
    }

    /// Last applied overflow alignment.
    ///
    /// Only used in dimensions of the text that overflow [`align_size`].
    ///
    /// [`align_size`]: Self::align_size
    pub fn overflow_align(&self) -> Align {
        self.overflow_align
    }

    /// Last applied alignment area.
    ///
    /// The lines are aligned inside this size. If the text is inlined only the mid-lines are aligned and only horizontally.
    pub fn align_size(&self) -> PxSize {
        self.align_size
    }

    /// Last applied alignment direction.
    ///
    /// Note that the glyph and word directions is defined by the [`TextShapingArgs::lang`] and the computed
    /// direction is in [`ShapedSegment::direction`].
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
    ///
    /// Note that this method clears justify fill, of `align` is fill X you must call [`reshape_lines_justify`] after to refill.
    ///
    /// [`reshape_lines_justify`]: Self::reshape_lines_justify
    #[expect(clippy::too_many_arguments)]
    pub fn reshape_lines(
        &mut self,
        constraints: PxConstraints2d,
        inline_constraints: Option<InlineConstraintsLayout>,
        align: Align,
        overflow_align: Align,
        line_height: Px,
        line_spacing: Px,
        direction: LayoutDirection,
    ) {
        self.clear_justify_impl(align.is_fill_x());
        self.reshape_line_height_and_spacing(line_height, line_spacing);

        let is_inlined = inline_constraints.is_some();

        let align_x = align.x(direction);
        let align_y = if is_inlined { 0.fct() } else { align.y() };
        let overflow_align_x = overflow_align.x(direction);
        let overflow_align_y = if is_inlined { 0.fct() } else { overflow_align.y() };

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

            match first.size.width.cmp(&align_size.width) {
                cmp::Ordering::Less => first.origin.x = (align_size.width - first.size.width) * align_x,
                cmp::Ordering::Equal => {}
                cmp::Ordering::Greater => first.origin.x = (align_size.width - first.size.width) * overflow_align_x,
            }
            match last.size.width.cmp(&align_size.width) {
                cmp::Ordering::Less => last.origin.x = (align_size.width - last.size.width) * align_x,
                cmp::Ordering::Equal => {}
                cmp::Ordering::Greater => last.origin.x = (align_size.width - last.size.width) * overflow_align_x,
            }

            match block_size.height.cmp(&align_size.height) {
                cmp::Ordering::Less => {
                    let align_y = (align_size.height - block_size.height) * align_y;
                    first.origin.y += align_y;
                    last.origin.y += align_y;
                }
                cmp::Ordering::Equal => {}
                cmp::Ordering::Greater => {
                    let align_y = (align_size.height - block_size.height) * overflow_align_y;
                    first.origin.y += align_y;
                    last.origin.y += align_y;
                }
            }

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
                // width is the same measured, unless the parent inliner changed it to fill,
                // in that case we need the original width in `reshape_lines_justify`.
                // first_line.width = first.size.width.0 as f32;
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
                // width is the same measured, unless the parent inliner changed it to justify, that is handled later.
                // last_line.width = last.size.width.0 as f32;
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

            let mid_offset = euclid::vec2::<f32, Px>(
                0.0,
                match block_size.height.cmp(&align_size.height) {
                    cmp::Ordering::Less => (align_size.height - block_size.height).0 as f32 * align_y + mid.0 as f32,
                    cmp::Ordering::Equal => mid.0 as f32,
                    cmp::Ordering::Greater => (align_size.height - block_size.height).0 as f32 * overflow_align_y + mid.0 as f32,
                },
            );
            let y_transform = mid_offset.y - self.mid_offset;
            let align_width = align_size.width.0 as f32;

            let skip_last = self.lines.0.len() - 2;
            let mut line_start = self.lines.0[0].end;
            for line in &mut self.lines.0[1..=skip_last] {
                let x_offset = if line.width < align_width {
                    (align_width - line.width) * align_x
                } else {
                    (align_width - line.width) * overflow_align_x
                };
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
        self.direction = direction;
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
            Align::TOP_LEFT,
            self.orig_line_height,
            self.orig_line_spacing,
            LayoutDirection::LTR,
        );
    }

    fn justify_lines_range(&self) -> ops::Range<usize> {
        let mut range = 0..self.lines_len();

        if !self.is_inlined {
            // skip last line
            range.end = range.end.saturating_sub(1);
        }
        // else inlined fills the first and last line rects

        range
    }

    /// Replace the applied [`justify_mode`], if the [`align`] is fill X.
    ///
    /// [`justify_mode`]: Self::justify_mode
    /// [`align`]: Self::align
    pub fn reshape_lines_justify(&mut self, mode: Justify, lang: &Lang) {
        self.clear_justify_impl(true);

        if !self.align.is_fill_x() {
            return;
        }

        let mode = mode.resolve(lang);

        let range = self.justify_lines_range();

        let fill_width = self.align_size.width.0 as f32;
        let last_li = range.end.saturating_sub(1);

        for li in range.clone() {
            let mut count;
            let mut space;
            let mut line_seg_range;
            let mut offset = 0.0;
            let mut last_is_space = false;

            let mut fill_width = fill_width;
            if self.is_inlined {
                // inlining parent provides the fill space for the first and last segment
                if li == 0 {
                    fill_width = self.first_line.width().0 as f32;
                } else if li == last_li {
                    fill_width = self.last_line.width().0 as f32;
                }
            }

            {
                // line scope
                let line = self.line(li).unwrap();

                // count of space insert points
                count = match mode {
                    Justify::InterWord => line.segs().filter(|s| s.kind().is_space()).count(),
                    Justify::InterLetter => line
                        .segs()
                        .map(|s| {
                            if s.kind().is_space() {
                                s.clusters_count().saturating_sub(1).max(1)
                            } else if s.kind().is_word() {
                                s.clusters_count().saturating_sub(1)
                            } else {
                                0
                            }
                        })
                        .sum(),
                    Justify::Auto => unreachable!(),
                };

                // space to distribute
                space = fill_width - self.lines.0[li].width;

                line_seg_range = 0..line.segs_len();

                // trim spaces at start and end
                let mut first_is_space = false;

                if let Some(s) = line.seg(0) {
                    if s.kind().is_space() && (!self.is_inlined || li > 0 || self.first_line.origin.x == Px(0)) {
                        // trim start, unless it inlining and the first seg is actually a continuation of another text on the same row
                        first_is_space = true;
                        count -= 1;
                        space += s.advance();
                    }
                }
                if let Some(s) = line.seg(line.segs_len().saturating_sub(1)) {
                    if s.kind().is_space()
                        && (!self.is_inlined || li < range.end - 1 || about_eq(self.first_line.size.width.0 as f32, fill_width, 1.0))
                    {
                        // trim end, unless its inlining and the last seg continues
                        last_is_space = true;
                        count -= 1;
                        space += s.advance();
                    }
                }
                if first_is_space {
                    line_seg_range.start += 1;
                    let gsi = self.line(li).unwrap().seg_range.start();
                    let adv = mem::take(&mut self.segments.0[gsi].advance);
                    offset -= adv;
                    self.justified.push(adv);
                }
                if last_is_space {
                    line_seg_range.end = line_seg_range.end.saturating_sub(1);
                    let gsi = self.line(li).unwrap().seg_range.end().saturating_sub(1);
                    let adv = mem::take(&mut self.segments.0[gsi].advance);
                    self.justified.push(adv);
                }
                if line_seg_range.start > line_seg_range.end {
                    line_seg_range = 0..0;
                }
            }
            let justify_advance = space / count as f32;
            self.justified.push(justify_advance);

            for si in line_seg_range {
                let is_space;
                let glyphs_range;
                let gsi;
                {
                    let line = self.line(li).unwrap();
                    let seg = line.seg(si).unwrap();

                    is_space = seg.kind().is_space();
                    glyphs_range = seg.glyphs_range();
                    gsi = line.seg_range.start() + si;
                }

                let mut cluster = if self.clusters.is_empty() {
                    0
                } else {
                    self.clusters[glyphs_range.start()]
                };
                for gi in glyphs_range {
                    self.glyphs[gi].point.x += offset;

                    if matches!(mode, Justify::InterLetter) && self.clusters[gi] != cluster {
                        cluster = self.clusters[gi];
                        offset += justify_advance;
                        self.segments.0[gsi].advance += justify_advance;
                    }
                }

                let seg = &mut self.segments.0[gsi];
                seg.x += offset;
                if is_space {
                    offset += justify_advance;
                    seg.advance += justify_advance;
                }
            }
            if last_is_space {
                let gsi = self.line(li).unwrap().seg_range.end().saturating_sub(1);
                let seg = &mut self.segments.0[gsi];
                debug_assert_eq!(seg.advance, 0.0);
                seg.x += offset;
            }
            self.justified.shrink_to_fit();
        }

        self.justify = mode;
    }

    /// Remove the currently applied [`justify_mode`].
    ///
    /// [`justify_mode`]: Self::justify_mode
    pub fn clear_justify(&mut self) {
        self.clear_justify_impl(false)
    }
    fn clear_justify_impl(&mut self, keep_alloc: bool) {
        if self.justify_mode().is_none() {
            return;
        }

        let range = self.justify_lines_range();
        debug_assert!(range.len() <= self.justified.len());

        let mut justified_alloc = mem::take(&mut self.justified);

        let mut justified = justified_alloc.drain(..);
        for li in range {
            let mut line_seg_range;
            let mut last_is_space = false;

            let mut offset = 0.0;

            {
                let line = self.line(li).unwrap();

                line_seg_range = 0..line.segs_len();

                // trim spaces at start and end
                let mut first_is_space = false;

                if let Some(s) = line.seg(0) {
                    first_is_space = s.kind().is_space();
                }
                if let Some(s) = line.seg(line.segs_len().saturating_sub(1)) {
                    last_is_space = s.kind().is_space();
                }
                if first_is_space {
                    line_seg_range.start += 1;
                    let gsi = self.line(li).unwrap().seg_range.start();

                    let adv = justified.next().unwrap();
                    self.segments.0[gsi].advance = adv;
                    offset -= adv;
                }
                if last_is_space {
                    line_seg_range.end = line_seg_range.end.saturating_sub(1);
                    let adv = justified.next().unwrap();
                    let gsi = self.line(li).unwrap().seg_range.end().saturating_sub(1);
                    self.segments.0[gsi].advance = adv;
                }
                if line_seg_range.start > line_seg_range.end {
                    line_seg_range = 0..0;
                }
            }

            let justify_advance = justified.next().unwrap();

            for si in line_seg_range {
                let is_space;
                let glyphs_range;
                let gsi;
                {
                    let line = self.line(li).unwrap();
                    let seg = line.seg(si).unwrap();

                    is_space = seg.kind().is_space();
                    glyphs_range = seg.glyphs_range();
                    gsi = line.seg_range.start() + si;
                }

                let mut cluster = if self.clusters.is_empty() {
                    0
                } else {
                    self.clusters[glyphs_range.start()]
                };
                for gi in glyphs_range {
                    self.glyphs[gi].point.x -= offset;

                    if matches!(self.justify, Justify::InterLetter) && self.clusters[gi] != cluster {
                        cluster = self.clusters[gi];
                        offset += justify_advance;
                        self.segments.0[gsi].advance -= justify_advance;
                    }
                }

                let seg = &mut self.segments.0[gsi];
                seg.x -= offset;
                if is_space {
                    offset += justify_advance;
                    seg.advance -= justify_advance;
                }
            }
            if last_is_space {
                let gsi = self.line(li).unwrap().seg_range.end().saturating_sub(1);
                self.segments.0[gsi].x -= offset;
            }
        }

        self.justify = Justify::Auto;

        if keep_alloc {
            drop(justified);
            self.justified = justified_alloc;
        }
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
        let just_width = self.justify_mode().map(|_| self.align_size.width);
        self.lines.iter_segs().enumerate().map(move |(i, (w, r))| ShapedLine {
            text: self,
            seg_range: r,
            index: i,
            width: just_width.unwrap_or_else(|| Px(w.round() as i32)),
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
        if line_idx >= self.lines.0.len() {
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
                directions: LayoutDirections::empty(),
            }]),
            fonts: FontRangeVec(vec![FontRange {
                font: self.fonts.font(0).clone(),
                end: 0,
            }]),
            images: vec![],
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
            justify: Justify::Auto,
            justified: vec![],
            overflow_align: Align::TOP_LEFT,
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

    fn debug_assert_ranges(&self) {
        #[cfg(debug_assertions)]
        {
            #[allow(unused)]
            macro_rules! trace_assert {
                ($cond:expr $(,)?) => {
                    #[allow(clippy::all)]
                    if !($cond) {
                        tracing::error!("{}", stringify!($cond));
                        return;
                    }
                };
                ($cond:expr, $($arg:tt)+) => {
                    #[allow(clippy::all)]
                    if !($cond) {
                        tracing::error!($($arg)*);
                        return;
                    }
                };
            }

            let mut prev_seg_end = 0;
            for seg in &self.segments.0 {
                trace_assert!(seg.end >= prev_seg_end);
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
                    // false positive in cases of heavy use of combining chars
                    // only observed in "Zalgo" text, remove if we there is a legitimate
                    // Script that causing this error.
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

    /// Gets the top-middle origin for a caret visual that marks the insert `index` in the string.
    pub fn caret_origin(&self, caret: CaretIndex, full_text: &str) -> PxPoint {
        let index = caret.index;
        let mut end_line = None;
        for line in self.line(caret.line).into_iter().chain(self.lines()) {
            for seg in line.segs() {
                let txt_range = seg.text_range();
                if !txt_range.contains(&index) {
                    continue;
                }
                let local_index = index - txt_range.start;
                let is_rtl = seg.direction().is_rtl();

                let seg_rect = seg.rect();
                let mut origin = seg_rect.origin;

                let clusters = seg.clusters();
                let mut cluster_i = 0;
                let mut search_lig = true;

                if is_rtl {
                    for (i, c) in clusters.iter().enumerate().rev() {
                        match (*c as usize).cmp(&local_index) {
                            cmp::Ordering::Less => {
                                cluster_i = i;
                            }
                            cmp::Ordering::Equal => {
                                cluster_i = i;
                                search_lig = false;
                                break;
                            }
                            cmp::Ordering::Greater => break,
                        }
                    }
                } else {
                    for (i, c) in clusters.iter().enumerate() {
                        match (*c as usize).cmp(&local_index) {
                            cmp::Ordering::Less => {
                                cluster_i = i;
                            }
                            cmp::Ordering::Equal => {
                                cluster_i = i;
                                search_lig = false;
                                break;
                            }
                            cmp::Ordering::Greater => break,
                        }
                    }
                }

                let mut origin_x = origin.x.0 as f32;

                // glyphs are always in display order (LTR) and map
                // to each cluster entry.
                //
                // in both LTR and RTL we sum advance until `cluster_i` is found,
                // but in RTL we sum *back* to the char (so it needs to be covered +1)
                let mut glyph_take = cluster_i;
                if is_rtl {
                    glyph_take += 1;
                }

                let mut search_lig_data = None;

                'outer: for (font, glyphs) in seg.glyphs_with_x_advance() {
                    for (g, advance) in glyphs {
                        search_lig_data = Some((font, g.index, advance));

                        if glyph_take == 0 {
                            break 'outer;
                        }
                        origin_x += advance;
                        glyph_take -= 1;
                    }
                }

                if search_lig {
                    if let Some((font, g_index, advance)) = search_lig_data {
                        let lig_start = txt_range.start + clusters[cluster_i] as usize;
                        let lig_end = if is_rtl {
                            if cluster_i == 0 {
                                txt_range.end
                            } else {
                                txt_range.start + clusters[cluster_i - 1] as usize
                            }
                        } else {
                            clusters
                                .get(cluster_i + 1)
                                .map(|c| txt_range.start + *c as usize)
                                .unwrap_or_else(|| txt_range.end)
                        };

                        let maybe_lig = &full_text[lig_start..lig_end];

                        let lig_len = unicode_segmentation::UnicodeSegmentation::grapheme_indices(maybe_lig, true).count();
                        if lig_len > 1 {
                            // is ligature

                            let lig_taken = &full_text[lig_start..index];
                            let lig_taken = unicode_segmentation::UnicodeSegmentation::grapheme_indices(lig_taken, true).count();

                            for (i, lig_advance) in font.ligature_caret_offsets(g_index).enumerate() {
                                if i == lig_taken {
                                    // font provided ligature caret for index
                                    origin_x += lig_advance;
                                    search_lig = false;
                                    break;
                                }
                            }

                            if search_lig {
                                // synthetic lig. caret
                                let lig_advance = advance * (lig_taken as f32 / lig_len as f32);

                                if is_rtl {
                                    origin_x -= lig_advance;
                                } else {
                                    origin_x += lig_advance;
                                }
                            }
                        }
                    }
                }

                origin.x = Px(origin_x.round() as _);
                return origin;
            }

            if line.index == caret.line && line.text_range().end == index && line.ended_by_wrap() {
                // is at the end of a wrap.
                end_line = Some(line.index);
                break;
            }
        }

        // position at the end of the end_line.
        let line_end = end_line.unwrap_or_else(|| self.lines_len().saturating_sub(1));
        if let Some(line) = self.line(line_end) {
            let rect = line.rect();
            if self.direction().is_rtl() {
                // top-left of last line if it the text is RTL overall.
                PxPoint::new(rect.min_x(), rect.min_y())
            } else {
                // top-right of last line for LTR
                PxPoint::new(rect.max_x(), rect.min_y())
            }
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

    /// Changes the caret line if the current line cannot contain the current char byte index.
    ///
    /// This retains the same line at ambiguous points at the end/start of wrapped lines.
    pub fn snap_caret_line(&self, mut caret: CaretIndex) -> CaretIndex {
        for line in self.lines() {
            let range = line.text_range();

            if range.start == caret.index {
                // at start that can be by wrap
                if line.started_by_wrap() {
                    if caret.line >= line.index {
                        caret.line = line.index;
                    } else {
                        caret.line = line.index.saturating_sub(1);
                    }
                } else {
                    caret.line = line.index;
                }
                return caret;
            } else if range.contains(&caret.index) {
                // inside of line
                caret.line = line.index;
                return caret;
            }
        }
        caret.line = self.lines.0.len().saturating_sub(1);
        caret
    }

    /// Gets a full overflow analysis.
    pub fn overflow_info(&self, max_size: PxSize, overflow_suffix_width: Px) -> Option<TextOverflowInfo> {
        // check y overflow

        let (last_line, overflow_line) = match self.overflow_line(max_size.height) {
            Some(l) => {
                if l.index == 0 {
                    // all text overflows
                    return Some(TextOverflowInfo {
                        line: 0,
                        text_char: 0,
                        included_glyphs: smallvec::smallvec![],
                        suffix_origin: l.rect().origin.cast().cast_unit(),
                    });
                } else {
                    (self.line(l.index - 1).unwrap(), l.index)
                }
            }
            None => (self.line(self.lines_len().saturating_sub(1))?, self.lines_len()),
        };

        // check x overflow

        let max_width = max_size.width - overflow_suffix_width;

        if last_line.width <= max_width {
            // No x overflow
            return if overflow_line < self.lines_len() {
                Some(TextOverflowInfo {
                    line: overflow_line,
                    text_char: last_line.text_range().end,
                    included_glyphs: smallvec::smallvec_inline![0..last_line.glyphs_range().end()],
                    suffix_origin: {
                        let r = last_line.rect();
                        let mut o = r.origin;
                        match self.direction {
                            LayoutDirection::LTR => o.x += r.width(),
                            LayoutDirection::RTL => o.x -= overflow_suffix_width,
                        }
                        o.cast().cast_unit()
                    },
                })
            } else {
                None
            };
        }

        let directions = last_line.directions();
        if directions == LayoutDirections::BIDI {
            let mut included_glyphs = smallvec::SmallVec::<[ops::Range<usize>; 1]>::new_const();

            let min_x = match self.direction {
                LayoutDirection::LTR => Px(0),
                LayoutDirection::RTL => last_line.rect().max_x() - max_width,
            };
            let max_x = min_x + max_width;

            let mut end_seg = None;

            for seg in last_line.segs() {
                let (x, width) = seg.x_width();
                let seg_max_x = x + width;

                if x < max_x && seg_max_x >= min_x {
                    let mut glyphs_range = seg.glyphs_range().iter();
                    let mut text_range = seg.text_range();
                    if x < min_x {
                        if let Some((c, g)) = seg.overflow_char_glyph((width - (min_x - x)).0 as f32) {
                            glyphs_range.start += g + 1;
                            text_range.start += c;
                        }
                    } else if seg_max_x > max_x {
                        if let Some((c, g)) = seg.overflow_char_glyph((width - seg_max_x - max_x).0 as f32) {
                            glyphs_range.end -= g;
                            text_range.end -= c;
                        }
                    }

                    if let Some(l) = included_glyphs.last_mut() {
                        if l.end == glyphs_range.start {
                            l.end = glyphs_range.end;
                        } else if glyphs_range.end == l.start {
                            l.start = glyphs_range.start;
                        } else {
                            included_glyphs.push(glyphs_range.clone());
                        }
                    } else {
                        included_glyphs.push(glyphs_range.clone());
                    }

                    match self.direction {
                        LayoutDirection::LTR => {
                            if let Some((sx, se, gr, tr)) = &mut end_seg {
                                if x < *sx {
                                    *sx = x;
                                    *se = seg;
                                    *gr = glyphs_range;
                                    *tr = text_range;
                                }
                            } else {
                                end_seg = Some((x, seg, glyphs_range, text_range));
                            }
                        }
                        LayoutDirection::RTL => {
                            if let Some((smx, se, gr, tr)) = &mut end_seg {
                                if seg_max_x < *smx {
                                    *smx = seg_max_x;
                                    *se = seg;
                                    *gr = glyphs_range;
                                    *tr = text_range;
                                }
                            } else {
                                end_seg = Some((seg_max_x, seg, glyphs_range, text_range));
                            }
                        }
                    }
                }
            }

            if let Some((_, seg, glyphs_range, text_range)) = end_seg {
                Some(match self.direction {
                    LayoutDirection::LTR => TextOverflowInfo {
                        line: overflow_line,
                        text_char: text_range.end,
                        included_glyphs,
                        suffix_origin: {
                            let r = seg.rect();
                            let seg_range = seg.glyphs_range().iter();
                            let mut o = r.origin.cast().cast_unit();
                            let mut w = r.width();
                            if seg_range != glyphs_range {
                                if let Some(g) = seg.glyph(glyphs_range.end - seg_range.start) {
                                    o.x = g.1.point.x;
                                    w = Px(0);
                                }
                            }
                            o.x += w.0 as f32;
                            o
                        },
                    },
                    LayoutDirection::RTL => TextOverflowInfo {
                        line: overflow_line,
                        text_char: text_range.start,
                        included_glyphs,
                        suffix_origin: {
                            let r = seg.rect();
                            let mut o = r.origin.cast().cast_unit();
                            let seg_range = seg.glyphs_range().iter();
                            if seg_range != glyphs_range {
                                if let Some(g) = seg.glyph(glyphs_range.start - seg_range.start) {
                                    o.x = g.1.point.x;
                                }
                            }
                            o.x -= overflow_suffix_width.0 as f32;
                            o
                        },
                    },
                })
            } else {
                None
            }
        } else {
            // single direction overflow
            let mut max_width_f32 = max_width.0 as f32;
            for seg in last_line.segs() {
                let seg_advance = seg.advance();
                max_width_f32 -= seg_advance;
                if max_width_f32 <= 0.0 {
                    let seg_text_range = seg.text_range();
                    let seg_glyphs_range = seg.glyphs_range();

                    if directions == LayoutDirections::RTL {
                        let (c, g) = match seg.overflow_char_glyph(seg_advance + max_width_f32) {
                            Some(r) => r,
                            None => (seg_text_range.len(), seg_glyphs_range.len()),
                        };

                        return Some(TextOverflowInfo {
                            line: overflow_line,
                            text_char: seg_text_range.start + c,
                            included_glyphs: smallvec::smallvec![
                                0..seg_glyphs_range.start(),
                                seg_glyphs_range.start() + g + 1..seg_glyphs_range.end()
                            ],
                            suffix_origin: {
                                let mut o = if let Some(g) = seg.glyph(g + 1) {
                                    euclid::point2(g.1.point.x, seg.rect().origin.y.0 as f32)
                                } else {
                                    let rect = seg.rect();
                                    let mut o = rect.origin.cast().cast_unit();
                                    o.x += seg.advance();
                                    o
                                };
                                o.x -= overflow_suffix_width.0 as f32;
                                o
                            },
                        });
                    } else {
                        // LTR or empty

                        let (c, g) = match seg.overflow_char_glyph((max_width - seg.x_width().0).0 as f32) {
                            Some(r) => r,
                            None => (seg_text_range.len(), seg_glyphs_range.len()),
                        };

                        return Some(TextOverflowInfo {
                            line: overflow_line,
                            text_char: seg_text_range.start + c,
                            included_glyphs: smallvec::smallvec_inline![0..seg_glyphs_range.start() + g],
                            suffix_origin: {
                                if let Some(g) = seg.glyph(g) {
                                    euclid::point2(g.1.point.x, seg.rect().origin.y.0 as f32)
                                } else {
                                    let rect = seg.rect();
                                    let mut o = rect.origin.cast().cast_unit();
                                    o.x += seg.advance();
                                    o
                                }
                            },
                        });
                    }
                }
            }
            // no overflow, rounding issue?
            None
        }
    }

    /// Rectangles of the text selected by `range`.
    pub fn highlight_rects(&self, range: ops::Range<CaretIndex>, full_txt: &str) -> impl Iterator<Item = PxRect> + '_ {
        let start_origin = self.caret_origin(range.start, full_txt).x;
        let end_origin = self.caret_origin(range.end, full_txt).x;

        MergingRectIter::new(
            self.lines()
                .skip(range.start.line)
                .take(range.end.line + 1 - range.start.line)
                .flat_map(|l| l.segs())
                .skip_while(move |s| s.text_end() <= range.start.index)
                .take_while(move |s| s.text_start() < range.end.index)
                .map(move |s| {
                    let mut r = s.rect();

                    if s.text_start() <= range.start.index {
                        // first segment in selection

                        match s.direction() {
                            LayoutDirection::LTR => {
                                r.size.width = r.max_x() - start_origin;
                                r.origin.x = start_origin;
                            }
                            LayoutDirection::RTL => {
                                r.size.width = start_origin - r.origin.x;
                            }
                        }
                    }
                    if s.text_end() > range.end.index {
                        // last segment in selection

                        match s.direction() {
                            LayoutDirection::LTR => {
                                r.size.width = end_origin - r.origin.x;
                            }
                            LayoutDirection::RTL => {
                                r.size.width = r.max_x() - end_origin;
                                r.origin.x = end_origin;
                            }
                        }
                    }

                    r
                }),
        )
    }

    /// Clip under/overline to a text `clip_range` area, if `clip_out` only lines outside the range are visible.
    pub fn clip_lines(
        &self,
        clip_range: ops::Range<CaretIndex>,
        clip_out: bool,
        txt: &str,
        lines: impl Iterator<Item = (PxPoint, Px)>,
    ) -> Vec<(PxPoint, Px)> {
        let clips: Vec<_> = self.highlight_rects(clip_range, txt).collect();

        let mut out_lines = vec![];

        if clip_out {
            let mut exclude_buf = vec![];
            for (origin, width) in lines {
                let line_max = origin.x + width;

                for clip in clips.iter() {
                    if origin.y >= clip.origin.y && origin.y <= clip.max_y() {
                        // line contains
                        if origin.x < clip.max_x() && line_max > clip.origin.x {
                            // intersects
                            exclude_buf.push((clip.origin.x, clip.max_x()));
                        }
                    }
                }

                if !exclude_buf.is_empty() {
                    // clips don't overlap, enforce LTR
                    exclude_buf.sort_by_key(|(s, _)| *s);

                    if origin.x < exclude_buf[0].0 {
                        // bit before the first clip
                        out_lines.push((origin, exclude_buf[0].0 - origin.x));
                    }
                    let mut blank_start = exclude_buf[0].1;
                    for (clip_start, clip_end) in exclude_buf.drain(..).skip(1) {
                        if clip_start > blank_start {
                            // space between clips
                            if line_max > clip_start {
                                // bit in-between two clips
                                out_lines.push((PxPoint::new(blank_start, origin.y), line_max.min(clip_start) - blank_start));
                            }
                            blank_start = clip_end;
                        }
                    }
                    if line_max > blank_start {
                        // bit after the last clip
                        out_lines.push((PxPoint::new(blank_start, origin.y), line_max - blank_start));
                    }
                } else {
                    // not clipped
                    out_lines.push((origin, width));
                }
            }
        } else {
            let mut include_buf = vec![];
            for (origin, width) in lines {
                let line_max = origin.x + width;

                for clip in clips.iter() {
                    if origin.y >= clip.origin.y && origin.y <= clip.max_y() {
                        // line contains
                        if origin.x < clip.max_x() && line_max > clip.origin.x {
                            // intersects
                            include_buf.push((clip.origin.x, clip.max_x()));
                        }
                    }
                }

                if !include_buf.is_empty() {
                    include_buf.sort_by_key(|(s, _)| *s);

                    for (clip_start, clip_end) in include_buf.drain(..) {
                        let start = clip_start.max(origin.x);
                        let end = clip_end.min(line_max);

                        out_lines.push((PxPoint::new(start, origin.y), end - start));
                    }

                    include_buf.clear();
                }
            }
        }

        out_lines
    }
}

struct ImageGlyphsIter<'a, G>
where
    G: Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a,
{
    glyphs: G,
    glyphs_i: u32,
    images: &'a [(u32, GlyphImage)],
    maybe_img: Option<(&'a Font, &'a [GlyphInstance])>,
}
impl<'a, G> Iterator for ImageGlyphsIter<'a, G>
where
    G: Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a,
{
    type Item = (&'a Font, ShapedImageGlyphs<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((font, glyphs)) = &mut self.maybe_img {
                // new glyph sequence or single emoji(maybe img)

                // advance images to the next in or after glyph sequence
                while self.images.first().map(|(i, _)| *i < self.glyphs_i).unwrap_or(false) {
                    self.images = &self.images[1..];
                }

                if let Some((i, img)) = self.images.first() {
                    // if there is still images
                    if *i == self.glyphs_i {
                        // if the next glyph is replaced by image
                        self.glyphs_i += 1;
                        let mut size = img.0.with(|i| i.size()).cast::<f32>();
                        let scale = font.size().0 as f32 / size.width.max(size.height);
                        size *= scale;
                        let r = (
                            *font,
                            ShapedImageGlyphs::Image {
                                rect: euclid::Rect::new(glyphs[0].point - euclid::vec2(0.0, size.height), size),
                                base_glyph: glyphs[0].index,
                                img: &img.0,
                            },
                        );
                        *glyphs = &glyphs[1..];
                        if glyphs.is_empty() {
                            self.maybe_img = None;
                        }
                        return Some(r);
                    } else {
                        // if the next glyph is not replaced by image, yield slice to end or next image
                        let normal = &glyphs[..glyphs.len().min(*i as _)];
                        self.glyphs_i += normal.len() as u32;

                        *glyphs = &glyphs[normal.len()..];
                        let r = (*font, ShapedImageGlyphs::Normal(normal));

                        if glyphs.is_empty() {
                            self.maybe_img = None;
                        }
                        return Some(r);
                    }
                } else {
                    // if there are no more images
                    let r = (*font, ShapedImageGlyphs::Normal(glyphs));
                    self.maybe_img = None;
                    return Some(r);
                }
            } else if let Some(seq) = self.glyphs.next() {
                // all sequences can contain images
                self.maybe_img = Some(seq);
            } else {
                // no more glyphs to yield
                return None;
            }
        }
    }
}

struct ColoredGlyphsIter<'a, G>
where
    G: Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a,
{
    glyphs: G,
    maybe_colored: Option<(&'a Font, &'a [GlyphInstance])>,
}
impl<'a, G> Iterator for ColoredGlyphsIter<'a, G>
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

/// Info about a shaped text overflow in constraint.
///
/// Can be computed using [`ShapedText::overflow_info`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextOverflowInfo {
    /// First overflow line.
    ///
    /// All segments in this line and next lines are fully overflown. The previous line may
    /// be partially overflown, the lines before that are fully visible.
    ///
    /// Is the [`ShapedText::lines_len`] if the last line is fully visible.
    pub line: usize,

    /// First overflow character in the text.
    ///
    /// Note that if overflow is not wrapping (single text line) the char may not cover all visible
    /// glyphs in the line if it is bidirectional.
    pub text_char: usize,

    /// Glyphs not overflown in the last not overflown line.
    ///
    /// If the line is not bidirectional this will be a single range covering the not overflow glyphs,
    /// if it is bidi multiple ranges are possible due to bidi reordering.
    pub included_glyphs: smallvec::SmallVec<[ops::Range<usize>; 1]>,

    /// Placement of the suffix (ellipses or custom).
    ///
    /// The suffix must be of the width given to [`ShapedText::overflow_info`] and the same line height
    /// as the text.
    pub suffix_origin: euclid::Point2D<f32, Px>,
}

trait FontListRef {
    /// Shape segment, try fallback fonts if a glyph in the segment is not resolved.
    fn shape_segment<R>(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        features: &[rustybuzz::Feature],
        out: impl FnOnce(&ShapedSegmentData, &Font) -> R,
    ) -> R;
}
impl FontListRef for [Font] {
    fn shape_segment<R>(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        features: &[rustybuzz::Feature],
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
    lang: Lang,

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
                justify: Justify::Auto,
                justified: vec![],
                overflow_align: Align::TOP_LEFT,
                direction: LayoutDirection::LTR,
                first_wrapped: false,
                is_inlined: config.inline_constraints.is_some(),
                first_line: PxRect::zero(),
                mid_clear: Px(0),
                mid_size: PxSize::zero(),
                last_line: PxRect::zero(),
                has_colored_glyphs: false,
                images: vec![],
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
            lang: config.lang.clone(),
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

        if !matches!(config.hyphens, Hyphens::None) && t.max_width.is_finite() && config.obscuring_char.is_none() {
            // "hyphen" can be any char and we need the x-advance for the wrap algorithm.
            t.hyphen_glyphs = fonts.shape_segment(config.hyphen_char.as_str(), &word_ctx_key, &config.font_features, |s, f| {
                (s.clone(), f.clone())
            });
        }

        if let Some(c) = config.obscuring_char {
            t.push_obscured_text(fonts, &config.font_features, &mut word_ctx_key, text, c);
        } else {
            t.push_text(fonts, &config.font_features, &mut word_ctx_key, text);
        }

        t.out.glyphs.shrink_to_fit();
        t.out.clusters.shrink_to_fit();
        t.out.segments.0.shrink_to_fit();
        t.out.lines.0.shrink_to_fit();
        t.out.fonts.0.shrink_to_fit();
        t.out.images.shrink_to_fit();

        t.out.debug_assert_ranges();
        t.out
    }

    fn push_obscured_text(
        &mut self,
        fonts: &[Font],
        features: &RFontFeatures,
        word_ctx_key: &mut WordContextKey,
        text: &SegmentedText,
        obscuring_char: char,
    ) {
        if text.is_empty() {
            self.push_last_line(text);
            self.push_font(&fonts[0]);
            return;
        }

        let (glyphs, font) = fonts.shape_segment(Txt::from_char(obscuring_char).as_str(), word_ctx_key, features, |s, f| {
            (s.clone(), f.clone())
        });

        for (seg, info) in text.iter() {
            let mut seg_glyphs = ShapedSegmentData::default();
            for (cluster, _) in seg.char_indices() {
                let i = seg_glyphs.glyphs.len();
                seg_glyphs.glyphs.extend(glyphs.glyphs.iter().copied());
                for g in &mut seg_glyphs.glyphs[i..] {
                    g.point.0 += seg_glyphs.x_advance;
                    g.cluster = cluster as u32;
                }
                seg_glyphs.x_advance += glyphs.x_advance;
            }
            self.push_glyphs(&seg_glyphs, self.letter_spacing);
            self.push_text_seg(seg, info);
        }

        self.push_last_line(text);

        self.push_font(&font);
    }

    fn push_text(&mut self, fonts: &[Font], features: &RFontFeatures, word_ctx_key: &mut WordContextKey, text: &SegmentedText) {
        static LIG: [&[u8]; 4] = [b"liga", b"clig", b"dlig", b"hlig"];
        let ligature_enabled = fonts[0].face().has_ligatures()
            && features.iter().any(|f| {
                let tag = f.tag.to_bytes();
                LIG.iter().any(|l| *l == tag)
            });

        if ligature_enabled {
            let mut start = 0;
            let mut words_start = None;
            for (i, info) in text.segs().iter().enumerate() {
                if info.kind.is_word() && info.kind != TextSegmentKind::Emoji {
                    if words_start.is_none() {
                        words_start = Some(i);
                    }
                } else {
                    if let Some(s) = words_start.take() {
                        self.push_ligature_words(fonts, features, word_ctx_key, text, s, i);
                    }

                    let seg = &text.text()[start..info.end];
                    self.push_seg(fonts, features, word_ctx_key, text, seg, *info);
                }
                start = info.end;
            }
            if let Some(s) = words_start.take() {
                self.push_ligature_words(fonts, features, word_ctx_key, text, s, text.segs().len());
            }
        } else {
            for (seg, info) in text.iter() {
                self.push_seg(fonts, features, word_ctx_key, text, seg, info);
            }
        }

        self.push_last_line(text);

        self.push_font(&fonts[0]);
    }
    fn push_ligature_words(
        &mut self,
        fonts: &[Font],
        features: &RFontFeatures,
        word_ctx_key: &mut WordContextKey,
        text: &SegmentedText,
        words_start: usize,
        words_end: usize,
    ) {
        let seg_start = if words_start == 0 { 0 } else { text.segs()[words_start - 1].end };
        let end_info = text.segs()[words_end - 1];
        let seg_end = end_info.end;
        let seg = &text.text()[seg_start..seg_end];

        if words_end - words_start == 1 {
            self.push_seg(fonts, features, word_ctx_key, text, seg, end_info);
        } else {
            // check if `is_word` sequence is a ligature that covers more than one word.
            let handled = fonts[0].shape_segment(seg, word_ctx_key, features, |shaped_seg| {
                let mut cluster_start = 0;
                let mut cluster_end = None;
                for g in shaped_seg.glyphs.iter() {
                    if g.index == 0 {
                        // top font not used for at least one word in this sequence
                        return false;
                    }
                    if seg[cluster_start as usize..g.cluster as usize].chars().take(2).count() > 1 {
                        cluster_end = Some(g.index);
                        break;
                    }
                    cluster_start = g.cluster;
                }

                if cluster_end.is_none() && seg[cluster_start as usize..].chars().take(2).count() > 1 {
                    cluster_end = Some(seg.len() as u32);
                }
                if let Some(cluster_end) = cluster_end {
                    // previous glyph is a ligature, check word boundaries.
                    let cluster_start_in_txt = seg_start + cluster_start as usize;
                    let cluster_end_in_txt = seg_start + cluster_end as usize;

                    let handle = text.segs()[words_start..words_end]
                        .iter()
                        .any(|info| info.end > cluster_start_in_txt && info.end <= cluster_end_in_txt);

                    if handle {
                        let max_width = self.actual_max_width();
                        if self.origin.x + shaped_seg.x_advance > max_width {
                            // need wrap
                            if shaped_seg.x_advance > max_width {
                                // need segment split
                                return false;
                            }

                            self.push_line_break(true, text);
                            self.push_glyphs(shaped_seg, self.letter_spacing);
                        }
                        self.push_glyphs(shaped_seg, self.letter_spacing);
                        let mut seg = seg;
                        for info in &text.segs()[words_start..words_end] {
                            self.push_text_seg(seg, *info);
                            seg = "";
                        }

                        return true;
                    }
                }

                false
            });

            if !handled {
                let mut seg_start = seg_start;
                for info in text.segs()[words_start..words_end].iter() {
                    let seg = &text.text()[seg_start..info.end];
                    self.push_seg(fonts, features, word_ctx_key, text, seg, *info);
                    seg_start = info.end;
                }
            }
        }
    }
    fn push_seg(
        &mut self,
        fonts: &[Font],
        features: &RFontFeatures,
        word_ctx_key: &mut WordContextKey,
        text: &SegmentedText,
        seg: &str,
        info: TextSegment,
    ) {
        word_ctx_key.direction = info.direction();
        if info.kind.is_word() {
            let max_width = self.actual_max_width();

            fonts.shape_segment(seg, word_ctx_key, features, |shaped_seg, font| {
                if self.origin.x + shaped_seg.x_advance > max_width {
                    // need wrap
                    if shaped_seg.x_advance > max_width {
                        // need segment split

                        // try to hyphenate
                        let hyphenated = self.push_hyphenate(seg, font, shaped_seg, info, text);

                        if !hyphenated && self.break_words {
                            // break word
                            self.push_split_seg(shaped_seg, seg, info, self.letter_spacing, text);
                        } else if !hyphenated {
                            let current_start = if self.out.lines.0.is_empty() {
                                0
                            } else {
                                self.out.lines.last().end
                            };
                            if !self.out.segments.0[current_start..].is_empty() {
                                self.push_line_break(true, text);
                            } else if current_start == 0 && self.allow_first_wrap {
                                self.out.first_wrapped = true;
                            }
                            self.push_glyphs(shaped_seg, self.letter_spacing);
                            self.push_text_seg(seg, info);
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

                if matches!(info.kind, TextSegmentKind::Emoji) {
                    if !font.face().color_glyphs().is_empty() {
                        self.out.has_colored_glyphs = true;
                    }
                    if font.face().has_raster_images() || (cfg!(feature = "svg") && font.face().has_svg_images()) {
                        if let Some(ttf) = font.face().ttf() {
                            for (i, g) in shaped_seg.glyphs.iter().enumerate() {
                                let id = ttf_parser::GlyphId(g.index as _);
                                let ppm = font.size().0 as u16;
                                let glyphs_i = self.out.glyphs.len() - shaped_seg.glyphs.len() + i;
                                if let Some(img) = ttf.glyph_raster_image(id, ppm) {
                                    self.push_glyph_raster(glyphs_i as _, img);
                                } else if cfg!(feature = "svg") {
                                    if let Some(img) = ttf.glyph_svg_image(id) {
                                        self.push_glyph_svg(glyphs_i as _, img);
                                    }
                                }
                            }
                        }
                    }
                }

                self.push_font(font);
            });
        } else if info.kind.is_space() {
            if matches!(info.kind, TextSegmentKind::Tab) {
                let max_width = self.actual_max_width();
                for (i, _) in seg.char_indices() {
                    if self.origin.x + self.tab_x_advance > max_width {
                        // normal wrap, advance overflow
                        self.push_line_break(true, text);
                    }
                    let point = euclid::point2(self.origin.x, self.origin.y);
                    self.origin.x += self.tab_x_advance;
                    self.out.glyphs.push(GlyphInstance {
                        index: self.tab_index,
                        point,
                    });
                    self.out.clusters.push(i as u32);
                }

                self.push_text_seg(seg, info);
                self.push_font(&fonts[0]);
            } else {
                let max_width = self.actual_max_width();
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

    fn push_glyph_raster(&mut self, glyphs_i: u32, img: ttf_parser::RasterGlyphImage) {
        use ttf_parser::RasterImageFormat;
        let size = PxSize::new(Px(img.width as _), Px(img.height as _));
        let bgra_fmt = ImageDataFormat::Bgra8 { size, ppi: None };
        let bgra_len = img.width as usize * img.height as usize * 4;
        let (data, fmt) = match img.format {
            RasterImageFormat::PNG => (img.data.to_vec(), ImageDataFormat::from("png")),
            RasterImageFormat::BitmapMono => {
                // row aligned 1-bitmap
                let mut bgra = Vec::with_capacity(bgra_len);
                let bytes_per_row = (img.width as usize + 7) / 8;
                for y in 0..img.height as usize {
                    let row_start = y * bytes_per_row;
                    for x in 0..img.width as usize {
                        let byte_index = row_start + x / 8;
                        let bit_index = 7 - (x % 8);
                        let bit = (img.data[byte_index] >> bit_index) & 1;
                        let color = if bit == 1 { [0, 0, 0, 255] } else { [255, 255, 255, 255] };
                        bgra.extend_from_slice(&color);
                    }
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapMonoPacked => {
                // packed 1-bitmap
                let mut bgra = Vec::with_capacity(bgra_len);
                for &c8 in img.data {
                    for bit in 0..8 {
                        let color = if (c8 >> (7 - bit)) & 1 == 1 {
                            [0, 0, 0, 255]
                        } else {
                            [255, 255, 255, 255]
                        };
                        bgra.extend_from_slice(&color);
                        if bgra.len() == bgra_len {
                            break;
                        }
                    }
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapGray2 => {
                // row aligned 2-bitmap
                let mut bgra = Vec::with_capacity(bgra_len);
                let bytes_per_row = (img.width as usize + 3) / 4;
                for y in 0..img.height as usize {
                    let row_start = y * bytes_per_row;
                    for x in 0..img.width as usize {
                        let byte_index = row_start + x / 4;
                        let shift = (3 - (x % 4)) * 2;
                        let gray = (img.data[byte_index] >> shift) & 0b11;
                        let color = match gray {
                            0b00 => [0, 0, 0, 255],       // Black
                            0b01 => [85, 85, 85, 255],    // Dark gray
                            0b10 => [170, 170, 170, 255], // Light gray
                            0b11 => [255, 255, 255, 255], // White
                            _ => unreachable!(),
                        };
                        bgra.extend_from_slice(&color);
                    }
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapGray2Packed => {
                // packed 2-bitmap
                let mut bgra = Vec::with_capacity(bgra_len);
                for &c4 in img.data {
                    for i in 0..4 {
                        let gray = (c4 >> (7 - i * 2)) & 0b11;
                        let color = match gray {
                            0b00 => [0, 0, 0, 255],       // Black
                            0b01 => [85, 85, 85, 255],    // Dark gray
                            0b10 => [170, 170, 170, 255], // Light gray
                            0b11 => [255, 255, 255, 255], // White
                            _ => unreachable!(),
                        };
                        bgra.extend_from_slice(&color);
                        if bgra.len() == bgra_len {
                            break;
                        }
                    }
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapGray4 => {
                // row aligned 4-bitmap
                let mut bgra = Vec::with_capacity(bgra_len);
                let bytes_per_row = (img.width as usize + 1) / 2;
                for y in 0..img.height as usize {
                    let row_start = y * bytes_per_row;
                    for x in 0..img.width as usize {
                        let byte_index = row_start + x / 2;
                        let shift = if x % 2 == 0 { 4 } else { 0 };
                        let gray = (img.data[byte_index] >> shift) & 0b1111;
                        let g = gray * 17;
                        bgra.extend_from_slice(&[g, g, g, 255]);
                    }
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapGray4Packed => {
                let mut bgra = Vec::with_capacity(bgra_len);
                for &c2 in img.data {
                    for i in 0..2 {
                        let gray = (c2 >> (7 - i * 4)) & 0b1111;
                        let g = gray * 17;
                        bgra.extend_from_slice(&[g, g, g, 255]);
                        if bgra.len() == bgra_len {
                            break;
                        }
                    }
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapGray8 => {
                let mut bgra = Vec::with_capacity(bgra_len);
                for &c in img.data {
                    bgra.extend_from_slice(&[c, c, c, 255]);
                }
                (bgra, bgra_fmt)
            }
            RasterImageFormat::BitmapPremulBgra32 => {
                let mut bgra = img.data.to_vec();
                for c in bgra.chunks_exact_mut(4) {
                    let (b, g, r, a) = (c[0], c[1], c[2], c[3]);
                    let unp = if a == 255 {
                        [b, g, r]
                    } else {
                        [
                            (b as u32 * 255 / a as u32) as u8,
                            (g as u32 * 255 / a as u32) as u8,
                            (r as u32 * 255 / a as u32) as u8,
                        ]
                    };
                    c.copy_from_slice(&unp);
                }
                (bgra, bgra_fmt)
            }
        };
        self.push_glyph_img(glyphs_i, ImageSource::from_data(Arc::new(data), fmt));
    }

    fn push_glyph_svg(&mut self, glyphs_i: u32, img: ttf_parser::svg::SvgDocument) {
        self.push_glyph_img(
            glyphs_i,
            ImageSource::from_data(Arc::new(img.data.to_vec()), ImageDataFormat::from("svg")),
        );
    }

    fn push_glyph_img(&mut self, glyphs_i: u32, source: ImageSource) {
        let img = IMAGES.cache(source);

        self.out.images.push((glyphs_i, GlyphImage(img)));
    }

    fn push_last_line(&mut self, text: &SegmentedText) {
        let directions = self.finish_current_line_bidi(text);
        self.out.lines.0.push(LineRange {
            end: self.out.segments.0.len(),
            width: self.origin.x,
            x_offset: 0.0,
            directions,
        });

        self.out.update_mid_size();
        self.out.update_first_last_lines();
        self.out.orig_first_line = self.out.first_line.size;
        self.out.orig_last_line = self.out.last_line.size;
        if self.out.is_inlined && self.out.lines.0.len() > 1 {
            self.out.last_line.origin.y += self.out.mid_clear;
        }
    }

    fn push_hyphenate(&mut self, seg: &str, font: &Font, shaped_seg: &ShapedSegmentData, info: TextSegment, text: &SegmentedText) -> bool {
        if !matches!(self.hyphens, Hyphens::Auto) {
            return false;
        }

        let split_points = HYPHENATION.hyphenate(&self.lang, seg);
        self.push_hyphenate_pt(&split_points, 0, font, shaped_seg, seg, info, text)
    }

    #[allow(clippy::too_many_arguments)]
    fn push_hyphenate_pt(
        &mut self,
        split_points: &[usize],
        split_points_sub: usize,
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
            let mut point = *point - split_points_sub;
            let mut width = 0.0;
            let mut c = u32::MAX;
            let mut gi = 0;
            // find the first glyph in the cluster at the char byte index `point`
            for (i, g) in shaped_seg.glyphs.iter().enumerate() {
                width = g.point.0;
                if g.cluster != c {
                    // advanced cluster, advance point
                    if point == 0 {
                        break;
                    }
                    c = g.cluster;
                    point -= 1;
                }
                gi = i;
            }

            if self.origin.x + width + self.hyphen_glyphs.0.x_advance > max_width {
                // fragment+hyphen is to large
                if end_glyph == 0 {
                    // found no candidate, there is no way to avoid overflow, use smallest option
                    end_glyph = gi + 1;
                    end_point_i = i + 1;
                }
                break;
            } else {
                // found candidate fragment
                end_glyph = gi + 1;
                end_point_i = i + 1;
            }
        }

        // split and push the first half + hyphen
        let end_glyph_x = shaped_seg.glyphs[end_glyph].point.0;
        let (glyphs_a, glyphs_b) = shaped_seg.glyphs.split_at(end_glyph);

        if glyphs_a.is_empty() || glyphs_b.is_empty() {
            debug_assert!(false, "invalid hyphenation split");
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
            if self.push_hyphenate_pt(
                &split_points[end_point_i..],
                split_points_sub + seg_a.len(),
                font,
                &shaped_seg_b,
                seg_b,
                info,
                text,
            ) {
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
            let directions = self.finish_current_line_bidi(text);

            self.out.lines.0.push(LineRange {
                end: self.out.segments.0.len(),
                width: self.origin.x,
                x_offset: 0.0,
                directions,
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

    #[must_use]
    fn finish_current_line_bidi(&mut self, text: &SegmentedText) -> LayoutDirections {
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

        let mut d = LayoutDirections::empty();
        d.set(LayoutDirections::LTR, self.line_has_ltr);
        d.set(LayoutDirections::RTL, self.line_has_rtl);

        self.line_has_ltr = false;
        self.line_has_rtl = false;

        d
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

bitflags! {
    /// Identifies what direction segments a [`ShapedLine`] has.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct LayoutDirections: u8 {
        /// Line has left-to-right segments.
        const LTR = 1;
        /// Line has right-to-left segments.
        const RTL = 2;
        /// Line as both left-to-right and right-to-left segments.
        ///
        /// When this is the case the line segments positions may be re-ordered.
        const BIDI = Self::LTR.bits() | Self::RTL.bits();
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
impl fmt::Debug for ShapedLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapedLine")
            .field("seg_range", &self.seg_range)
            .field("index", &self.index)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}
impl<'a> ShapedLine<'a> {
    /// Height of the line.
    pub fn height(&self) -> Px {
        if self.index == 0 {
            self.text.first_line.height()
        } else if self.index == self.text.lines.0.len() - 1 {
            self.text.last_line.height()
        } else {
            self.text.line_height
        }
    }

    /// Width of the line.
    pub fn width(&self) -> Px {
        if self.index == 0 {
            self.text.first_line.width()
        } else if self.index == self.text.lines.0.len() - 1 {
            self.text.last_line.width()
        } else {
            self.width
        }
    }

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
    ///
    /// The glyphs are in text order by segments and in visual order (LTR) within segments, so
    /// the RTL text "لما " will have the space glyph first, then "’álif", "miim", "láam".
    ///
    /// All glyph points are set as offsets to the top-left of the text full text.
    pub fn glyphs(&self) -> impl Iterator<Item = (&'a Font, &'a [GlyphInstance])> + 'a {
        let r = self.glyphs_range();
        self.text.glyphs_range(r)
    }

    /// Glyphs in the line paired with the *x-advance*.
    pub fn glyphs_with_x_advance(&self) -> impl Iterator<Item = (&'a Font, impl Iterator<Item = (GlyphInstance, f32)> + 'a)> + 'a {
        self.segs().flat_map(|s| s.glyphs_with_x_advance())
    }

    fn glyphs_range(&self) -> IndexRange {
        self.text.segments.glyphs_range(self.seg_range)
    }

    /// Iterate over word and space segments in this line.
    pub fn segs(&self) -> impl DoubleEndedIterator<Item = ShapedSegment<'a>> + ExactSizeIterator + use<'a> {
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
                .unwrap() // only last line can be empty
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
                .unwrap() // only last line can be empty
    }

    /// Returns the line or first previous line that is not [`started_by_wrap`].
    ///
    /// [`started_by_wrap`]: Self::started_by_wrap
    pub fn actual_line_start(&self) -> Self {
        let mut r = *self;
        while r.started_by_wrap() {
            r = r.text.line(r.index - 1).unwrap();
        }
        r
    }

    /// Returns the line or first next line that is not [`ended_by_wrap`].
    ///
    /// [`ended_by_wrap`]: Self::ended_by_wrap
    pub fn actual_line_end(&self) -> Self {
        let mut r = *self;
        while r.ended_by_wrap() {
            r = r.text.line(r.index + 1).unwrap();
        }
        r
    }

    /// Get the text bytes range of this line in the original text.
    pub fn text_range(&self) -> ops::Range<usize> {
        let start = self.seg_range.start();
        let start = if start == 0 { 0 } else { self.text.segments.0[start - 1].text.end };
        let end = self.seg_range.end();
        let end = if end == 0 { 0 } else { self.text.segments.0[end - 1].text.end };

        start..end
    }

    /// Get the text bytes range of this line in the original text, excluding the line break
    /// to keep [`end`] in the same line.
    ///
    /// [`end`]: ops::Range<usize>::end
    pub fn text_caret_range(&self) -> ops::Range<usize> {
        let start = self.seg_range.start();
        let start = if start == 0 { 0 } else { self.text.segments.0[start - 1].text.end };
        let end = self.seg_range.end();
        let end = if end == 0 {
            0
        } else if self.seg_range.start() == end {
            start
        } else {
            let seg = &self.text.segments.0[end - 1];
            if !matches!(seg.text.kind, TextSegmentKind::LineBreak) {
                seg.text.end
            } else {
                // start of LineBreak segment
                if end == 1 { 0 } else { self.text.segments.0[end - 2].text.end }
            }
        };

        start..end
    }

    /// Gets the text range of the actual line, joining shaped lines that are started by wrap.
    pub fn actual_text_range(&self) -> ops::Range<usize> {
        let start = self.actual_line_start().text_range().start;
        let end = self.actual_line_end().text_range().end;
        start..end
    }

    /// Gets the text range of the actual line, excluding the line break at the end.
    pub fn actual_text_caret_range(&self) -> ops::Range<usize> {
        let start = self.actual_line_start().text_range().start;
        let end = self.actual_line_end().text_caret_range().end;
        start..end
    }

    /// Select the string represented by this line.
    ///
    /// The `full_text` must be equal to the original text that was used to generate the parent [`ShapedText`].
    pub fn text<'s>(&self, full_text: &'s str) -> &'s str {
        let r = self.text_range();

        let start = r.start.min(full_text.len());
        let end = r.end.min(full_text.len());

        &full_text[start..end]
    }

    /// Gets the segment that contains `x` or is nearest to it.
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

    /// Gets the line index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Layout directions of segments in this line.
    pub fn directions(&self) -> LayoutDirections {
        self.text.lines.0[self.index].directions
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
                Some(line) => {
                    if let Some(prev_line) = &mut self.line {
                        fn min_x((origin, _width): (PxPoint, Px)) -> Px {
                            origin.x
                        }
                        fn max_x((origin, width): (PxPoint, Px)) -> Px {
                            origin.x + width
                        }

                        if prev_line.0.y == line.0.y && min_x(*prev_line) <= max_x(line) && max_x(*prev_line) >= min_x(line) {
                            let x = min_x(*prev_line).min(min_x(line));
                            prev_line.1 = max_x(*prev_line).max(max_x(line)) - x;
                            prev_line.0.x = x;
                        } else {
                            let cut = mem::replace(prev_line, line);
                            return Some(cut);
                        }
                    } else {
                        self.line = Some(line);
                        continue;
                    }
                }
                None => return self.line.take(),
            }
        }
    }
}

struct MergingRectIter<I> {
    iter: I,
    rect: Option<PxBox>,
}
impl<I> MergingRectIter<I> {
    pub fn new(iter: I) -> Self {
        MergingRectIter { iter, rect: None }
    }
}
impl<I: Iterator<Item = PxRect>> Iterator for MergingRectIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(rect) => {
                    let rect = rect.to_box2d();
                    if let Some(prev_rect) = &mut self.rect {
                        if prev_rect.min.y == rect.min.y
                            && prev_rect.max.y == rect.max.y
                            && prev_rect.min.x <= rect.max.x
                            && prev_rect.max.x >= rect.min.x
                        {
                            prev_rect.min.x = prev_rect.min.x.min(rect.min.x);
                            prev_rect.max.x = prev_rect.max.x.max(rect.max.x);
                            continue;
                        } else {
                            let cut = mem::replace(prev_rect, rect);
                            return Some(cut.to_rect());
                        }
                    } else {
                        self.rect = Some(rect);
                        continue;
                    }
                }
                None => return self.rect.take().map(|r| r.to_rect()),
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
impl fmt::Debug for ShapedSegment<'_> {
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
    ///
    /// The glyphs are in visual order (LTR) within segments, so
    /// the RTL text "لما" will yield "’álif", "miim", "láam".
    ///
    /// All glyph points are set as offsets to the top-left of the text full text.
    ///
    /// Note that multiple glyphs can map to the same char and multiple chars can map to the same glyph, you can use the [`clusters`]
    /// map to find the char for each glyph. Some font ligatures also bridge multiple segments, in this case only the first shaped
    /// segment has glyphs the subsequent ones are empty.
    ///
    /// [`clusters`]: Self::clusters
    pub fn glyphs(&self) -> impl Iterator<Item = (&'a Font, &'a [GlyphInstance])> {
        let r = self.glyphs_range();
        self.text.glyphs_range(r)
    }

    /// Gets the specific glyph and font.
    pub fn glyph(&self, index: usize) -> Option<(&'a Font, GlyphInstance)> {
        let mut r = self.glyphs_range();
        r.0 += index;
        self.text.glyphs_range(r).next().map(|(f, g)| (f, g[0]))
    }

    /// Map glyph -> char.
    ///
    /// Each [`glyphs`] glyph pairs with an entry in this slice that is the char byte index in [`text`]. If
    /// a font ligature bridges multiple segments only the first segment will have a non-empty map.
    ///
    /// [`glyphs`]: Self::glyphs
    /// [`text`]: Self::text
    pub fn clusters(&self) -> &[u32] {
        let r = self.glyphs_range();
        self.text.clusters_range(r)
    }

    /// Count the deduplicated [`clusters`].
    ///
    /// [`clusters`]: Self::clusters
    pub fn clusters_count(&self) -> usize {
        let mut c = u32::MAX;
        let mut count = 0;
        for &i in self.clusters() {
            if i != c {
                c = i;
                count += 1;
            }
        }
        count
    }

    /// Number of next segments that are empty because their text is included in a ligature
    /// glyph or glyphs started in this segment.
    pub fn ligature_segs_count(&self) -> usize {
        let range = self.glyphs_range();
        if range.iter().is_empty() {
            0
        } else {
            self.text.segments.0[self.index + 1..]
                .iter()
                .filter(|s| s.end == range.end())
                .count()
        }
    }

    /// Glyphs in the segment, paired with the *x-advance*.
    ///
    /// Yields `(Font, [(glyph, advance)])`.
    pub fn glyphs_with_x_advance(
        &self,
    ) -> impl Iterator<Item = (&'a Font, impl Iterator<Item = (GlyphInstance, f32)> + use<'a>)> + use<'a> {
        let r = self.glyphs_range();
        self.text.seg_glyphs_with_x_advance(self.index, r)
    }

    /// Glyphs per cluster in the segment, paired with the *x-advance* of the cluster.
    ///
    /// Yields `(Font, [(cluster, [glyph], advance)])`.
    pub fn cluster_glyphs_with_x_advance(
        &self,
    ) -> impl Iterator<Item = (&'a Font, impl Iterator<Item = (u32, &'a [GlyphInstance], f32)> + use<'a>)> + use<'a> {
        let r = self.glyphs_range();
        self.text.seg_cluster_glyphs_with_x_advance(self.index, r)
    }

    /// Gets the segment x offset and advance.
    pub fn x_width(&self) -> (Px, Px) {
        let IndexRange(start, end) = self.glyphs_range();

        let is_line_break = start == end && matches!(self.kind(), TextSegmentKind::LineBreak);

        let start_x = match self.direction() {
            LayoutDirection::LTR => {
                if is_line_break || start == self.text.glyphs.len() {
                    let x = self.text.lines.x_offset(self.line_index);
                    let w = self.text.lines.width(self.line_index);
                    return (Px((x + w) as i32), Px(0));
                }
                self.text.glyphs[start].point.x
            }
            LayoutDirection::RTL => {
                if is_line_break || start == self.text.glyphs.len() {
                    let x = self.text.lines.x_offset(self.line_index);
                    return (Px(x as i32), Px(0));
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

    /// Gets the first char and glyph with advance that overflows `max_width`.
    pub fn overflow_char_glyph(&self, max_width_px: f32) -> Option<(usize, usize)> {
        if self.advance() > max_width_px {
            match self.direction() {
                LayoutDirection::LTR => {
                    let mut x = 0.0;
                    let mut g = 0;
                    for (_, c) in self.cluster_glyphs_with_x_advance() {
                        for (cluster, glyphs, advance) in c {
                            x += advance;
                            if x > max_width_px {
                                return Some((cluster as usize, g));
                            }
                            g += glyphs.len();
                        }
                    }
                }
                LayoutDirection::RTL => {
                    let mut g = 0;
                    let mut rev = smallvec::SmallVec::<[_; 10]>::new();
                    for (_, c) in self.cluster_glyphs_with_x_advance() {
                        for (cluster, glyphs, advance) in c {
                            rev.push((cluster, g, advance));
                            g += glyphs.len();
                        }
                    }

                    let mut x = 0.0;
                    for (c, g, advance) in rev.into_iter().rev() {
                        x += advance;
                        if x > max_width_px {
                            return Some((c as usize, g));
                        }
                    }
                }
            }
        }
        None
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
    pub fn underline_skip_glyphs(&self, thickness: Px) -> impl Iterator<Item = (PxPoint, Px)> + use<'a> {
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
        impl<I, J> UnderlineSkipGlyphs<'_, I, J> {
            fn line(&self) -> Option<(PxPoint, Px)> {
                fn f32_to_px(px: f32) -> Px {
                    Px(px.round() as i32)
                }
                let r = (PxPoint::new(f32_to_px(self.x), self.y), f32_to_px(self.width));
                if r.1 >= self.min_width { Some(r) } else { None }
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
                            if let Some((ex_start, ex_end)) = font.h_line_hits(g.index, self.line_y_range) {
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
    pub fn text_range(&self) -> ops::Range<usize> {
        self.text_start()..self.text_end()
    }

    /// Get the text byte range start of this segment in the original text.
    pub fn text_start(&self) -> usize {
        if self.index == 0 {
            0
        } else {
            self.text.segments.0[self.index - 1].text.end
        }
    }

    /// Get the text byte range end of this segment in the original text.
    pub fn text_end(&self) -> usize {
        self.text.segments.0[self.index].text.end
    }

    /// Get the text bytes range of the `glyph_range` in this segment's [`text`].
    ///
    /// [`text`]: Self::text
    pub fn text_glyph_range(&self, glyph_range: impl ops::RangeBounds<usize>) -> ops::Range<usize> {
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
        .iter()
    }

    /// Select the string represented by this segment.
    ///
    /// The `full_text` must be equal to the original text that was used to generate the parent [`ShapedText`].
    pub fn text<'s>(&self, full_text: &'s str) -> &'s str {
        let r = self.text_range();
        let start = r.start.min(full_text.len());
        let end = r.end.min(full_text.len());
        &full_text[start..end]
    }

    /// Gets the insert index in the segment text that is nearest to `x`.
    pub fn nearest_char_index(&self, x: Px, full_text: &str) -> usize {
        let txt_range = self.text_range();
        let is_rtl = self.direction().is_rtl();
        let x = x.0 as f32;

        let seg_clusters = self.clusters();

        for (font, clusters) in self.cluster_glyphs_with_x_advance() {
            for (cluster, glyphs, advance) in clusters {
                let found = x < glyphs[0].point.x || glyphs[0].point.x + advance > x;
                if !found {
                    continue;
                }
                let cluster_i = seg_clusters.iter().position(|&c| c == cluster).unwrap();

                let char_a = txt_range.start + cluster as usize;
                let char_b = if is_rtl {
                    if cluster_i == 0 {
                        txt_range.end
                    } else {
                        txt_range.start + seg_clusters[cluster_i - 1] as usize
                    }
                } else {
                    let next_cluster = cluster_i + glyphs.len();
                    if next_cluster == seg_clusters.len() {
                        txt_range.end
                    } else {
                        txt_range.start + seg_clusters[next_cluster] as usize
                    }
                };

                if char_b - char_a > 1 && glyphs.len() == 1 {
                    // maybe ligature

                    let text = &full_text[char_a..char_b];

                    let mut lig_parts = smallvec::SmallVec::<[u16; 6]>::new_const();
                    for (i, _) in unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true) {
                        lig_parts.push(i as u16);
                    }

                    if lig_parts.len() > 1 {
                        // is ligature

                        let x = x - glyphs[0].point.x;

                        let mut split = true;
                        for (i, font_caret) in font.ligature_caret_offsets(glyphs[0].index).enumerate() {
                            if i == lig_parts.len() {
                                break;
                            }
                            split = false;

                            if font_caret > x {
                                // found font defined caret
                                return char_a + lig_parts[i] as usize;
                            }
                        }
                        if split {
                            // no font caret, ligature glyph is split in equal parts
                            let lig_part = advance / lig_parts.len() as f32;
                            let mut lig_x = lig_part;
                            if is_rtl {
                                for c in lig_parts.into_iter().rev() {
                                    if lig_x > x {
                                        // fond
                                        return char_a + c as usize;
                                    }
                                    lig_x += lig_part;
                                }
                            } else {
                                for c in lig_parts {
                                    if lig_x > x {
                                        return char_a + c as usize;
                                    }
                                    lig_x += lig_part;
                                }
                            }
                        }
                    }
                }
                // not ligature

                let middle_x = glyphs[0].point.x + advance / 2.0;

                return if is_rtl {
                    if x <= middle_x { char_b } else { char_a }
                } else if x <= middle_x {
                    char_a
                } else {
                    char_b
                };
            }
        }

        let mut start = is_rtl;
        if matches!(self.kind(), TextSegmentKind::LineBreak) {
            start = !start;
        }
        if start { txt_range.start } else { txt_range.end }
    }

    /// Gets the segment index in the line.
    pub fn index(&self) -> usize {
        self.index - self.text.lines.segs(self.line_index).start()
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
                    let mut h = feature.tag.0 as u64;
                    h |= (feature.value as u64) << 32;
                    features.push(h as usize);
                } else {
                    features.push(feature.tag.0 as usize);
                    features.push(feature.value as usize);
                }

                features.push(feature.start as usize);
                features.push(feature.end as usize);
            }
        }

        WordContextKey {
            lang: lang.language,
            script: lang.script,
            direction,
            features: features.into_boxed_slice(),
        }
    }

    pub fn harfbuzz_lang(&self) -> Option<rustybuzz::Language> {
        self.lang.as_str().parse().ok()
    }

    pub fn harfbuzz_script(&self) -> Option<rustybuzz::Script> {
        let t: u32 = self.script?.into();
        let t = t.to_le_bytes(); // Script is a TinyStr4 that uses LE
        rustybuzz::Script::from_iso15924_tag(ttf_parser::Tag::from_bytes(&[t[0], t[1], t[2], t[3]]))
    }

    pub fn harfbuzz_direction(&self) -> rustybuzz::Direction {
        into_harf_direction(self.direction)
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
    fn buffer_segment(&self, segment: &str, key: &WordContextKey) -> rustybuzz::UnicodeBuffer {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.set_direction(key.harfbuzz_direction());
        buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneCharacters);

        if let Some(lang) = key.harfbuzz_lang() {
            buffer.set_language(lang);
        }
        if let Some(script) = key.harfbuzz_script() {
            buffer.set_script(script);
        }

        buffer.push_str(segment);
        buffer
    }

    fn shape_segment_no_cache(&self, seg: &str, key: &WordContextKey, features: &[rustybuzz::Feature]) -> ShapedSegmentData {
        let buffer = if let Some(font) = self.harfbuzz() {
            let buffer = self.buffer_segment(seg, key);
            rustybuzz::shape(&font, features, buffer)
        } else {
            return ShapedSegmentData {
                glyphs: vec![],
                x_advance: 0.0,
                y_advance: 0.0,
            };
        };

        let size_scale = self.metrics().size_scale;
        let to_layout = |p: i32| p as f32 * size_scale;

        let mut w_x_advance = 0.0;
        let mut w_y_advance = 0.0;

        let glyphs: Vec<_> = buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(i, p)| {
                let x_offset = to_layout(p.x_offset);
                let y_offset = -to_layout(p.y_offset);
                let x_advance = to_layout(p.x_advance);
                let y_advance = to_layout(p.y_advance);

                let point = (w_x_advance + x_offset, w_y_advance + y_offset);
                w_x_advance += x_advance;
                w_y_advance += y_advance;

                ShapedGlyph {
                    index: i.glyph_id,
                    cluster: i.cluster,
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

    fn shape_segment<R>(
        &self,
        seg: &str,
        word_ctx_key: &WordContextKey,
        features: &[rustybuzz::Feature],
        out: impl FnOnce(&ShapedSegmentData) -> R,
    ) -> R {
        if !(1..=WORD_CACHE_MAX_LEN).contains(&seg.len()) || self.face().is_empty() {
            let seg = self.shape_segment_no_cache(seg, word_ctx_key, features);
            out(&seg)
        } else if let Some(small) = Self::to_small_word(seg) {
            // try cached
            let cache = self.0.small_word_cache.read();

            let hash = cache.hasher().hash_one(WordCacheKeyRef {
                string: &small,
                ctx_key: word_ctx_key,
            });

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

            let hash = cache.hasher().hash_one(WordCacheKeyRef {
                string: &seg,
                ctx_key: word_ctx_key,
            });

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
        self.shape_space().0
    }

    /// Returns the horizontal advance of the space `' '` character.
    pub fn space_x_advance(&self) -> Px {
        self.shape_space().1
    }

    fn shape_space(&self) -> (GlyphIndex, Px) {
        let mut id = 0;
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
            |r| {
                id = r.glyphs.last().map(|g| g.index).unwrap_or(0);
                adv = r.x_advance;
            },
        );
        (id, Px(adv as _))
    }

    /// Calculates a [`ShapedText`].
    pub fn shape_text(self: &Font, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        ShapedTextBuilder::shape_text(&[self.clone()], text, config)
    }

    /// Sends the sized vector path for a glyph to `sink`.
    ///
    /// Returns the glyph bounds if a full outline was sent to the sink.
    pub fn outline(&self, glyph_id: GlyphIndex, sink: &mut impl OutlineSink) -> Option<PxRect> {
        struct AdapterSink<'a, S> {
            sink: &'a mut S,
            scale: f32,
        }
        impl<S: OutlineSink> ttf_parser::OutlineBuilder for AdapterSink<'_, S> {
            fn move_to(&mut self, x: f32, y: f32) {
                self.sink.move_to(euclid::point2(x, y) * self.scale)
            }

            fn line_to(&mut self, x: f32, y: f32) {
                self.sink.line_to(euclid::point2(x, y) * self.scale)
            }

            fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
                let ctrl = euclid::point2(x1, y1) * self.scale;
                let to = euclid::point2(x, y) * self.scale;
                self.sink.quadratic_curve_to(ctrl, to)
            }

            fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
                let l_from = euclid::point2(x1, y1) * self.scale;
                let l_to = euclid::point2(x2, y2) * self.scale;
                let to = euclid::point2(x, y) * self.scale;
                self.sink.cubic_curve_to((l_from, l_to), to)
            }

            fn close(&mut self) {
                self.sink.close()
            }
        }

        let scale = self.metrics().size_scale;

        let f = self.harfbuzz()?;
        let r = f.outline_glyph(ttf_parser::GlyphId(glyph_id as _), &mut AdapterSink { sink, scale })?;
        Some(
            PxRect::new(
                PxPoint::new(Px(r.x_min as _), Px(r.y_min as _)),
                PxSize::new(Px(r.width() as _), Px(r.height() as _)),
            ) * Factor2d::uniform(scale),
        )
    }

    /// Ray cast an horizontal line across the glyph and returns the entry and exit hits.
    ///
    /// The `line_y_range` are two vertical offsets relative to the baseline, the offsets define
    /// the start and inclusive end of the horizontal line, that is, `(underline, underline + thickness)`, note
    /// that positions under the baseline are negative so a 2px underline set 1px under the baseline becomes `(-1.0, -3.0)`.
    ///
    /// Returns `Ok(Some(x_enter, x_exit))` where the two values are x-advances, returns `None` if there is no hit.
    /// The first x-advance is from the left typographic border to the first hit on the outline,
    /// the second x-advance is from the first across the outline to the exit hit.
    pub fn h_line_hits(&self, glyph_id: GlyphIndex, line_y_range: (f32, f32)) -> Option<(f32, f32)> {
        // Algorithm:
        //
        // - Ignore curves, everything is direct line.
        // - If a line-y crosses `line_y_range` register the min-x and max-x from the two points.
        // - Same if a line is inside `line_y_range`.
        struct InterceptsSink {
            start: Option<euclid::Point2D<f32, Px>>,
            current: euclid::Point2D<f32, Px>,
            under: (bool, bool),

            line_y_range: (f32, f32),
            hit: Option<(f32, f32)>,
        }
        impl OutlineSink for InterceptsSink {
            fn move_to(&mut self, to: euclid::Point2D<f32, Px>) {
                self.start = Some(to);
                self.current = to;
                self.under = (to.y < self.line_y_range.0, to.y < self.line_y_range.1);
            }

            fn line_to(&mut self, to: euclid::Point2D<f32, Px>) {
                let under = (to.y < self.line_y_range.0, to.y < self.line_y_range.1);

                if self.under != under || under == (true, false) {
                    // crossed one or two y-range boundaries or both points are inside
                    self.under = under;

                    let (x0, x1) = if self.current.x < to.x {
                        (self.current.x, to.x)
                    } else {
                        (to.x, self.current.x)
                    };
                    if let Some((min, max)) = &mut self.hit {
                        *min = min.min(x0);
                        *max = max.max(x1);
                    } else {
                        self.hit = Some((x0, x1));
                    }
                }

                self.current = to;
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
                    if s != self.current {
                        self.line_to(s);
                    }
                }
            }
        }
        let mut sink = InterceptsSink {
            start: None,
            current: euclid::point2(0.0, 0.0),
            under: (false, false),

            line_y_range,
            hit: None,
        };
        self.outline(glyph_id, &mut sink)?;

        sink.hit.map(|(a, b)| (a, b - a))
    }
}

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

/// Like [`std::ops::Range<usize>`], but implements [`Copy`].
#[derive(Clone, Copy)]
struct IndexRange(pub usize, pub usize);
impl fmt::Debug for IndexRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.0, self.1)
    }
}
impl IntoIterator for IndexRange {
    type Item = usize;

    type IntoIter = std::ops::Range<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl From<IndexRange> for std::ops::Range<usize> {
    fn from(c: IndexRange) -> Self {
        c.iter()
    }
}
impl From<std::ops::Range<usize>> for IndexRange {
    fn from(r: std::ops::Range<usize>) -> Self {
        IndexRange(r.start, r.end)
    }
}
impl IndexRange {
    pub fn from_bounds(bounds: impl ops::RangeBounds<usize>) -> Self {
        // start..end
        let start = match bounds.start_bound() {
            ops::Bound::Included(&i) => i,
            ops::Bound::Excluded(&i) => i + 1,
            ops::Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            ops::Bound::Included(&i) => i + 1,
            ops::Bound::Excluded(&i) => i,
            ops::Bound::Unbounded => 0,
        };
        Self(start, end)
    }

    /// Into `Range<usize>`.
    pub fn iter(self) -> std::ops::Range<usize> {
        self.0..self.1
    }

    /// `self.0`
    pub fn start(self) -> usize {
        self.0
    }

    /// `self.1`
    pub fn end(self) -> usize {
        self.1
    }

    /// `self.end - self.start`
    pub fn len(self) -> usize {
        self.end() - self.start()
    }
}
impl std::ops::RangeBounds<usize> for IndexRange {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Included(&self.0)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Excluded(&self.1)
    }
}

/// `f32` comparison, panics for `NaN`.
pub fn f32_cmp(a: &f32, b: &f32) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap()
}

fn into_harf_direction(d: LayoutDirection) -> rustybuzz::Direction {
    match d {
        LayoutDirection::LTR => rustybuzz::Direction::LeftToRight,
        LayoutDirection::RTL => rustybuzz::Direction::RightToLeft,
    }
}

#[cfg(test)]
mod tests {
    use crate::{FONTS, Font, FontManager, FontName, FontStretch, FontStyle, FontWeight, SegmentedText, TextShapingArgs, WordContextKey};
    use zng_app::APP;
    use zng_ext_l10n::lang;
    use zng_layout::{
        context::LayoutDirection,
        unit::{Px, PxConstraints2d, TimeUnits},
    };

    fn test_font() -> Font {
        let mut app = APP.minimal().extend(FontManager::default()).run_headless(false);
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
                60.secs(),
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
            test.overflow_align(),
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
            test.overflow_align(),
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
        let mut app = APP.minimal().extend(FontManager::default()).run_headless(false);
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
            60.secs(),
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
