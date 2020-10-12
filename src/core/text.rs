//! Font resolving and text shaping.

use super::{
    units::{layout_length_to_pt, LayoutLength, LayoutPoint, LayoutRect, LayoutSize},
    var::IntoVar,
    var::OwnedVar,
};
use font_kit::family_name::FamilyName;
use std::{borrow::Cow, fmt, rc::Rc};
use webrender::api::GlyphInstance;

pub use unicode_script::{self, Script};

pub mod font_features;
pub use font_features::FontFeatures;

mod font_loading;
pub use font_loading::*;

pub use font_kit::properties::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};
pub use webrender::api::FontInstanceKey;

pub use zero_ui_macros::formatx;

/// Font size in round points.
pub type FontSizePt = u32;

/// Convert a [`LayoutLength`] to [`FontSizePt`].
#[inline]
pub fn font_size_from_layout_length(length: LayoutLength) -> FontSizePt {
    layout_length_to_pt(length).round().max(0.0) as u32
}

impl FontInstance {
    /// Shapes the text line using the font.
    ///
    /// The `text` should not contain line breaks, if it does the line breaks are ignored.
    pub fn shape_line(&self, text: &str, config: &ShapingConfig) -> ShapedLine {
        let mut buffer = harfbuzz_rs::UnicodeBuffer::new().set_direction(if config.right_to_left {
            harfbuzz_rs::Direction::Rtl
        } else {
            harfbuzz_rs::Direction::Ltr
        });
        if config.script != Script::Unknown {
            buffer = buffer.set_script(script_to_tag(config.script)).add_str(text);
        } else {
            buffer = buffer.add_str(text).guess_segment_properties();
        }

        let mut features = vec![];
        if config.ignore_ligatures {
            features.push(harfbuzz_rs::Feature::new(b"liga", 0, 0..buffer.len()));
        }
        if config.disable_kerning {
            features.push(harfbuzz_rs::Feature::new(b"kern", 0, 0..buffer.len()));
        }

        let metrics = self.metrics();

        let r = harfbuzz_rs::shape(&self.inner.harfbuzz_font, buffer, &features);

        let baseline = metrics.ascent + metrics.line_gap / 2.0;
        let mut origin = LayoutPoint::new(0.0, baseline);

        let glyphs: Vec<_> = r
            .get_glyph_infos()
            .iter()
            .zip(r.get_glyph_positions())
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
                origin.x += x_advance;
                origin.y += y_advance;
                GlyphInstance { index: i.codepoint, point }
            })
            .collect();

        let bounds = LayoutSize::new(origin.x, config.line_height(metrics));

        ShapedLine { glyphs, baseline, bounds }
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

/// Extra configuration for [`shape_line`](FontInstance::shape_line).
#[derive(Debug, Clone, Default)]
pub struct ShapingConfig {
    /// Extra spacing to add between characters.
    pub letter_spacing: Option<f32>,

    /// Spacing to add between each word.
    ///
    /// Use [`word_spacing(..)`](function@Self::word_spacing) to compute the value.
    pub word_spacing: Option<f32>,

    /// Height of each line.
    ///
    /// Use [`line_height(..)`](function@Self::line_height) to compute the value.
    pub line_height: Option<f32>,

    /// Space to add between each line.
    pub line_spacing: f32,

    /// Space to add between each paragraph.
    ///
    /// use [`paragraph_spacing(.).`](function@Self::paragraph_spacing) to compute the value.
    pub paragraph_spacing: Option<f32>,

    /// Unicode script of the text.
    pub script: Script,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

    /// Text is right-to-left.
    pub right_to_left: bool,

    pub word_break: WordBreak,

    pub line_break: LineBreak,

    pub text_align: TextAlign,

    /// Width of the TAB character.
    ///
    /// By default 3 x space.
    pub tab_size: Option<f32>,

    /// Extra space before the start of the first line.
    pub text_indent: f32,

    /// Collapse/preserve line-breaks/etc.
    pub white_space: WhiteSpace,
}

impl ShapingConfig {
    /// Gets the custom word spacing or 0.25em.
    #[inline]
    pub fn word_spacing(&self, font_size: f32) -> f32 {
        self.word_spacing.unwrap_or(font_size * 0.25)
    }

    /// Gets the custom line height or the font line height.
    #[inline]
    pub fn line_height(&self, metrics: &FontMetrics) -> f32 {
        // servo uses the line-gap as default I think.
        self.line_height.unwrap_or_else(|| metrics.line_height())
    }

    /// Gets the custom paragraph spacing or one line height + two line spacing.
    #[inline]
    pub fn paragraph_spacing(&self, metrics: &FontMetrics) -> f32 {
        self.line_height(metrics) + self.line_spacing * 2.0
    }
}

/// Result of [`shape_line`](FontInstance::shape_line).
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

/// Various metrics about a [`FontInstance`].
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
            TextTransformFn::Uppercase => Cow::Owned(text.to_uppercase()),
            TextTransformFn::Lowercase => Cow::Owned(text.to_lowercase()),
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
pub type Text = Cow<'static, str>;

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

impl IntoVar<Text> for &'static str {
    type Var = OwnedVar<Text>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Cow::from(self))
    }
}
impl IntoVar<Text> for String {
    type Var = OwnedVar<Text>;

    fn into_var(self) -> Self::Var {
        OwnedVar(Cow::from(self))
    }
}
