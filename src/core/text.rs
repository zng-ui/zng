//! Font resolving and text shaping.

use super::{
    units::{layout_to_pt, LayoutLength, LayoutPoint, LayoutRect, LayoutSize},
    var::IntoVar,
    var::OwnedVar,
};
use derive_more as dm;
use font_kit::family_name::FamilyName;
use std::{borrow::Cow, fmt, rc::Rc};
use webrender::api::GlyphInstance;
use xi_unicode::LineBreakIterator;

pub use unicode_script::{self, Script};

pub mod font_features;
pub use font_features::FontFeatures;
use font_features::HFontFeatures;

mod font_loading;
pub use font_loading::*;

pub use font_kit::properties::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};
pub use webrender::api::FontInstanceKey;

pub use zero_ui_macros::formatx;

impl FontInstanceRef {
    fn buffer_segment(&self, segment: &str, config: &TextShapingArgs) -> harfbuzz_rs::UnicodeBuffer {
        let mut buffer = harfbuzz_rs::UnicodeBuffer::new().set_direction(if config.right_to_left {
            harfbuzz_rs::Direction::Rtl
        } else {
            harfbuzz_rs::Direction::Ltr
        });
        if config.script != Script::Unknown {
            buffer = buffer.set_script(script_to_tag(config.script)).add_str(segment);
        } else {
            buffer = buffer.add_str(segment).guess_segment_properties();
        }

        buffer
    }
    /// Shapes the text line using the font.
    ///
    /// The `text` should not contain line breaks, if it does the line breaks are ignored.
    pub fn shape_line_deprecated(&self, text: &str, config: &TextShapingArgs) -> ShapedLine {
        let buffer = self.buffer_segment(text, config);

        let mut features = vec![];
        if config.ignore_ligatures {
            features.push(harfbuzz_rs::Feature::new(b"liga", 0, 0..buffer.len()));
        }
        if config.disable_kerning {
            features.push(harfbuzz_rs::Feature::new(b"kern", 0, 0..buffer.len()));
        }

        let metrics = self.metrics();

        let buffer = harfbuzz_rs::shape(&self.harfbuzz_handle(), buffer, &features);

        let baseline = metrics.ascent + metrics.line_gap / 2.0;
        let mut origin = LayoutPoint::new(0.0, baseline);

        let glyphs: Vec<_> = buffer
            .get_glyph_infos()
            .iter()
            .zip(buffer.get_glyph_positions())
            .map(|(i, p)| {
                fn to_layout(p: harfbuzz_rs::Position) -> f32 {
                    // remove our scale of 64 and convert to layout pixels
                    (p as f32 / 64.0) * 96.0 / 72.0
                }
                let x_offset = to_layout(p.x_offset);
                let y_offset = to_layout(p.y_offset);
                let x_advance = to_layout(p.x_advance);
                let y_advance = to_layout(p.y_advance);

                let point = LayoutPoint::new(origin.x + x_offset, origin.y + y_offset);
                origin.x += x_advance + config.letter_spacing;
                origin.y += y_advance;
                // TODO https://harfbuzz.github.io/clusters.html
                GlyphInstance {
                    index: dbg!(i.codepoint),
                    point,
                }
            })
            .collect();

        let bounds = LayoutSize::new(origin.x, config.line_height(metrics));

        ShapedLine { glyphs, baseline, bounds }
    }

    // see https://raphlinus.github.io/text/2020/10/26/text-layout.html
    pub fn shape_text(&self, text: &SegmentedText, config: &TextShapingArgs) -> ShapedText {
        let mut out = ShapedText::default();
        let metrics = self.metrics();
        let line_height = config.line_height(metrics);
        let baseline = metrics.ascent + metrics.line_gap / 2.0;
        let mut origin = LayoutPoint::new(0.0, baseline);
        let mut max_line_x = 0.0;

        for (seg, kind) in text.iter() {
            let mut shape_seg = |cluster_spacing: f32| {
                let buffer = self.buffer_segment(seg, config);
                let buffer = harfbuzz_rs::shape(self.harfbuzz_handle(), buffer, &config.font_features);

                let mut prev_cluster = u32::MAX;
                let glyphs = buffer.get_glyph_infos().iter().zip(buffer.get_glyph_positions()).map(|(i, p)| {
                    fn to_layout(p: harfbuzz_rs::Position) -> f32 {
                        // remove our scale of 64 and convert to layout pixels
                        (p as f32 / 64.0) * 96.0 / 72.0
                    }
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

                    GlyphInstance { index: i.codepoint, point }
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
                    let space_idx = self.harfbuzz_handle().get_nominal_glyph(' ').expect("no U+20 SPACE glyph");
                    let space_advance = self.harfbuzz_handle().get_glyph_h_advance(space_idx) as f32;
                    let point = LayoutPoint::new(origin.x, origin.y);

                    origin.x += config.tab_size(space_advance);

                    out.glyphs.push(GlyphInstance { index: space_idx, point });
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

    pub fn glyph_outline(&self, _line: &ShapedLine) {
        todo!("Implement this after full text shaping")
        // https://docs.rs/font-kit/0.10.0/font_kit/loaders/freetype/struct.Font.html#method.outline
        // Frame of reference: https://searchfox.org/mozilla-central/source/gfx/2d/ScaledFontDWrite.cpp#148
        // Text shaping: https://crates.io/crates/harfbuzz_rs
    }
}

fn script_to_tag(script: Script) -> harfbuzz_rs::Tag {
    let mut name = script.short_name().chars();
    harfbuzz_rs::Tag::new(
        name.next().unwrap(),
        name.next().unwrap(),
        name.next().unwrap(),
        name.next().unwrap(),
    )
}

/// Extra configuration for [`shape_text`](FontInstanceRef::shape_text).
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
    // Finalized font features.
    pub font_features: HFontFeatures,
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
            font_features: HFontFeatures::default(),
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

    #[inline]
    pub fn tab_size(&self, space_advance: f32) -> f32 {
        match self.tab_size {
            TextShapingUnit::Exact(l) => l,
            TextShapingUnit::Relative(r) => space_advance * r,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TextShapingUnit {
    Exact(f32),
    Relative(f32),
}
impl Default for TextShapingUnit {
    fn default() -> Self {
        TextShapingUnit::Exact(0.0)
    }
}

#[derive(Default)]
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

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

/// Result of [`shape_text`](FontInstanceRef::shape_text).
#[derive(Debug, Clone)]
pub struct ShapedLine {
    /// Glyphs for the renderer.
    pub glyphs: Vec<GlyphInstance>,

    /// Baseline within `bounds`.
    ///
    /// This is the font ascent + half the line gap.
    pub baseline: f32,

    /// Size of the text for the layout.
    pub bounds: LayoutSize,
}

/// Configuration of text wrapping for Chinese, Japanese, or Korean text.
#[derive(Debug, Copy, Clone)]
pub enum LineBreak {
    /// The same rule used by other languages.
    Auto,
    /// The least restrictive rule, good for short lines.
    Loose,
    /// The most common rule.
    Normal,
    /// The most stringent rule.
    Strict,
    /// Allow line breaks in between any character including punctuation.
    Anywhere,
}
impl Default for LineBreak {
    /// [`LineBreak::Auto`]
    fn default() -> Self {
        LineBreak::Auto
    }
}

/// Hyphenation configuration.
#[derive(Debug, Copy, Clone)]
pub enum Hyphens {
    /// Hyphens are never inserted in word breaks.
    None,
    /// Word breaks only happen in specially marked break characters: `-` and `\u{00AD} SHY`.
    ///
    /// * `U+2010` - The visible hyphen character.
    /// * `U+00AD` - The invisible hyphen character, is made visible in a word break.
    Manual,
    /// Hyphens are inserted like `Manual` and also using language specific hyphenation rules.
    // TODO https://sourceforge.net/projects/hunspell/files/Hyphen/2.8/
    Auto,
}
impl Default for Hyphens {
    /// [`Hyphens::Auto`]
    fn default() -> Self {
        Hyphens::Auto
    }
}

/// Configure line breaks inside words during text wrap.
///
/// This value is only considered if it is impossible to fit the a word to a line.
///
/// Hyphens can be inserted in word breaks using the [`Hyphens`] configuration.
#[derive(Debug, Copy, Clone)]
pub enum WordBreak {
    /// Line breaks can be inserted in between letters of Chinese/Japanese/Korean text only.
    Normal,
    /// Line breaks can be inserted between any letter.
    BreakAll,
    /// Line breaks are not inserted between any letter.
    KeepAll,
}
impl Default for WordBreak {
    /// [`WordBreak::Normal`]
    fn default() -> Self {
        WordBreak::Normal
    }
}

/// Text alignment.
#[derive(Debug, Copy, Clone)]
pub enum TextAlign {
    /// `Left` in LTR or `Right` in RTL.
    Start,
    /// `Right` in LTR or `Left` in RTL.
    End,

    Left,
    Center,
    Right,

    /// Adjust spacing to fill the available width.
    ///
    /// The justify can be configured using [`Justify`].
    Justify(Justify),
}
impl TextAlign {
    /// Justify Auto.
    #[inline]
    pub fn justify() -> Self {
        TextAlign::Justify(Justify::Auto)
    }
}
impl Default for TextAlign {
    /// [`TextAlign::Start`].
    #[inline]
    fn default() -> Self {
        TextAlign::Start
    }
}

/// Text alignment justification mode.
#[derive(Debug, Copy, Clone)]
pub enum Justify {
    /// Selects the justification mode based on the language.
    /// For Chinese/Japanese/Korean uses `InterLetter` for the others uses `InterWord`.
    Auto,
    /// The text is justified by adding space between words.
    ///
    /// This only works if [`word_spacing`](crate::properties::text_theme::word_spacing) is set to auto.
    InterWord,
    /// The text is justified by adding space between letters.
    ///
    /// This only works if [`letter_spacing`](crate::properties::text_theme::letter_spacing) is set to auto.
    InterLetter,
}
impl Default for Justify {
    /// [`Justify::Auto`]
    fn default() -> Self {
        Justify::Auto
    }
}

/// Various metrics about a [`FontInstanceRef`].
#[derive(Clone, Debug)]
pub struct FontMetrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,

    /// The maximum amount the font rises above the baseline, in layout units.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in layout units.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: f32,

    /// Distance between baselines, in layout units.
    pub line_gap: f32,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in layout units.
    pub underline_position: f32,

    /// A suggested value for the underline thickness, in layout units.
    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in layout units.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in layout units.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounding_box: LayoutRect,
}
impl FontMetrics {
    /// Calculate metrics from global.
    fn new(font_size_px: f32, metrics: &font_kit::metrics::Metrics) -> Self {
        let em = metrics.units_per_em as f32;
        let s = move |f: f32| f / em * font_size_px;
        FontMetrics {
            units_per_em: metrics.units_per_em,
            ascent: s(metrics.ascent),
            descent: s(metrics.descent),
            line_gap: s(metrics.line_gap),
            underline_position: s(metrics.underline_position),
            underline_thickness: s(metrics.underline_thickness),
            cap_height: s(metrics.cap_height),
            x_height: (s(metrics.x_height)),
            bounding_box: {
                let b = metrics.bounding_box;
                LayoutRect::new(
                    LayoutPoint::new(s(b.origin_x()), s(b.origin_y())),
                    LayoutSize::new(s(b.width()), s(b.height())),
                )
            },
        }
    }

    /// The font line height.
    pub fn line_height(&self) -> f32 {
        self.ascent - self.descent + self.line_gap
    }
}

/// Text transform function.
#[derive(Clone)]
pub enum TextTransformFn {
    /// No transform.
    None,
    /// To UPPERCASE.
    Uppercase,
    /// to lowercase.
    Lowercase,
    /// Custom transform function.
    Custom(Rc<dyn Fn(Text) -> Text>),
}
impl TextTransformFn {
    /// Apply the text transform.
    pub fn transform(&self, text: Text) -> Text {
        match self {
            TextTransformFn::None => text,
            TextTransformFn::Uppercase => Text::owned(text.to_uppercase()),
            TextTransformFn::Lowercase => Text::owned(text.to_lowercase()),
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    /// New [`Custom`](Self::Custom).
    pub fn custom(fn_: impl Fn(Text) -> Text + 'static) -> Self {
        TextTransformFn::Custom(Rc::new(fn_))
    }
}
impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}

/// Text white space transform.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum WhiteSpace {
    /// Text is not changed, all white spaces and line breaks are preserved.
    Preserve,
    /// Replace sequences of white space with a single `U+0020 SPACE` and trim lines. Line breaks are preserved.
    Merge,
    /// Replace sequences of white space and line breaks with `U+0020 SPACE` and trim the text.
    MergeNoBreak,
}
impl Default for WhiteSpace {
    /// [`WhiteSpace::Preserve`].
    #[inline]
    fn default() -> Self {
        WhiteSpace::Preserve
    }
}
impl WhiteSpace {
    /// Transform the white space of the text.
    #[inline]
    pub fn transform(self, text: Text) -> Text {
        match self {
            WhiteSpace::Preserve => text,
            WhiteSpace::Merge => text.split_ascii_whitespace().collect::<Vec<_>>().join(" ").into(),
            WhiteSpace::MergeNoBreak => text.split_whitespace().collect::<Vec<_>>().join(" ").into(),
        }
    }
}

/// A possible value for the `font_family` property.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontName(Text);

impl FontName {
    #[inline]
    pub fn new(name: impl Into<Text>) -> Self {
        FontName(name.into())
    }

    /// New "serif" font.
    ///
    /// Serif fonts represent the formal text style for a script.
    #[inline]
    pub fn serif() -> Self {
        Self::new("serif")
    }

    /// New "sans-serif" font.
    ///
    /// Glyphs in sans-serif fonts, are generally low contrast (vertical and horizontal stems have the close to the same thickness)
    /// and have stroke endings that are plain — without any flaring, cross stroke, or other ornamentation.
    #[inline]
    pub fn sans_serif() -> Self {
        Self::new("sans-serif")
    }

    /// New "monospace" font.
    ///
    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    #[inline]
    pub fn monospace() -> Self {
        Self::new("monospace")
    }

    /// New "cursive" font.
    ///
    /// Glyphs in cursive fonts generally use a more informal script style, and the result looks more
    /// like handwritten pen or brush writing than printed letter-work.
    #[inline]
    pub fn cursive() -> Self {
        Self::new("cursive")
    }

    /// New "fantasy" font.
    ///
    /// Fantasy fonts are primarily decorative or expressive fonts that contain decorative or expressive representations of characters.
    #[inline]
    pub fn fantasy() -> Self {
        Self::new("fantasy")
    }

    /// Reference the font name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.0
    }
}
impl From<FamilyName> for FontName {
    #[inline]
    fn from(family_name: FamilyName) -> Self {
        match family_name {
            FamilyName::Title(title) => FontName::new(title),
            FamilyName::Serif => FontName::serif(),
            FamilyName::SansSerif => FontName::sans_serif(),
            FamilyName::Monospace => FontName::monospace(),
            FamilyName::Cursive => FontName::cursive(),
            FamilyName::Fantasy => FontName::fantasy(),
        }
    }
}
impl From<FontName> for FamilyName {
    fn from(font_name: FontName) -> Self {
        match font_name.name() {
            "serif" => FamilyName::Serif,
            "sans-serif" => FamilyName::SansSerif,
            "monospace" => FamilyName::Monospace,
            "cursive" => FamilyName::Cursive,
            "fantasy" => FamilyName::Fantasy,
            _ => FamilyName::Title(font_name.0.into()),
        }
    }
}
impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl IntoVar<Box<[FontName]>> for &'static str {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Box::new([FontName::new(self)]))
    }
}
impl IntoVar<Box<[FontName]>> for String {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Box::new([FontName::new(self)]))
    }
}
impl IntoVar<Box<[FontName]>> for Text {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Box::new([FontName(self)]))
    }
}
impl IntoVar<Box<[FontName]>> for Vec<FontName> {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_boxed_slice())
    }
}
impl IntoVar<Box<[FontName]>> for Vec<&'static str> {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_iter().map(FontName::new).collect::<Vec<FontName>>().into_boxed_slice())
    }
}
impl IntoVar<Box<[FontName]>> for Vec<String> {
    type Var = OwnedVar<Box<[FontName]>>;

    fn into_var(self) -> Self::Var {
        OwnedVar(self.into_iter().map(FontName::new).collect::<Vec<FontName>>().into_boxed_slice())
    }
}

/// Text string type, can be either a `&'static str` or a `String`.
#[derive(Clone, dm::Display, dm::Add, dm::AddAssign, PartialEq, Eq, Hash)]
pub struct Text(Cow<'static, str>);
impl Text {
    /// New text that is a static str.
    pub const fn borrowed(s: &'static str) -> Text {
        Text(Cow::Borrowed(s))
    }

    /// New text that is an owned string.
    pub const fn owned(s: String) -> Text {
        Text(Cow::Owned(s))
    }

    /// If the text is a a static str.
    pub const fn is_borrowed(&self) -> bool {
        match &self.0 {
            Cow::Borrowed(_) => true,
            Cow::Owned(_) => false,
        }
    }

    /// If the text is an owned [`String`].
    pub const fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }

    /// Acquires a mutable reference to string.
    ///
    /// Turns the text to owned if it was borrowed.
    pub fn to_mut(&mut self) -> &mut String {
        self.0.to_mut()
    }

    /// Extracts the owned string.
    ///
    /// Turns the text to owned if it was borrowed.
    pub fn into_owned(self) -> String {
        self.0.into_owned()
    }

    /// Reference the underlying cow.
    pub fn cow(&self) -> &Cow<'static, str> {
        &self.0
    }

    /// Clears the string ([`clear`](String::clear)) if owned otherwise
    /// replaces `self` with [`empty`](Self::empty).
    pub fn clear(&mut self) {
        match &mut self.0 {
            Cow::Borrowed(s) => {
                *s = "";
            }
            Cow::Owned(s) => {
                s.clear();
            }
        }
    }

    /// Removes the last character from the string buffer and returns it.
    /// Returns [`None`] if this `Text` is empty.
    pub fn pop(&mut self) -> Option<char> {
        match &mut self.0 {
            Cow::Borrowed(s) => {
                if let Some((i, c)) = s.char_indices().last() {
                    *s = &s[..i];
                    Some(c)
                } else {
                    None
                }
            }
            Cow::Owned(s) => s.pop(),
        }
    }

    /// New empty static str.
    pub const fn empty() -> Text {
        Self::borrowed("")
    }
}
impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl Default for Text {
    /// Empty.
    fn default() -> Self {
        Self::empty()
    }
}
impl From<Text> for String {
    fn from(t: Text) -> Self {
        t.into_owned()
    }
}
impl_from_and_into_var! {
    fn from(s: &'static str) -> Text {
        Text::borrowed(s)
    }
    fn from(s: String) -> Text {
        Text::owned(s)
    }
    fn from(s: Cow<'static, str>) -> Text {
        Text(s)
    }
}
impl From<Text> for Cow<'static, str> {
    fn from(t: Text) -> Self {
        t.0
    }
}
impl From<Text> for Box<dyn std::error::Error> {
    fn from(err: Text) -> Self {
        err.into_owned().into()
    }
}
impl From<Text> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: Text) -> Self {
        err.into_owned().into()
    }
}
impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl std::borrow::Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}
impl<'a> std::ops::Add<&'a str> for Text {
    type Output = Text;

    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}
impl std::ops::AddAssign<&str> for Text {
    fn add_assign(&mut self, rhs: &str) {
        self.0.to_mut().push_str(rhs);
    }
}
impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(other)
    }
}
impl PartialEq<String> for Text {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}
impl PartialEq<Text> for &str {
    fn eq(&self, other: &Text) -> bool {
        other.0.eq(self)
    }
}
impl PartialEq<Text> for String {
    fn eq(&self, other: &Text) -> bool {
        other.0.eq(self)
    }
}

/// A trait for converting a value to a [`Text`].
///
/// This trait is automatically implemented for any type which implements the [`ToString`] trait.
///
/// You can use [`formatx!`](macro.formatx.html) to `format!` a text.
pub trait ToText {
    fn to_text(self) -> Text;
}
impl<T: ToString> ToText for T {
    fn to_text(self) -> Text {
        self.to_string().into()
    }
}

bitflags! {
    /// Configure if a synthetic font is generated for fonts that do not implement **bold** or *oblique* variants.
    pub struct FontSynthesis: u8 {
        /// No synthetic font generated, if font resolution does not find a variant the matches the requested propertied
        /// the properties are ignored and the normal font is returned.
        const DISABLED = 0;
        /// Enable synthetic bold. Font resolution finds the closest bold variant, the difference added using extra stroke.
        const BOLD = 1;
        /// Enable synthetic oblique. If the font resolution does not find a oblique variant a skew transform is applied.
        const STYLE = 2;
        /// Enabled all synthetic font possibilities.
        const ENABLED = Self::BOLD.bits | Self::STYLE.bits;
    }
}
impl Default for FontSynthesis {
    /// [`FontSynthesis::ENABLED`]
    #[inline]
    fn default() -> Self {
        FontSynthesis::ENABLED
    }
}
impl_from_and_into_var! {
    /// Convert to full [`ENABLED`](FontSynthesis::ENABLED) or [`DISABLED`](FontSynthesis::DISABLED).
    fn from(enabled: bool) -> FontSynthesis {
        if enabled { FontSynthesis::ENABLED } else { FontSynthesis::DISABLED }
    }
}

/// The type of a [text segment](SegmentedText).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TextSegmentKind {
    /// A sequence of characters that cannot be separated by a line-break.
    Word,
    /// A sequence of characters that all have the `White_Space` Unicode property, except the [`Tab`](Self::Tab) and
    ///[`LineBreak`](Self::LineBreak) characters..
    Space,
    /// A sequence of `U+0009 TABULAR` characters.
    Tab,
    /// A single line-break, `\n` or `\r\n`.
    LineBreak,
}

/// Represents a single text segment in a [`SegmentedText`].
#[derive(Clone, Debug)]
pub struct TextSegment {
    /// Segment kind.
    pub kind: TextSegmentKind,
    /// Exclusive end index on the source text.
    ///
    /// The segment range starts from the `end` of the previous segment, or `0`, e.g: `prev_seg.end..self.end`.
    pub end: usize,
}

/// A string segmented in sequences of words, spaces, tabs and separated line breaks.
///
/// Each segment is tagged with a [`TextSegmentKind`] and is represented as
/// an offset from the last segment.
///
/// Line-break segments must be applied and a line-break can be inserted in between the other segment kinds
/// for wrapping the text.
#[derive(Default)]
pub struct SegmentedText {
    text: Text,
    segs: Vec<TextSegment>,
}
impl SegmentedText {
    pub fn new(text: impl Into<Text>) -> Self {
        Self::new_text(text.into())
    }
    fn new_text(text: Text) -> Self {
        let mut segs: Vec<TextSegment> = vec![];
        let text_str: &str = &text;

        for (offset, hard_break) in LineBreakIterator::new(text_str) {
            // a hard-break is a '\n', "\r\n".
            if hard_break {
                // start of this segment.
                let start = segs.last().map(|s| s.end).unwrap_or(0);

                // The segment can have other characters before the line-break character(s).

                let seg = &text_str[start..offset];
                let break_start = if seg.ends_with("\r\n") {
                    // the break was a "\r\n"
                    offset - 2
                } else {
                    debug_assert!(seg.ends_with('\n'));
                    // the break was a '\n'
                    offset - 1
                };

                if break_start > start {
                    // the segment has more characters than the line-break character(s).
                    Self::push_seg(text_str, &mut segs, break_start);
                }
                if break_start < offset {
                    // the line break character(s).
                    segs.push(TextSegment {
                        kind: TextSegmentKind::LineBreak,
                        end: offset,
                    })
                }
            } else {
                // is a soft-break, an opportunity to break the line if needed
                Self::push_seg(text_str, &mut segs, offset);
            }
        }
        SegmentedText { text, segs }
    }
    fn push_seg(text: &str, segs: &mut Vec<TextSegment>, end: usize) {
        let start = segs.last().map(|s| s.end).unwrap_or(0);

        let mut kind = TextSegmentKind::Word;
        for (i, c) in text[start..end].char_indices() {
            let c_kind = if c == '\t' {
                TextSegmentKind::Tab
            } else if ['\u{0020}', '\u{000a}', '\u{000c}', '\u{000d}'].contains(&c) {
                TextSegmentKind::Space
            } else {
                TextSegmentKind::Word
            };

            if c_kind != kind {
                if i > 0 {
                    segs.push(TextSegment { kind, end: i + start });
                }
                kind = c_kind;
            }
        }
        segs.push(TextSegment { kind, end });
    }

    /// The text string.
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The raw segment data.
    #[inline]
    pub fn segs(&self) -> &[TextSegment] {
        &self.segs
    }

    /// Returns `true` if text is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.segs.is_empty()
    }

    /// Destructs `self` into the text and segments.
    #[inline]
    pub fn into_parts(self) -> (Text, Vec<TextSegment>) {
        (self.text, self.segs)
    }

    /// New segmented text from [parts](Self::into_parts).
    ///
    /// # Panics
    ///
    /// Some basic validation is done on the input:
    ///
    /// * If one of the inputs is empty but the other is not.
    /// * If text is not empty and the last segment ends after the last text byte.
    #[inline]
    pub fn from_parts(text: Text, segments: Vec<TextSegment>) -> Self {
        assert_eq!(text.is_empty(), segments.is_empty());
        if !text.is_empty() {
            assert!(segments.last().unwrap().end < text.len());
        }

        SegmentedText { text, segs: segments }
    }

    /// Segments iterator.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui::core::text::SegmentedText;
    /// for (sub_str, segment_kind) in SegmentedText::new("Foo bar!\nBaz.").iter() {
    ///     println!("s: {:?} is a `{:?}`", sub_str, segment_kind);
    /// }
    /// ```
    /// Prints
    /// ```text
    /// "Foo" is a `Word`
    /// " " is a `Space`
    /// "bar!" is a `Word`
    /// "\n" is a `LineBreak`
    /// "Baz." is a `Word`
    /// ```
    #[inline]
    pub fn iter(&self) -> SegmentedTextIter {
        SegmentedTextIter {
            text: &self.text,
            start: 0,
            segs_iter: self.segs.iter(),
        }
    }
}

/// Segmented text iterator.
///
/// This `struct` is created by the [`SegmentedText::iter`] method.
pub struct SegmentedTextIter<'a> {
    text: &'a str,
    start: usize,
    segs_iter: std::slice::Iter<'a, TextSegment>,
}
impl<'a> Iterator for SegmentedTextIter<'a> {
    type Item = (&'a str, TextSegmentKind);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(seg) = self.segs_iter.next() {
            let r = Some((&self.text[self.start..seg.end], seg.kind));
            self.start = seg.end;
            r
        } else {
            None
        }
    }
}

/// An offset in a text.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct TextPoint {
    /// Line index, 0 based.
    pub line: usize,
    /// Byte index in the line text. The byte is in a [char boundary](str::is_char_boundary) and is 0 based.
    pub index: usize,
}
impl TextPoint {
    #[inline]
    pub fn new(line: usize, index: usize) -> Self {
        TextPoint { line, index }
    }

    /// *Ln 1, Col 1* display info.
    ///
    /// `line` if the pointed line.
    #[inline]
    pub fn display(self, line: &str) -> TextPointDisplay {
        TextPointDisplay::new(line, self)
    }
}

/// *Ln 1, Col 1* display info of a [`TextPoint`].
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct TextPointDisplay {
    /// Line number, 1 based.
    pub line: usize,
    /// Character number, 1 based.
    pub column: usize,
}
impl TextPointDisplay {
    /// `line` is the pointed line.
    #[inline]
    pub fn new(line: &str, point: TextPoint) -> Self {
        TextPointDisplay {
            line: point.line + 1,
            column: line[0..point.index].chars().count(),
        }
    }
}
impl fmt::Display for TextPointDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ln {}, Col {}", self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segmented_text1() {
        let t = SegmentedText::new("foo \n\nbar\n");

        use TextSegmentKind::*;
        let expected = vec![
            ("foo", Word),
            (" ", Space),
            ("\n", LineBreak),
            ("\n", LineBreak),
            ("bar", Word),
            ("\n", LineBreak),
        ];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
    #[test]
    fn segmented_text2() {
        let t = SegmentedText::new("baz  \r\n\r\n  fa".to_owned());

        use TextSegmentKind::*;
        let expected = vec![
            ("baz", Word),
            ("  ", Space),
            ("\r\n", LineBreak),
            ("\r\n", LineBreak),
            ("  ", Space),
            ("fa", Word),
        ];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
    #[test]
    fn segmented_text3() {
        let t = SegmentedText::new("\u{200B}	");

        use TextSegmentKind::*;
        let expected = vec![("\u{200B}", Word), ("\t", Tab)];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn segmented_text4() {
        let t = SegmentedText::new("move to 0x0");

        use TextSegmentKind::*;
        let expected = vec![("move", Word), (" ", Space), ("to", Word), (" ", Space), ("0x0", Word)];
        let actual: Vec<_> = t.iter().collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
}
