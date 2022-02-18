use std::{
    hash::{BuildHasher, Hash, Hasher},
    mem,
};

use super::{
    font_features::RFontFeatures, lang, Font, FontList, GlyphIndex, GlyphInstance, InternedStr, Lang, SegmentedText, TextSegment,
    TextSegmentKind,
};
use crate::units::*;

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
    line_height: Px,
    line_spacing: Px,
}
impl ShapedText {
    /// Glyphs for the renderer.
    #[inline]
    pub fn glyphs(&self) -> &[GlyphInstance] {
        &self.glyphs
    }

    /// Glyphs segments.
    #[inline]
    pub fn segments(&self) -> &[TextSegment] {
        &self.glyph_segs
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

    /// Iterate over [`ShapedLine`] selections split by [`LineBreak`].
    #[inline]
    pub fn lines(&self) -> impl Iterator<Item = ShapedLine> {
        struct Lines<'a> {
            segs: std::slice::Iter<'a, TextSegment>,
            start: usize,
            next: usize,
            done: bool,
        }

        impl<'a> Iterator for Lines<'a> {
            type Item = (usize, usize);

            fn next(&mut self) -> Option<Self::Item> {
                if self.done {
                    return None;
                }
                loop {
                    match self.segs.next() {
                        Some(s) => {
                            self.next += 1;
                            if let TextSegmentKind::LineBreak = s.kind {
                                let r = Some((self.start, self.next));
                                self.start = self.next;
                                return r;
                            }
                        }
                        None => {
                            self.done = true;
                            return Some((self.start, self.next));
                        }
                    }
                }
            }
        }

        let lines = Lines {
            segs: self.glyph_segs.iter(),
            start: 0,
            next: 0,
            done: false,
        };

        lines.enumerate().map(|(i, r)| ShapedLine {
            text: self,
            seg_range: r,
            index: i,
        })
    }
}

/// Represents a line selection of a [`ShapedText`].
#[derive(Clone, Copy)]
pub struct ShapedLine<'a> {
    text: &'a ShapedText,
    seg_range: (usize, usize),
    index: usize,
}
impl<'a> ShapedLine<'a> {
    pub fn rect(&self) -> PxRect {
        let height = self.text.line_height;
        let y = height * Px(self.index as i32);
        //let width = self.glyphs().map(|g| );
        todo!()
    }

    // line over full line, exclude trailing space?
    pub fn overline(&self) -> (PxPoint, Px) {
        todo!()
    }

    pub fn strikethrough(&self) -> (PxPoint, Px) {
        todo!()
    }

    pub fn underline(&self) -> (PxPoint, Px) {
        todo!()
    }

    /// Text segments of the line, does not include the line-break that started the line, can include
    /// the line break that starts the next line.
    #[inline]
    pub fn segments(&self) -> &'a [TextSegment] {
        &self.text.glyph_segs[self.seg_range.0..=self.seg_range.1]
    }

    /// Glyphs in the line.
    #[inline]
    pub fn glyphs(&self) -> &'a [GlyphInstance] {
        let start = if self.seg_range.0 == 0 {
            0
        } else {
            self.text.glyph_segs[self.seg_range.0 - 1].end
        };
        let end = self.text.glyph_segs[self.seg_range.1].end;

        &self.text.glyphs[start..=end]
    }

    /// Iterate over word segments.
    #[inline]
    pub fn words(&self) -> impl Iterator<Item = ShapedWord> {
        struct Words<'a> {
            segs: std::slice::Iter<'a, TextSegment>,
            start: usize,
            next: usize,
            done: bool,
        }
        impl<'a> Iterator for Words<'a> {
            type Item = (usize, usize);

            fn next(&mut self) -> Option<Self::Item> {
                if self.done {
                    return None;
                }
                todo!("review what seg-kinds are in a ShapedText");
            }
        }

        let words = Words {
            segs: self.segments().iter(),
            start: 0,
            next: 0,
            done: false,
        };

        words.map(|r| ShapedWord { text: self.text, range: r })
    }
}

/// Represents a word selection of a [`ShapedText`].
#[derive(Clone, Copy)]
pub struct ShapedWord<'a> {
    text: &'a ShapedText,
    range: (usize, usize),
}
impl<'a> ShapedWord<'a> {
    /// Glyphs in the word.
    #[inline]
    pub fn glyphs(&self) -> &'a [GlyphInstance] {
        &self.text.glyphs[self.range.0..=self.range.1]
    }

    pub fn rect(&self) -> PxRect {
        todo!()
    }

    // line over word only, excluding space.
    pub fn overline(&self) -> (PxPoint, Px) {
        todo!()
    }

    pub fn strikethrough(&self) -> (PxPoint, Px) {
        todo!()
    }

    pub fn underline(&self) -> (PxPoint, Px) {
        todo!()
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
    // see https://raphlinus.github.io/text/2020/10/26/text-layout.html
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        // let _scope = tracing::trace_span!("shape_text").entered();

        let mut out = ShapedText::default();
        out.line_height = config.line_height;
        out.line_spacing = config.line_spacing;
        let metrics = self.metrics();
        let line_height = config.line_height.0 as f32;
        let line_spacing = config.line_spacing.0 as f32;
        let baseline = metrics.ascent + metrics.line_gap / 2.0;

        let dft_line_height = self.metrics().line_height().0 as f32;
        let center_height = (line_height - dft_line_height) / 2.0;

        let mut origin = euclid::point2::<_, ()>(0.0, baseline.0 as f32 + center_height);
        let mut max_line_x = 0.0;

        let word_ctx_key = WordContextKey::new(config);

        let letter_spacing = config.letter_spacing.0 as f32;
        let word_spacing = config.word_spacing.0 as f32;
        let tab_x_advance = config.tab_x_advance.0 as f32;
        let tab_index = self.space_index();
        let mut line_count = 1.0;

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
                    line_count += 1.0;
                    max_line_x = origin.x.max(max_line_x);
                    origin.x = 0.0;
                    origin.y += line_height + line_spacing;
                }
            }

            out.glyph_segs.push(TextSegment {
                kind,
                end: out.glyphs.len(),
            });
        }

        // longest line width X line heights.
        out.size = PxSize::new(
            Px(origin.x.max(max_line_x) as i32),
            Px((((line_height + line_spacing) * line_count) - line_spacing) as i32),
        );

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
