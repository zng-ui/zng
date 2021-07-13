use super::{
    font_features::RFontFeatures, Font, FontList, FontMetrics, GlyphInstance, Script, SegmentedText, TextSegment, TextSegmentKind,
};
use crate::units::{FactorNormal, FactorPercent, LayoutPoint, LayoutSize};

/// Extra configuration for [`shape_text`](Font::shape_text).
#[derive(Debug, Clone)]
pub struct TextShapingArgs {
    /// Extra spacing to add after each character.
    pub letter_spacing: f32,

    /// Extra spacing to add after each space (U+0020 SPACE).
    pub word_spacing: f32,

    /// Height of each line.
    ///
    /// Use [`line_height(..)`](function@Self::line_height) to compute the value.
    pub line_height: Option<f32>,

    /// Unicode script of the text.
    pub script: Script,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

    /// Text is right-to-left.
    pub right_to_left: bool,

    /// Width of the TAB character.
    ///
    /// By default 3 x space.
    pub tab_size: TextShapingUnit,

    /// Extra space before the start of the first line.
    pub text_indent: f32,

    /// Finalized font features.
    pub font_features: RFontFeatures,
}
impl Default for TextShapingArgs {
    fn default() -> Self {
        TextShapingArgs {
            letter_spacing: 0.0,
            word_spacing: 0.0,
            line_height: None,
            script: Script::Unknown,
            ignore_ligatures: false,
            disable_kerning: false,
            right_to_left: false,
            tab_size: TextShapingUnit::Relative(3.0),
            text_indent: 0.0,
            font_features: RFontFeatures::default(),
        }
    }
}
impl TextShapingArgs {
    /// Gets the custom line height or the font line height.
    #[inline]
    pub fn line_height(&self, metrics: &FontMetrics) -> f32 {
        // servo uses the line-gap as default I think.
        self.line_height.unwrap_or_else(|| metrics.line_height())
    }

    /// Gets the custom tab advance.
    #[inline]
    pub fn tab_size(&self, space_advance: f32) -> f32 {
        match self.tab_size {
            TextShapingUnit::Exact(l) => l,
            TextShapingUnit::Relative(r) => space_advance * r,
        }
    }
}

/// Unit of a text shaping size like [`tab_size`](TextShapingArgs::tab_size).
#[derive(Debug, Clone)]
pub enum TextShapingUnit {
    /// The exact size in layout pixels.
    Exact(f32),
    /// A multiplicator for the base size.
    ///
    /// For `tab_size` the base size is the `space` advance, so setting
    /// it to `Relative(3.0)` gives the tab a size of three spaces.
    Relative(f32),
}
impl Default for TextShapingUnit {
    fn default() -> Self {
        TextShapingUnit::Exact(0.0)
    }
}
/// Initializes the factor as a [`Relative`](TextShapingUnit::Relative) value.
impl From<FactorNormal> for TextShapingUnit {
    fn from(f: FactorNormal) -> Self {
        TextShapingUnit::Relative(f.0)
    }
}
/// Initializes the factor as a [`Relative`](TextShapingUnit::Relative) value, dividing by `100`.
impl From<FactorPercent> for TextShapingUnit {
    fn from(p: FactorPercent) -> Self {
        TextShapingUnit::Relative(p.0 / 100.0)
    }
}

/// Output of [text layout](Font::shape_text).

/// Contains a sequence of glyphs positioned in straight [segments](TextSegment).
/// This means that further text wrapping layout can be calculated from this `ShapedText`
/// without needing font information.
#[derive(Clone, Debug, Default)]
pub struct ShapedText {
    glyphs: Vec<GlyphInstance>,
    glyph_segs: Vec<TextSegment>,
    size: LayoutSize,
}
impl ShapedText {
    /// Glyphs for the renderer.
    #[inline]
    pub fn glyphs(&self) -> &[GlyphInstance] {
        &self.glyphs
    }

    /// Bounding box size.
    #[inline]
    pub fn size(&self) -> LayoutSize {
        self.size
    }

    /// No glyphs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

impl Font {
    fn buffer_segment(&self, segment: &str, config: &TextShapingArgs) -> rustybuzz::UnicodeBuffer {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.set_direction(if config.right_to_left {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
        if config.script != Script::Unknown {
            buffer.set_script(to_rustybuzz_script(config.script));
            buffer.push_str(segment);
        } else {
            buffer.push_str(segment);
            buffer.guess_segment_properties();
        }
        buffer
    }

    /// Calculates a [`ShapedText`].
    // see https://raphlinus.github.io/text/2020/10/26/text-layout.html
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        let mut out = ShapedText::default();
        let metrics = self.metrics();
        let line_height = config.line_height(metrics);
        let baseline = metrics.ascent + metrics.line_gap / 2.0;
        let mut origin = LayoutPoint::new(0.0, baseline);
        let mut max_line_x = 0.0;
        let ppem = self.size().get().round() as u16;

        let mut face = rustybuzz::Face::from_slice(&self.face().bytes(), self.face().index()).unwrap();
        face.set_pixels_per_em(Some((ppem, ppem)));
        face.set_points_per_em(None); // TODO?
        face.set_variations(self.variations());

        let to_layout = |p: i32| p as f32 * metrics.size_scale;

        // space metrics used for Tab
        let space_buff = self.buffer_segment(" ", config);
        let space_buff = rustybuzz::shape(&face, &config.font_features, space_buff);
        let space_index = space_buff.glyph_infos()[0].glyph_id;
        let space_advance = to_layout(space_buff.glyph_positions()[0].x_advance);

        for (seg, kind) in text.iter() {
            let mut shape_seg = |cluster_spacing: f32| {
                let buffer = self.buffer_segment(seg, config);
                let buffer = rustybuzz::shape(&face, &config.font_features, buffer);

                let mut prev_cluster = u32::MAX;
                let glyphs = buffer.glyph_infos().iter().zip(buffer.glyph_positions()).map(|(i, p)| {
                    let x_offset = to_layout(p.x_offset);
                    let y_offset = to_layout(p.y_offset);
                    let x_advance = to_layout(p.x_advance);
                    let y_advance = to_layout(p.y_advance);

                    let point = LayoutPoint::new(origin.x + x_offset, origin.y + y_offset);
                    origin.x += x_advance + config.letter_spacing;
                    origin.y += y_advance;

                    if prev_cluster != i.cluster {
                        origin.x += cluster_spacing;
                        prev_cluster = i.cluster;
                    }

                    GlyphInstance { index: i.glyph_id, point }
                });

                out.glyphs.extend(glyphs);
            };

            match kind {
                TextSegmentKind::Word => {
                    shape_seg(config.letter_spacing);
                }
                TextSegmentKind::Space => {
                    shape_seg(config.word_spacing);
                }
                TextSegmentKind::Tab => {
                    let point = LayoutPoint::new(origin.x, origin.y);
                    origin.x += config.tab_size(space_advance);

                    out.glyphs.push(GlyphInstance { index: space_index, point });
                }
                TextSegmentKind::LineBreak => {
                    max_line_x = origin.x.max(max_line_x);
                    origin.x = 0.0;
                    origin.y += line_height;
                }
            }

            out.glyph_segs.push(TextSegment {
                kind,
                end: out.glyphs.len(),
            });
        }

        // longest line width X line heights.
        out.size = LayoutSize::new(origin.x.max(max_line_x), origin.y - metrics.descent); // TODO, add descend?

        out
    }

    /// Gets vector paths that outline the shaped text.
    pub fn glyph_outline(&self, _text: &ShapedText) {
        todo!("Implement this after full text shaping")
        // https://docs.rs/font-kit/0.10.0/font_kit/loaders/freetype/struct.Font.html#method.outline
        // Frame of reference: https://searchfox.org/mozilla-central/source/gfx/2d/ScaledFontDWrite.cpp#148
        // Text shaping: https://crates.io/crates/harfbuzz_rs
    }
}

impl FontList {
    /// Calculates a [`ShapedText`] using the [best](FontList::best) font in this list.
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        // TODO inspect result of best for unknown glyphs, try unknown segments in fallback fonts.
        self.best().shape_text(text, config)
    }
}

fn to_rustybuzz_script(script: Script) -> rustybuzz::Script {
    let t: Vec<_> = script.short_name().bytes().collect();
    rustybuzz::Script::from_iso15924_tag(rustybuzz::Tag::from_bytes(&[t[0], t[1], t[2], t[3]])).unwrap()
}
