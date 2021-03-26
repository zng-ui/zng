//! Font resolving and text shaping.

use super::units::{LayoutPoint, LayoutRect, LayoutSize};
use derive_more as dm;
use std::{borrow::Cow, fmt, ops::Deref, rc::Rc};
pub use webrender::api::GlyphInstance;

pub use unicode_script::{self, Script};

pub mod font_features;
pub use font_features::FontFeatures;

mod font_loading;
pub use font_loading::*;

mod segmenting;
pub use segmenting::*;

mod shaping;
pub use shaping::*;

pub use font_kit::properties::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};

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

/// Various metrics that apply to the entire [`FontFace`].
///
/// For OpenType fonts, these mostly come from the `OS/2` table.
#[derive(Clone, Debug)]
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
    pub bounding_box: euclid::Rect<f32, FontUnit>,
}
impl FontFaceMetrics {
    pub fn sized(&self, font_size_px: f32) -> FontMetrics {
        let em = self.units_per_em as f32;
        let s = move |f: f32| f / em * font_size_px;
        FontMetrics {
            ascent: s(self.ascent),
            descent: s(self.descent),
            line_gap: s(self.line_gap),
            underline_position: s(self.underline_position),
            underline_thickness: s(self.underline_thickness),
            cap_height: s(self.cap_height),
            x_height: (s(self.x_height)),
            bounding_box: {
                let b = self.bounding_box;
                LayoutRect::new(
                    LayoutPoint::new(s(b.origin.x), s(b.origin.y)),
                    LayoutSize::new(s(b.size.width), s(b.size.height)),
                )
            },
        }
    }
}
#[doc(hidden)]
pub struct FontUnit;

/// Various metrics about a [`Font`].
///
/// You can compute these metrics from a [`FontFaceMetrics`]
#[derive(Clone, Debug)]
pub struct FontMetrics {
    /// The maximum amount the font rises above the baseline, in layout pixels.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in layout pixels.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: f32,

    /// Distance between baselines, in layout pixels.
    pub line_gap: f32,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in layout pixels.
    pub underline_position: f32,

    /// A suggested value for the underline thickness, in layout pixels.
    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in layout pixels.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in layout pixels.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounding_box: LayoutRect,
}
impl FontMetrics {
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

/// Font family name.
///
/// A possible value for the `font_family` property.
///
/// # Case Insensitive
///
/// Font family names are case-insensitive. `"Arial"` and `"ARIAL"` are equal and have the same hash.
#[derive(Clone, Debug)]
pub struct FontName {
    text: Text,
    is_ascii: bool,
}
impl PartialEq for FontName {
    fn eq(&self, other: &Self) -> bool {
        self.unicase() == other.unicase()
    }
}
impl Eq for FontName {}
impl std::hash::Hash for FontName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.unicase(), state)
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

    #[inline]
    pub fn new(name: impl Into<Text>) -> Self {
        let text = name.into();
        FontName {
            is_ascii: text.is_ascii(),
            text,
        }
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
        &self.text
    }

    /// Unwraps into a [`Text`].
    #[inline]
    pub fn into_text(self) -> Text {
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

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct FontNames(pub Vec<FontName>);
impl Default for FontNames {
    fn default() -> Self {
        FontNames(vec![
            FontName::sans_serif(),
            FontName::serif(),
            FontName::monospace(),
            FontName::cursive(),
            FontName::fantasy(),
        ])
    }
}
impl_from_and_into_var! {
    fn from(font_name: &'static str) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: String) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: Text) -> FontNames {
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
    type Target = [FontName];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
macro_rules! impl_font_names_from_array {
    ($($N:tt),+ $(,)?) => {
        impl_from_and_into_var! {
            $(
            fn from(font_names: [FontName; $N]) -> FontNames {
                FontNames(font_names.into())
            }

            fn from(font_names: [&'static str; $N]) -> FontNames {
                FontNames(std::array::IntoIter::new(font_names).map(FontName::new).collect())
            }

            fn from(font_names: [String; $N]) -> FontNames {
                FontNames(std::array::IntoIter::new(font_names).map(FontName::new).collect())
            }

            )+
        }
    };
}
impl_font_names_from_array! {
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    28,
    29,
    30,
    31,
    32,
}

/// Text string type, can be either a `&'static str` or a `String`.
///
/// Note that this type dereferences to [`str`] so you can use all methods
/// of that type also. For mutation you can call [`to_mut`](Text::to_mut)
/// to access all mutating methods of [`String`]. The mutations that can be
/// implemented using only a borrowed `str` are provided as methods in this type.
#[derive(Clone, dm::Display, dm::Add, dm::AddAssign, PartialEq, Eq, Hash)]
pub struct Text(pub Cow<'static, str>);
impl Text {
    /// New text that is a static str.
    pub const fn borrowed(s: &'static str) -> Text {
        Text(Cow::Borrowed(s))
    }

    /// New text that is an owned string.
    pub const fn owned(s: String) -> Text {
        Text(Cow::Owned(s))
    }

    /// New empty text.
    pub const fn empty() -> Text {
        Self::borrowed("")
    }

    /// If the text is a `&'static str`.
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

    /// Acquires a mutable reference to a [`String`] buffer.
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

    /// Truncates this String, removing all contents.
    ///
    /// This method calls [`String::clear`] if the text is owned, otherwise
    /// replaces `self` with an empty str (`""`).
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

    /// Removes the last character from the text and returns it.
    ///
    /// Returns None if this `Text` is empty.
    ///
    /// This method calls [`String::pop`] if the text is owned, otherwise
    /// reborrows a slice of the `str` without the last character.
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

    /// Shortens this `Text` to the specified length.
    ///
    /// If `new_len` is greater than the text's current length, this has no
    /// effect.
    ///
    /// This method calls [`String::truncate`] if the text is owned, otherwise
    /// reborrows a slice of the text.
    pub fn truncate(&mut self, new_len: usize) {
        match &mut self.0 {
            Cow::Borrowed(s) => {
                if new_len <= s.len() {
                    assert!(s.is_char_boundary(new_len));
                    *s = &s[..new_len];
                }
            }
            Cow::Owned(s) => s.truncate(new_len),
        }
    }

    /// Splits the text into two at the given index.
    ///
    /// Returns a new `Text`. `self` contains bytes `[0, at)`, and
    /// the returned `Text` contains bytes `[at, len)`. `at` must be on the
    /// boundary of a UTF-8 code point.
    ///
    /// This method calls [`String::split_off`] if the text is owned, otherwise
    /// reborrows slices of the text.
    pub fn split_off(&mut self, at: usize) -> Text {
        match &mut self.0 {
            Cow::Borrowed(s) => {
                assert!(s.is_char_boundary(at));
                let other = &s[at..];
                *s = &s[at..];
                Text::borrowed(other)
            }
            Cow::Owned(s) => Text::owned(s.split_off(at)),
        }
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
    fn from(t: Text) -> String {
        t.into_owned()
    }
    fn from(t: Text) -> Cow<'static, str> {
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
    fn to_text(&self) -> Text;
}
impl<T: ToString> ToText for T {
    fn to_text(&self) -> Text {
        self.to_string().into()
    }
}

pub use crate::render::FontSynthesis;

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

/// Creates a [`Text`](crate::text::Text) by calling the `format!` macro and
/// wrapping the result in a `Cow::Owned`.
///
/// # Example
/// ```
/// # use zero_ui_core::text::formatx;
/// let text = formatx!("Hello {}", "World!");
/// ```
#[macro_export]
macro_rules! formatx {
    ($str:tt) => {
        $crate::text::Text::borrowed($str)
    };
    ($($tt:tt)*) => {
        $crate::text::Text::owned(format!($($tt)*))
    };
}
#[doc(inline)]
pub use crate::formatx;
