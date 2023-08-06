//! Font resolving and text shaping.

use crate::l10n::{lang, Lang};
pub use crate::render::webrender_api::{GlyphIndex, GlyphInstance};
use crate::units::*;
use crate::var::animation::Transitionable;
use crate::var::impl_from_and_into_var;
use derive_more as dm;
use std::hash::Hash;
use std::{
    borrow::Cow,
    fmt, mem,
    ops::{Deref, DerefMut},
    sync::Arc,
};

mod emoji_util;
pub use emoji_util::*;

pub mod font_features;
mod font_kit_cache;
mod unicode_bidi_util;

pub use font_features::FontFeatures;

mod font_loading;
pub use font_loading::*;

mod segmenting;
pub use segmenting::*;

mod shaping;
pub use shaping::*;

mod hyphenation;
pub use self::hyphenation::*;

mod ligature_util;

pub use font_kit::properties::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};

impl Transitionable for FontWeight {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        FontWeight(self.0.lerp(&to.0, step))
    }
}

impl Transitionable for FontStretch {
    fn lerp(self, to: &Self, step: EasingStep) -> Self {
        FontStretch(self.0.lerp(&to.0, step))
    }
}

/// Configuration of text wrapping for Chinese, Japanese, or Korean text.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
impl fmt::Debug for LineBreak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineBreak::")?;
        }
        match self {
            LineBreak::Auto => write!(f, "Auto"),
            LineBreak::Loose => write!(f, "Loose"),
            LineBreak::Normal => write!(f, "Normal"),
            LineBreak::Strict => write!(f, "Strict"),
            LineBreak::Anywhere => write!(f, "Anywhere"),
        }
    }
}

/// Hyphenation mode.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Hyphens {
    /// Hyphens are never inserted in word breaks.
    None,
    /// Word breaks only happen in specially marked break characters: `-` and `\u{00AD} SHY`.
    ///
    /// * `U+2010` - The visible hyphen character.
    /// * `U+00AD` - The invisible hyphen character, is made visible in a word break.
    Manual,
    /// Hyphens are inserted like `Manual` and also using language specific hyphenation rules.
    Auto,
}
impl Default for Hyphens {
    /// [`Hyphens::Auto`]
    fn default() -> Self {
        Hyphens::Auto
    }
}
impl fmt::Debug for Hyphens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Hyphens::")?;
        }
        match self {
            Hyphens::None => write!(f, "None"),
            Hyphens::Manual => write!(f, "Manual"),
            Hyphens::Auto => write!(f, "Auto"),
        }
    }
}

/// Configure line breaks inside words during text wrap.
///
/// This value is only considered if it is impossible to fit a full word to a line.
///
/// Hyphens can be inserted in word breaks using the [`Hyphens`] configuration.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
impl fmt::Debug for WordBreak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WordBreak::")?;
        }
        match self {
            WordBreak::Normal => write!(f, "Normal"),
            WordBreak::BreakAll => write!(f, "BreakAll"),
            WordBreak::KeepAll => write!(f, "KeepAll"),
        }
    }
}

/// Text alignment justification mode.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Justify {
    /// Selects the justification mode based on the language.
    ///
    /// For Chinese/Japanese/Korean uses `InterLetter` for the others uses `InterWord`.
    Auto,
    /// The text is justified by adding space between words.
    ///
    /// This only works if [`WordSpacing`](crate::units::WordSpacing) is set to auto.
    InterWord,
    /// The text is justified by adding space between letters.
    ///
    /// This only works if *letter spacing* is set to auto.
    InterLetter,
}
impl Default for Justify {
    /// [`Justify::Auto`]
    fn default() -> Self {
        Justify::Auto
    }
}
impl fmt::Debug for Justify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Justify::")?;
        }
        match self {
            Justify::Auto => write!(f, "Auto"),
            Justify::InterWord => write!(f, "InterWord"),
            Justify::InterLetter => write!(f, "InterLetter"),
        }
    }
}

/// Various metrics that apply to the entire [`FontFace`].
///
/// For OpenType fonts, these mostly come from the `OS/2` table.
///
/// See the [`FreeType Glyph Metrics`] documentation for an explanation of the various metrics.
///
/// [`FreeType Glyph Metrics`]: https://freetype.org/freetype2/docs/glyphs/glyphs-3.html
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FontFaceMetrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,

    /// The maximum amount the font rises above the baseline, in font units.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in font units.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: f32,

    /// Distance between baselines, in font units.
    pub line_gap: f32,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in font units.
    pub underline_position: f32,

    /// A suggested value for the underline thickness, in font units.
    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in font units.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in font units.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounds: euclid::Rect<f32, ()>,
}
impl FontFaceMetrics {
    /// Compute [`FontMetrics`] given a font size in pixels.
    pub fn sized(&self, font_size_px: Px) -> FontMetrics {
        let size_scale = 1.0 / self.units_per_em as f32 * font_size_px.0 as f32;
        let s = move |f: f32| Px((f * size_scale).round() as i32);
        FontMetrics {
            size_scale,
            ascent: s(self.ascent),
            descent: s(self.descent),
            line_gap: s(self.line_gap),
            underline_position: s(self.underline_position),
            underline_thickness: s(self.underline_thickness),
            cap_height: s(self.cap_height),
            x_height: (s(self.x_height)),
            bounds: {
                let b = self.bounds;
                PxRect::new(
                    PxPoint::new(s(b.origin.x), s(b.origin.y)),
                    PxSize::new(s(b.size.width), s(b.size.height)),
                )
            },
        }
    }
}

/// Various metrics about a [`Font`].
///
/// You can compute these metrics from a [`FontFaceMetrics`]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FontMetrics {
    /// Multiply this to a font EM value to get the size in pixels.
    pub size_scale: f32,

    /// The maximum amount the font rises above the baseline, in pixels.
    pub ascent: Px,

    /// The maximum amount the font descends below the baseline, in pixels.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: Px,

    /// Distance between baselines, in pixels.
    pub line_gap: Px,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in pixels.
    pub underline_position: Px,

    /// A suggested value for the underline thickness, in pixels.
    pub underline_thickness: Px,

    /// The approximate amount that uppercase letters rise above the baseline, in pixels.
    pub cap_height: Px,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: Px,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in pixels.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounds: PxRect,
}
impl FontMetrics {
    /// The font line height.
    pub fn line_height(&self) -> Px {
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
    Custom(Arc<dyn Fn(Txt) -> Txt + Send + Sync>),
}
impl TextTransformFn {
    /// Apply the text transform.
    pub fn transform(&self, text: Txt) -> Txt {
        match self {
            TextTransformFn::None => text,
            TextTransformFn::Uppercase => Txt::from_string(text.to_uppercase()),
            TextTransformFn::Lowercase => Txt::from_string(text.to_lowercase()),
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    /// New [`Custom`](Self::Custom).
    pub fn custom(fn_: impl Fn(Txt) -> Txt + Send + Sync + 'static) -> Self {
        TextTransformFn::Custom(Arc::new(fn_))
    }
}
impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "TextTransformFn::")?;
        }
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}
impl PartialEq for TextTransformFn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // can only fail by returning `false` in some cases where the value pointer is actually equal.
            // see: https://github.com/rust-lang/rust/issues/103763
            //
            // we are fine with this, worst case is just an extra var update
            #[allow(clippy::vtable_address_comparisons)]
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// Text white space transform.
#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WhiteSpace {
    /// Text is not changed, all white spaces and line breaks are preserved.
    Preserve,
    /// Replace white spaces with a single `U+0020 SPACE` and trim lines. Line breaks are preserved.
    Merge,
    /// Replace white spaces and line breaks with `U+0020 SPACE` and trim the text.
    MergeAll,
}
impl Default for WhiteSpace {
    /// [`WhiteSpace::Preserve`].
    fn default() -> Self {
        WhiteSpace::Preserve
    }
}
impl WhiteSpace {
    /// Transform the white space of the text.
    pub fn transform(self, text: Txt) -> Txt {
        match self {
            WhiteSpace::Preserve => text,
            WhiteSpace::Merge => {
                let is_white_space = |c: char| c.is_whitespace() && !"\n\r\u{85}".contains(c);
                let t = text.trim_matches(&is_white_space);

                let mut prev_space = false;
                for c in t.chars() {
                    if is_white_space(c) {
                        if prev_space || c != '\u{20}' {
                            // collapse spaces or replace non ' ' white space with ' '.

                            let mut r = String::new();
                            let mut sep = "";
                            for part in t.split(is_white_space).filter(|s| !s.is_empty()) {
                                r.push_str(sep);
                                r.push_str(part);
                                sep = "\u{20}";
                            }
                            return Txt::from_str(&r);
                        } else {
                            prev_space = true;
                        }
                    } else {
                        prev_space = false;
                    }
                }

                if t.len() != text.len() {
                    Txt::from_str(t)
                } else {
                    text
                }
            }
            WhiteSpace::MergeAll => {
                let t = text.trim();

                let mut prev_space = false;
                for c in t.chars() {
                    if c.is_whitespace() {
                        if prev_space || c != '\u{20}' {
                            // collapse spaces or replace non ' ' white space with ' '.

                            let mut r = String::new();
                            let mut sep = "";
                            for part in t.split_whitespace() {
                                r.push_str(sep);
                                r.push_str(part);
                                sep = "\u{20}";
                            }
                            return Txt::from_str(&r);
                        } else {
                            prev_space = true;
                        }
                    } else {
                        prev_space = false;
                    }
                }

                if t.len() != text.len() {
                    Txt::from_str(t)
                } else {
                    text
                }
            }
        }
    }
}
impl fmt::Debug for WhiteSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WhiteSpace::")?;
        }
        match self {
            WhiteSpace::Preserve => write!(f, "Preserve"),
            WhiteSpace::Merge => write!(f, "Merge"),
            WhiteSpace::MergeAll => write!(f, "MergeAll"),
        }
    }
}

/// Font family name.
///
/// A possible value for the `font_family` property.
///
/// # Case Insensitive
///
/// Font family names are case-insensitive. `"Arial"` and `"ARIAL"` are equal and have the same hash.
#[derive(Clone)]
pub struct FontName {
    text: Txt,
    is_ascii: bool,
}
impl fmt::Debug for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("FontName")
                .field("text", &self.text)
                .field("is_ascii", &self.is_ascii)
                .finish()
        } else {
            write!(f, "{:?}", self.text)
        }
    }
}
impl PartialEq for FontName {
    fn eq(&self, other: &Self) -> bool {
        self.unicase() == other.unicase()
    }
}
impl Eq for FontName {}
impl PartialEq<str> for FontName {
    fn eq(&self, other: &str) -> bool {
        self.unicase() == unicase::UniCase::<&str>::from(other)
    }
}
impl Hash for FontName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.unicase(), state)
    }
}
impl FontName {
    fn unicase(&self) -> unicase::UniCase<&str> {
        if self.is_ascii {
            unicase::UniCase::ascii(self)
        } else {
            unicase::UniCase::unicode(self)
        }
    }

    /// New font name from `&'static str`.
    pub const fn from_static(name: &'static str) -> Self {
        FontName {
            text: Txt::from_static(name),
            is_ascii: {
                // str::is_ascii is not const
                let name_bytes = name.as_bytes();
                let mut i = name_bytes.len();
                let mut is_ascii = true;
                while i > 0 {
                    i -= 1;
                    if !name_bytes[i].is_ascii() {
                        is_ascii = false;
                        break;
                    }
                }
                is_ascii
            },
        }
    }

    /// New font name.
    ///
    /// Note that the inner name value is a [`Txt`] so you can define a font name using `&'static str` or `String`.
    ///
    /// Font names are case insensitive but the input casing is preserved, this casing shows during display and in
    /// the value of [`name`](Self::name).
    pub fn new(name: impl Into<Txt>) -> Self {
        let text = name.into();
        FontName {
            is_ascii: text.is_ascii(),
            text,
        }
    }

    /// New "serif" font.
    ///
    /// Serif fonts represent the formal text style for a script.
    pub fn serif() -> Self {
        Self::new("serif")
    }

    /// New "sans-serif" font.
    ///
    /// Glyphs in sans-serif fonts, are generally low contrast (vertical and horizontal stems have the close to the same thickness)
    /// and have stroke endings that are plain — without any flaring, cross stroke, or other ornamentation.
    pub fn sans_serif() -> Self {
        Self::new("sans-serif")
    }

    /// New "monospace" font.
    ///
    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    pub fn monospace() -> Self {
        Self::new("monospace")
    }

    /// New "cursive" font.
    ///
    /// Glyphs in cursive fonts generally use a more informal script style, and the result looks more
    /// like handwritten pen or brush writing than printed letter-work.
    pub fn cursive() -> Self {
        Self::new("cursive")
    }

    /// New "fantasy" font.
    ///
    /// Fantasy fonts are primarily decorative or expressive fonts that contain decorative or expressive representations of characters.
    pub fn fantasy() -> Self {
        Self::new("fantasy")
    }

    /// Reference the font name.
    pub fn name(&self) -> &str {
        &self.text
    }

    /// Unwraps into a [`Txt`].
    pub fn into_text(self) -> Txt {
        self.text
    }
}
impl_from_and_into_var! {
    fn from(s: &'static str) -> FontName {
        FontName::new(s)
    }
    fn from(s: String) -> FontName {
        FontName::new(s)
    }
    fn from(s: Cow<'static, str>) -> FontName {
        FontName::new(s)
    }
    fn from(f: FontName) -> Txt {
        f.into_text()
    }
}
impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
impl std::ops::Deref for FontName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.text.deref()
    }
}
impl AsRef<str> for FontName {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}
impl serde::Serialize for FontName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.text.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for FontName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Txt::deserialize(deserializer).map(FontName::new)
    }
}

/// A list of [font names](FontName) in priority order.
///
/// # Examples
///
/// This type is usually initialized using conversion:
///
/// ```
/// # use zero_ui_core::text::*;
/// fn foo(font_names: impl Into<FontNames>) { }
///
/// foo(["Arial", "sans-serif", "monospace"]);
/// ```
///
/// You can also use the specialized [`push`](Self::push) that converts:
///
/// ```
/// # use zero_ui_core::text::*;
/// let user_preference = "Comic Sans".to_owned();
///
/// let mut names = FontNames::empty();
/// names.push(user_preference);
/// names.push("Arial");
/// names.extend(FontNames::default());
/// ```
///
/// # Default
///
/// The default value is the [`system_ui`](FontNames::system_ui) for the undefined language (`und`).
#[derive(Eq, PartialEq, Hash, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FontNames(pub Vec<FontName>);
impl FontNames {
    /// Empty list.
    pub fn empty() -> Self {
        FontNames(vec![])
    }

    /// Returns the default UI fonts for Windows.
    pub fn windows_ui(lang: &Lang) -> Self {
        // source: VSCode
        // https://github.com/microsoft/vscode/blob/6825c886700ac11d07f7646d8d8119c9cdd9d288/src/vs/code/electron-sandbox/processExplorer/media/processExplorer.css

        if lang!("zh-Hans").matches(&lang, true, false) {
            ["Segoe UI", "Microsoft YaHei", "Segoe Ui Emoji", "sans-serif"].into()
        } else if lang!("zh-Hant").matches(&lang, true, false) {
            ["Segoe UI", "Microsoft Jhenghei", "Segoe Ui Emoji", "sans-serif"].into()
        } else if lang!(ja).matches(&lang, true, false) {
            ["Segoe UI", "Yu Gothic UI", "Meiryo UI", "Segoe Ui Emoji", "sans-serif"].into()
        } else if lang!(ko).matches(&lang, true, false) {
            ["Segoe UI", "Malgun Gothic", "Dotom", "Segoe Ui Emoji", "sans-serif"].into()
        } else {
            ["Segoe UI", "Segoe Ui Emoji", "sans-serif"].into()
        }
    }

    /// Returns the default UI fonts for MacOS/iOS.
    pub fn mac_ui(lang: &Lang) -> Self {
        // source: VSCode

        if lang!("zh-Hans").matches(&lang, true, false) {
            [
                "-apple-system",
                "PingFang SC",
                "Hiragino Sans GB",
                "Apple Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!("zh-Hant").matches(&lang, true, false) {
            ["-apple-system", "PingFang TC", "Apple Color Emoji", "sans-serif"].into()
        } else if lang!(ja).matches(&lang, true, false) {
            ["-apple-system", "Hiragino Kaku Gothic Pro", "Apple Color Emoji", "sans-serif"].into()
        } else if lang!(ko).matches(&lang, true, false) {
            [
                "-apple-system",
                "Nanum Gothic",
                "Apple SD Gothic Neo",
                "AppleGothic",
                "Apple Color Emoji",
                "sans-serif",
            ]
            .into()
        } else {
            ["-apple-system", "Apple Color Emoji", "sans-serif"].into()
        }
    }

    /// Returns the default UI fonts for Linux.
    pub fn linux_ui(lang: &Lang) -> Self {
        // source: VSCode

        if lang!("zh-Hans").matches(&lang, true, false) {
            [
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans SC",
                "Source Han Sans CN",
                "Source Han Sans",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!("zh-Hant").matches(&lang, true, false) {
            [
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans TC",
                "Source Han Sans TW",
                "Source Han Sans",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!(ja).matches(&lang, true, false) {
            [
                "system-ui",
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans J",
                "Source Han Sans JP",
                "Source Han Sans",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!(ko).matches(&lang, true, false) {
            [
                "system-ui",
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans K",
                "Source Han Sans JR",
                "Source Han Sans",
                "UnDotum",
                "FBaekmuk Gulim",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else {
            ["system-ui", "Ubuntu", "Droid Sans", "Noto Color Emoji", "sans-serif"].into()
        }
    }

    /// Returns the default UI fonts for the current operating system.
    pub fn system_ui(lang: &Lang) -> Self {
        if cfg!(windows) {
            Self::windows_ui(lang)
        } else if cfg!(target_os = "linux") {
            Self::linux_ui(lang)
        } else if cfg!(target_os = "mac") {
            Self::mac_ui(lang)
        } else {
            [FontName::sans_serif()].into()
        }
    }

    /// Push a font name from any type that converts to [`FontName`].
    pub fn push(&mut self, font_name: impl Into<FontName>) {
        self.0.push(font_name.into())
    }
}
impl Default for FontNames {
    fn default() -> Self {
        Self::system_ui(&Lang::default())
    }
}
impl fmt::Debug for FontNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FontNames").field(&self.0).finish()
        } else if self.0.is_empty() {
            write!(f, "[]")
        } else if self.0.len() == 1 {
            write!(f, "{:?}", self.0[0])
        } else {
            write!(f, "[{:?}, ", self.0[0])?;
            for name in &self.0[1..] {
                write!(f, "{name:?}, ")?;
            }
            write!(f, "]")
        }
    }
}
impl fmt::Display for FontNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(name) = iter.next() {
            write!(f, "{name}")?;
            for name in iter {
                write!(f, ", {name}")?;
            }
        }

        Ok(())
    }
}
impl_from_and_into_var! {
    fn from(font_name: &'static str) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: String) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: Txt) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_names: Vec<FontName>) -> FontNames {
        FontNames(font_names)
    }

    fn from(font_names: Vec<&'static str>) -> FontNames {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }

    fn from(font_names: Vec<String>) -> FontNames {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }

    fn from(font_name: FontName) -> FontNames {
        FontNames(vec![font_name])
    }
}
impl Deref for FontNames {
    type Target = Vec<FontName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for FontNames {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::iter::Extend<FontName> for FontNames {
    fn extend<T: IntoIterator<Item = FontName>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}
impl IntoIterator for FontNames {
    type Item = FontName;

    type IntoIter = std::vec::IntoIter<FontName>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<const N: usize> From<[FontName; N]> for FontNames {
    fn from(font_names: [FontName; N]) -> Self {
        FontNames(font_names.into())
    }
}
impl<const N: usize> IntoVar<FontNames> for [FontName; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}
impl<const N: usize> From<[&'static str; N]> for FontNames {
    fn from(font_names: [&'static str; N]) -> Self {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [&'static str; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}
impl<const N: usize> From<[String; N]> for FontNames {
    fn from(font_names: [String; N]) -> Self {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [String; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}
impl<const N: usize> From<[Txt; N]> for FontNames {
    fn from(font_names: [Txt; N]) -> Self {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [Txt; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}

/// Defines an insert offset in a shaped text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaretIndex {
    /// Char byte offset in the full text.
    ///
    /// This index can be computed using the [`SegmentedText`].
    pub index: usize,
    /// Line index in the shaped text.
    ///
    /// Note that this counts wrap lines, this value is used disambiguate
    /// between the *end* of a wrap and the *start* of the next, the text
    /// it-self does not have any line break but visually the user interacts
    /// with two lines.
    ///
    /// This index can be computed using the [`ShapedText::snap_caret_line`].
    pub line: usize,
}
impl CaretIndex {
    /// First position.
    pub const ZERO: CaretIndex = CaretIndex { index: 0, line: 0 };
}

const INLINE_MAX: usize = mem::size_of::<usize>() * 3;

fn inline_to_str(d: &[u8; INLINE_MAX]) -> &str {
    let utf8 = if let Some(i) = d.iter().position(|&b| b == b'\0') {
        &d[..i]
    } else {
        &d[..]
    };
    std::str::from_utf8(utf8).unwrap()
}
fn str_to_inline(s: &str) -> [u8; INLINE_MAX] {
    let mut inline = [b'\0'; INLINE_MAX];
    inline[..s.len()].copy_from_slice(s.as_bytes());
    inline
}

#[derive(Clone)]
enum TxtData {
    Static(&'static str),
    Inline([u8; INLINE_MAX]),
    String(String),
    Arc(Arc<str>),
}
impl fmt::Debug for TxtData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Self::Static(s) => write!(f, "Static({s:?})"),
                Self::Inline(d) => write!(f, "Inline({:?})", inline_to_str(d)),
                Self::String(s) => write!(f, "String({s:?})"),
                Self::Arc(s) => write!(f, "Arc({s:?})"),
            }
        } else {
            write!(f, "{:?}", self.deref())
        }
    }
}
impl fmt::Display for TxtData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}
impl PartialEq for TxtData {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}
impl Eq for TxtData {}
impl Hash for TxtData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.deref(), state)
    }
}
impl Deref for TxtData {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            TxtData::Static(s) => s,
            TxtData::Inline(d) => inline_to_str(d),
            TxtData::String(s) => s,
            TxtData::Arc(s) => s,
        }
    }
}

/// Identifies how a [`Txt`] is currently storing the string data.
///
/// Use [`Txt::repr`] to retrieve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxtRepr {
    /// Text data is stored as a `&'static str`.
    Static,
    /// Text data is a small string stored as a null terminated `[u8; {size_of::<usize>() * 3}]`.
    Inline,
    /// Text data is stored as a `String`.
    String,
    /// Text data is stored as an `Arc<str>`.
    Arc,
}

/// Text string type, can be one of multiple internal representations, mostly optimized for sharing and one for editing.
///
/// This type dereferences to [`str`] so you can use all methods of that type.
///
/// For editing some mutable methods are provided, you can also call [`Txt::to_mut`]
/// to access all mutating methods of [`String`]. After editing you can call [`Txt::end_mut`] to convert
/// back to an inner representation optimized for sharing.
///
/// See [`Txt::repr`] for more details about the inner representations.
#[derive(dm::Display, PartialEq, Eq, Hash)]
pub struct Txt(TxtData);
/// Clones the text.
///
/// If the inner representation is [`TxtRepr::String`] the returned value is in a representation optimized
/// for sharing, either a static empty, an inlined short or an `Arc<str>` long string.
impl Clone for Txt {
    fn clone(&self) -> Self {
        Self(match &self.0 {
            TxtData::Static(s) => TxtData::Static(s),
            TxtData::Inline(d) => TxtData::Inline(*d),
            TxtData::String(s) => return Self::from_str(s),
            TxtData::Arc(s) => TxtData::Arc(Arc::clone(s)),
        })
    }
}
impl Txt {
    /// New text that is a `&'static str`.
    pub const fn from_static(s: &'static str) -> Txt {
        Txt(TxtData::Static(s))
    }

    /// New text from a [`String`] optimized for editing.
    ///
    /// If you don't plan to edit the text after this call consider using [`from_str`] instead.
    ///
    /// [`from_str`]: Self::from_str
    pub const fn from_string(s: String) -> Txt {
        Txt(TxtData::String(s))
    }

    /// New cloned from `s`.
    ///
    /// The text will be internally optimized for sharing, if you plan to edit the text after this call
    /// consider using [`from_string`] instead.
    ///
    /// [`from_string`]: Self::from_string
    #[allow(clippy::should_implement_trait)] // have implemented trait, this one is infallible.
    pub fn from_str(s: &str) -> Txt {
        if s.is_empty() {
            Self::from_static("")
        } else if s.len() <= INLINE_MAX && !s.contains('\0') {
            Self(TxtData::Inline(str_to_inline(s)))
        } else {
            Self(TxtData::Arc(Arc::from(s)))
        }
    }

    /// New from a shared arc str.
    ///
    /// Note that the text can outlive the `Arc`, by cloning the string data when modified or
    /// to use a more optimal representation, you cannot use the reference count of `s` to track
    /// the lifetime of the text.
    ///
    /// [`from_string`]: Self::from_string
    pub fn from_arc(s: Arc<str>) -> Txt {
        if s.is_empty() {
            Self::from_static("")
        } else if s.len() <= INLINE_MAX && !s.contains('\0') {
            Self(TxtData::Inline(str_to_inline(&s)))
        } else {
            Self(TxtData::Arc(s))
        }
    }

    /// New text that is an inlined `char`.
    pub fn from_char(c: char) -> Txt {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(4 <= INLINE_MAX, "cannot inline char");

        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        if s.contains('\0') {
            return Txt(TxtData::Arc(Arc::from(&*s)));
        }

        Txt(TxtData::Inline(str_to_inline(s)))
    }

    /// New text from [`format_args!`], avoids allocation if the text is static (no args) or can fit the inlined representation.
    pub fn from_fmt(args: std::fmt::Arguments) -> Txt {
        if let Some(s) = args.as_str() {
            Txt::from_static(s)
        } else {
            let mut r = Txt(TxtData::Inline([b'\0'; INLINE_MAX]));
            std::fmt::write(&mut r, args).unwrap();
            r
        }
    }

    /// Identifies how the text is currently stored.
    pub const fn repr(&self) -> TxtRepr {
        match &self.0 {
            TxtData::Static(_) => TxtRepr::Static,
            TxtData::Inline(_) => TxtRepr::Inline,
            TxtData::String(_) => TxtRepr::String,
            TxtData::Arc(_) => TxtRepr::Arc,
        }
    }

    /// Acquires a mutable reference to a [`String`] buffer.
    ///
    /// Converts the text to an internal representation optimized for editing, you can call [`end_mut`] after
    /// editing to re-optimize the text for sharing.
    ///
    /// [`end_mut`]: Self::end_mut
    pub fn to_mut(&mut self) -> &mut String {
        self.0 = match mem::replace(&mut self.0, TxtData::Static("")) {
            TxtData::String(s) => TxtData::String(s),
            TxtData::Static(s) => TxtData::String(s.to_owned()),
            TxtData::Inline(d) => TxtData::String(inline_to_str(&d).to_owned()),
            TxtData::Arc(s) => TxtData::String((*s).to_owned()),
        };

        if let TxtData::String(s) = &mut self.0 {
            s
        } else {
            unreachable!()
        }
    }

    /// Convert the inner representation of the string to not be [`String`]. After
    /// this call the text can be cheaply cloned.
    pub fn end_mut(&mut self) {
        match mem::replace(&mut self.0, TxtData::Static("")) {
            TxtData::String(s) => {
                *self = Self::from_str(&s);
            }
            already => self.0 = already,
        }
    }

    /// Extracts the owned string.
    ///
    /// Turns the text to owned if it was borrowed.
    pub fn into_owned(self) -> String {
        match self.0 {
            TxtData::String(s) => s,
            TxtData::Static(s) => s.to_owned(),
            TxtData::Inline(d) => inline_to_str(&d).to_owned(),
            TxtData::Arc(s) => (*s).to_owned(),
        }
    }

    /// Calls [`String::clear`] if the text is owned, otherwise
    /// replaces `self` with an empty str (`""`).
    pub fn clear(&mut self) {
        match &mut self.0 {
            TxtData::String(s) => s.clear(),
            d => *d = TxtData::Static(""),
        }
    }

    /// Removes the last character from the text and returns it.
    ///
    /// Returns None if this `Txt` is empty.
    ///
    /// This method only converts to [`TxtRepr::String`] if the
    /// internal representation is [`TxtRepr::Arc`], other representations are reborrowed.
    pub fn pop(&mut self) -> Option<char> {
        match &mut self.0 {
            TxtData::String(s) => s.pop(),
            TxtData::Static(s) => {
                if let Some((i, c)) = s.char_indices().last() {
                    *s = &s[..i];
                    Some(c)
                } else {
                    None
                }
            }
            TxtData::Inline(d) => {
                let s = inline_to_str(d);
                if let Some((i, c)) = s.char_indices().last() {
                    if !s.is_empty() {
                        *d = str_to_inline(&s[..i]);
                    } else {
                        self.0 = TxtData::Static("");
                    }
                    Some(c)
                } else {
                    None
                }
            }
            TxtData::Arc(_) => self.to_mut().pop(),
        }
    }

    /// Shortens this `Txt` to the specified length.
    ///
    /// If `new_len` is greater than the text's current length, this has no
    /// effect.
    ///
    /// This method only converts to [`TxtRepr::String`] if the
    /// internal representation is [`TxtRepr::Arc`], other representations are reborrowed.
    pub fn truncate(&mut self, new_len: usize) {
        match &mut self.0 {
            TxtData::String(s) => s.truncate(new_len),
            TxtData::Static(s) => {
                if new_len <= s.len() {
                    assert!(s.is_char_boundary(new_len));
                    *s = &s[..new_len];
                }
            }
            TxtData::Inline(d) => {
                if new_len == 0 {
                    self.0 = TxtData::Static("");
                } else {
                    let s = inline_to_str(d);
                    if new_len < s.len() {
                        assert!(s.is_char_boundary(new_len));
                        d[new_len..].iter_mut().for_each(|b| *b = b'\0');
                    }
                }
            }
            TxtData::Arc(_) => self.to_mut().truncate(new_len),
        }
    }

    /// Splits the text into two at the given index.
    ///
    /// Returns a new `Txt`. `self` contains bytes `[0, at)`, and
    /// the returned `Txt` contains bytes `[at, len)`. `at` must be on the
    /// boundary of a UTF-8 code point.
    ///
    /// This method only converts to [`TxtRepr::String`] if the
    /// internal representation is [`TxtRepr::Arc`], other representations are reborrowed.
    pub fn split_off(&mut self, at: usize) -> Txt {
        match &mut self.0 {
            TxtData::String(s) => Txt::from_string(s.split_off(at)),
            TxtData::Static(s) => {
                assert!(s.is_char_boundary(at));
                let other = &s[at..];
                *s = &s[at..];
                Txt(TxtData::Static(other))
            }
            TxtData::Inline(d) => {
                let s = inline_to_str(d);
                assert!(s.is_char_boundary(at));
                let a_len = at;
                let b_len = s.len() - at;

                let r = Txt(if b_len == 0 {
                    TxtData::Static("")
                } else {
                    TxtData::Inline(str_to_inline(&s[at..]))
                });

                if a_len == 0 {
                    self.0 = TxtData::Static("");
                } else {
                    *d = str_to_inline(&s[..at]);
                }

                r
            }
            TxtData::Arc(_) => Txt::from_string(self.to_mut().split_off(at)),
        }
    }

    /// Push the character to the end of the text.
    ///
    /// This method avoids converting to [`TxtRepr::String`] when the current text
    /// plus char can fit inlined.
    pub fn push(&mut self, c: char) {
        match &mut self.0 {
            TxtData::String(s) => s.push(c),
            TxtData::Inline(inlined) => {
                if let Some(len) = inlined.iter().position(|&c| c == b'\0') {
                    let c_len = c.len_utf8();
                    if len + c_len <= INLINE_MAX && c != '\0' {
                        let mut buf = [0u8; 4];
                        let s = c.encode_utf8(&mut buf);
                        inlined[len..len + c_len].copy_from_slice(s.as_bytes());
                        return;
                    }
                }
                self.to_mut().push(c)
            }
            _ => {
                let len = self.len();
                let c_len = c.len_utf8();
                if len + c_len <= INLINE_MAX && c != '\0' {
                    let mut inlined = str_to_inline(self.as_str());
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    inlined[len..len + c_len].copy_from_slice(s.as_bytes());

                    self.0 = TxtData::Inline(inlined);
                } else {
                    self.to_mut().push(c)
                }
            }
        }
    }

    /// Push the string to the end of the text.
    ///
    /// This method avoids converting to [`TxtRepr::String`] when the current text
    /// plus char can fit inlined.
    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        match &mut self.0 {
            TxtData::String(str) => str.push_str(s),
            TxtData::Inline(inlined) => {
                if let Some(len) = inlined.iter().position(|&c| c == b'\0') {
                    if len + s.len() <= INLINE_MAX && !s.contains('\0') {
                        inlined[len..len + s.len()].copy_from_slice(s.as_bytes());
                        return;
                    }
                }
                self.to_mut().push_str(s)
            }
            _ => {
                let len = self.len();
                if len + s.len() <= INLINE_MAX && !s.contains('\0') {
                    let mut inlined = str_to_inline(self.as_str());
                    inlined[len..len + s.len()].copy_from_slice(s.as_bytes());

                    self.0 = TxtData::Inline(inlined);
                } else {
                    self.to_mut().push_str(s)
                }
            }
        }
    }

    /// Borrow the text as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.deref()
    }

    /// Copy the inner static `str` if this text represents one.
    pub fn as_static_str(&self) -> Option<&'static str> {
        match self.0 {
            TxtData::Static(s) => Some(s),
            _ => None,
        }
    }
}
impl fmt::Debug for Txt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl Default for Txt {
    /// Empty.
    fn default() -> Self {
        Self::from_static("")
    }
}
impl std::str::FromStr for Txt {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Txt::from_str(s))
    }
}
impl_from_and_into_var! {
    fn from(s: &'static str) -> Txt {
        Txt(TxtData::Static(s))
    }
    fn from(s: String) -> Txt {
        Txt(TxtData::String(s))
    }
    fn from(s: Cow<'static, str>) -> Txt {
        match s {
            Cow::Borrowed(s) => Txt(TxtData::Static(s)),
            Cow::Owned(s) => Txt(TxtData::String(s))
        }
    }
    fn from(c: char) -> Txt {
        Txt::from_char(c)
    }
    fn from(t: Txt) -> String {
        t.into_owned()
    }
    fn from(t: Txt) -> Cow<'static, str> {
        match t.0 {
            TxtData::Static(s) => Cow::Borrowed(s),
            TxtData::String(s) => Cow::Owned(s),
            TxtData::Inline(d) => Cow::Owned(inline_to_str(&d).to_owned()),
            TxtData::Arc(s) => Cow::Owned((*s).to_owned())
        }
    }
    fn from(t: Txt) -> std::path::PathBuf {
        t.into_owned().into()
    }
}
impl From<Txt> for Box<dyn std::error::Error> {
    fn from(err: Txt) -> Self {
        err.into_owned().into()
    }
}
impl From<Txt> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: Txt) -> Self {
        err.into_owned().into()
    }
}
impl std::ops::Deref for Txt {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl AsRef<str> for Txt {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl std::borrow::Borrow<str> for Txt {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl<'a> std::ops::Add<&'a str> for Txt {
    type Output = Txt;

    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}
impl std::ops::AddAssign<&str> for Txt {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
impl PartialEq<&str> for Txt {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}
impl PartialEq<str> for Txt {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<String> for Txt {
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<Txt> for &str {
    fn eq(&self, other: &Txt) -> bool {
        other.as_str().eq(*self)
    }
}
impl PartialEq<Txt> for str {
    fn eq(&self, other: &Txt) -> bool {
        other.as_str().eq(self)
    }
}
impl PartialEq<Txt> for String {
    fn eq(&self, other: &Txt) -> bool {
        other.as_str().eq(self)
    }
}
impl serde::Serialize for Txt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> serde::Deserialize<'de> for Txt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Txt::from)
    }
}
impl AsRef<[u8]> for Txt {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_ref()
    }
}
impl std::fmt::Write for Txt {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

/// A trait for converting a value to a [`Txt`].
///
/// This trait is automatically implemented for any type which implements the [`ToString`] trait.
///
/// You can use [`formatx!`](macro.formatx.html) to `format!` a text.
pub trait ToText {
    /// Converts the given value to an owned [`Txt`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use zero_ui_core::text::*;
    ///
    /// let expected = formatx!("10");
    /// let actual = 10.to_text();
    ///
    /// assert_eq!(expected, actual);
    /// ```
    fn to_text(&self) -> Txt;
}
impl<T: ToString> ToText for T {
    fn to_text(&self) -> Txt {
        self.to_string().into()
    }
}

pub use crate::render::FontSynthesis;

bitflags! {
    /// Represents what parts of a text the underline must skip over.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct UnderlineSkip: u8 {
        /// Underline spans the entire text length.
        const NONE = 0;

        /// Skip white space.
        const SPACES = 0b0001;

        /// Skip over glyph descenders that intersect with the underline.
        const GLYPHS = 0b0010;

        /// Default value, skip glyphs.
        const DEFAULT = Self::GLYPHS.bits();
    }
}
impl Default for UnderlineSkip {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Defines what line gets traced by the text underline decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum UnderlinePosition {
    /// Underline is positioned using the offset defined in the font file.
    #[default]
    Font,
    /// Underline is positioned after the text *descent*, avoiding crossover with all glyph descenders.
    Descent,
}

///<span data-del-macro-root></span> Creates a [`Txt`] by formatting using the [`format_args!`] syntax.
///
/// Note that this behaves like a [`format!`] for [`Txt`], but it can be more performant because the
/// text type can represent `&'static str` and can i
///
/// # Examples
///
/// ```
/// # use zero_ui_core::text::formatx;
/// let text = formatx!("Hello {}", "World!");
/// ```
///
/// [`Txt`]: crate::text::Txt
#[macro_export]
macro_rules! formatx {
    ($($tt:tt)*) => {
        $crate::text::Txt::from_fmt(format_args!($($tt)*))
    };
}
#[doc(inline)]
pub use crate::formatx;
use crate::var::{IntoVar, LocalVar};

#[cfg(test)]
mod tests {
    use crate::context::LayoutDirection;

    use super::*;

    #[test]
    fn segmented_text1() {
        let t = SegmentedText::new("foo \n\nbar\n", LayoutDirection::LTR);

        use TextSegmentKind::*;
        let expected = vec![
            ("foo", LeftToRight),
            (" ", Space),
            ("\n", LineBreak),
            ("\n", LineBreak),
            ("bar", LeftToRight),
            ("\n", LineBreak),
        ];
        let actual: Vec<_> = t.iter().map(|(s, k)| (s, k.kind)).collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
    #[test]
    fn segmented_text2() {
        let t = SegmentedText::new("baz  \r\n\r\n  fa".to_owned(), LayoutDirection::LTR);

        use TextSegmentKind::*;
        let expected = vec![
            ("baz", LeftToRight),
            ("  ", Space),
            ("\r\n", LineBreak),
            ("\r\n", LineBreak),
            ("  ", Space),
            ("fa", LeftToRight),
        ];
        let actual: Vec<_> = t.iter().map(|(s, k)| (s, k.kind)).collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{:?}", actual);
            assert_eq!(expected, actual);
        }
    }
    #[test]
    fn segmented_text3() {
        let t = SegmentedText::new("\u{200B}	", LayoutDirection::LTR);

        use TextSegmentKind::*;
        let expected = vec![("\u{200B}", BoundaryNeutral), ("\t", Tab)];
        let actual: Vec<_> = t.iter().map(|(s, k)| (s, k.kind)).collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{actual:?}");
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn segmented_text4() {
        let t = SegmentedText::new("move to 0x0", LayoutDirection::LTR);

        use TextSegmentKind::*;
        let expected = vec![
            ("move", LeftToRight),
            (" ", Space),
            ("to", LeftToRight),
            (" ", Space),
            ("0", EuropeanNumber),
            ("x", LeftToRight),
            ("0", EuropeanNumber),
        ];
        let actual: Vec<_> = t.iter().map(|(s, k)| (s, k.kind)).collect();

        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.into_iter().zip(actual) {
            //println!("{actual:?}");
            assert_eq!(expected, actual);
        }
    }
}
