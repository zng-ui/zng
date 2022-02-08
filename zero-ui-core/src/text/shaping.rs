use std::{
    hash::{BuildHasher, Hash, Hasher},
    mem,
};

use super::{
    font_features::RFontFeatures, lang, Font, FontList, FontMetrics, GlyphInstance, InternedStr, Lang, SegmentedText, TextSegment,
    TextSegmentKind,
};
use crate::units::*;

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
    pub line_height: Option<Px>,

    /// Language of the text, also identifies if RTL.
    pub lang: Lang,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

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
            lang: lang!(und),
            ignore_ligatures: false,
            disable_kerning: false,
            tab_size: TextShapingUnit::Relative(3.0),
            text_indent: 0.0,
            font_features: RFontFeatures::default(),
        }
    }
}
impl TextShapingArgs {
    /// Gets the custom line height or the font line height.
    #[inline]
    pub fn line_height(&self, metrics: &FontMetrics) -> Px {
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
impl From<Factor> for TextShapingUnit {
    fn from(f: Factor) -> Self {
        TextShapingUnit::Relative(f.0)
    }
}
/// Initializes the factor as a [`Relative`](TextShapingUnit::Relative) value, dividing by `100`.
impl From<FactorPercent> for TextShapingUnit {
    fn from(p: FactorPercent) -> Self {
        TextShapingUnit::Relative(p.0 / 100.0)
    }
}

/// Output of [text layout].

/// Contains a sequence of glyphs positioned in straight [segments](TextSegment).
/// This means that further text wrapping layout can be calculated from this `ShapedText`
/// without needing font information.
///
/// [text layout]: Font::shape_text
#[derive(Clone, Debug, Default)]
pub struct ShapedText {
    glyphs: Vec<GlyphInstance>,
    glyph_segs: Vec<TextSegment>,
    size: PxSize,
}
impl ShapedText {
    /// Glyphs for the renderer.
    #[inline]
    pub fn glyphs(&self) -> &[GlyphInstance] {
        &self.glyphs
    }

    /// Bounding box size.
    #[inline]
    pub fn size(&self) -> PxSize {
        self.size
    }

    /// No glyphs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
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
pub(super) struct ShapedSegment {
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

    fn shape_segment_no_cache(&self, seg: &str, lang: &Lang, features: &[harfbuzz_rs::Feature]) -> ShapedSegment {
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

        ShapedSegment {
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
        out: impl FnOnce(&ShapedSegment),
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

    /// Calculates a [`ShapedText`].
    // see https://raphlinus.github.io/text/2020/10/26/text-layout.html
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        // let _scope = tracing::trace_span!("shape_text").entered();

        let mut out = ShapedText::default();
        let metrics = self.metrics();
        let line_height = config.line_height(metrics).0 as f32;
        let baseline = metrics.ascent + metrics.line_gap / 2.0;
        let mut origin = euclid::point2::<_, ()>(0.0, baseline.0 as f32);
        let mut max_line_x = 0.0;

        let word_ctx_key = WordContextKey::new(config);

        for (seg, kind) in text.iter() {
            match kind {
                TextSegmentKind::Word => {
                    self.shape_segment(seg, &word_ctx_key, &config.lang, &config.font_features, |shaped_seg| {
                        out.glyphs.extend(shaped_seg.glyphs.iter().map(|gi| {
                            let r = GlyphInstance {
                                index: gi.index,
                                point: euclid::point2(gi.point.0 + origin.x, gi.point.1 + origin.y),
                            };
                            origin.x += config.letter_spacing;
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
                            origin.x += config.word_spacing;
                            r
                        }));
                        origin.x += shaped_seg.x_advance;
                        origin.y += shaped_seg.y_advance;
                    });
                }
                TextSegmentKind::Tab => {
                    self.shape_segment(" ", &word_ctx_key, &config.lang, &config.font_features, |s| {
                        let space = s.glyphs[0];
                        let point = euclid::point2(origin.x, origin.y);
                        origin.x += config.tab_size(s.x_advance);
                        out.glyphs.push(GlyphInstance { index: space.index, point });
                    });
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
        out.size = PxSize::new(Px(origin.x.max(max_line_x) as i32), Px(origin.y as i32) - metrics.descent); // TODO, add descend?

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

fn to_buzz_lang(lang: unic_langid::subtags::Language) -> Option<harfbuzz_rs::Language> {
    lang.as_str().parse().ok()
}

fn to_buzz_script(script: unic_langid::subtags::Script) -> harfbuzz_rs::Tag {
    let t: u32 = script.into();
    let t = t.to_le_bytes(); // Script is a TinyStr4 that uses LE
    harfbuzz_rs::Tag::from(&[t[0], t[1], t[2], t[3]])
}

#[cfg(test)]
mod tests {}
