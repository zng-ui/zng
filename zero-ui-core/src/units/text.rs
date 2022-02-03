use std::fmt;

use crate::impl_from_and_into_var;

use super::{Factor, FactorPercent, Length};

/// Text line height.
#[derive(Clone, PartialEq)]
pub enum LineHeight {
    /// Default height from the font data.
    ///
    /// The final value is computed from the font metrics: `ascent - descent + line_gap`. This
    /// is usually similar to `1.2.em()`.
    Font,
    /// Height in [`Length`] units.
    ///
    /// Relative lengths are computed to the font size.
    Length(Length),
}
impl fmt::Debug for LineHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineHeight::")?;
        }
        match self {
            LineHeight::Font => write!(f, "Font"),
            LineHeight::Length(l) => f.debug_tuple("Length").field(l).finish(),
        }
    }
}
impl Default for LineHeight {
    /// [`LineHeight::Font`]
    fn default() -> Self {
        LineHeight::Font
    }
}
impl_from_and_into_var! {
    fn from(length: Length) -> LineHeight {
        LineHeight::Length(length)
    }

    /// Percentage of font size.
    fn from(percent: FactorPercent) -> LineHeight {
        LineHeight::Length(percent.into())
    }
    /// Relative to font size.
    fn from(norm: Factor) -> LineHeight {
        LineHeight::Length(norm.into())
    }

    /// Exact size in layout pixels.
    fn from(f: f32) -> LineHeight {
        LineHeight::Length(f.into())
    }
    /// Exact size in layout pixels.
    fn from(i: i32) -> LineHeight {
        LineHeight::Length(i.into())
    }
}

/// Extra spacing added in between text letters.
///
/// Letter spacing is computed using the font data, this unit represents
/// extra space added to the computed spacing.
///
/// A "letter" is a character glyph cluster, e.g.: `a`, `â`, `1`, `-`, `漢`.
#[derive(Clone, PartialEq)]
pub enum LetterSpacing {
    /// Letter spacing can be tweaked when justification is enabled.
    Auto,
    /// Extra space in [`Length`] units.
    ///
    /// Relative lengths are computed from the affected glyph "advance",
    /// that is, how much "width" the next letter will take.
    ///
    /// This variant disables automatic adjustments for justification.
    Length(Length),
}
impl fmt::Debug for LetterSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LetterSpacing::")?;
        }
        match self {
            LetterSpacing::Auto => write!(f, "Auto"),
            LetterSpacing::Length(l) => f.debug_tuple("Length").field(l).finish(),
        }
    }
}
impl Default for LetterSpacing {
    /// [`LetterSpacing::Auto`]
    fn default() -> Self {
        LetterSpacing::Auto
    }
}
impl_from_and_into_var! {
    fn from(length: Length) -> LetterSpacing {
        LetterSpacing::Length(length)
    }

    /// Percentage of font size.
    fn from(percent: FactorPercent) -> LetterSpacing {
        LetterSpacing::Length(percent.into())
    }
    /// Relative to font size.
    fn from(norm: Factor) -> LetterSpacing {
        LetterSpacing::Length(norm.into())
    }

    /// Exact size in layout pixels.
    fn from(f: f32) -> LetterSpacing {
        LetterSpacing::Length(f.into())
    }
    /// Exact size in layout pixels.
    fn from(i: i32) -> LetterSpacing {
        LetterSpacing::Length(i.into())
    }
}

/// Extra spacing added to the Unicode `U+0020 SPACE` character.
///
/// Word spacing is done using the space character "advance" as defined in the font,
/// this unit represents extra spacing added to that default spacing.
///
/// A "word" is the sequence of characters in-between space characters. This extra
/// spacing is applied per space character not per word, if there are three spaces between words
/// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
/// see [`WhiteSpace`](crate::text::WhiteSpace).
#[derive(Clone, PartialEq)]
pub enum WordSpacing {
    /// Word spacing can be tweaked when justification is enabled.
    Auto,
    /// Extra space in [`Length`] units.
    ///
    /// Relative lengths are computed from the default space advance.
    ///
    /// This variant disables automatic adjustments for justification.
    Length(Length),
}
impl fmt::Debug for WordSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WordSpacing")?;
        }
        match self {
            WordSpacing::Auto => write!(f, "Auto"),
            WordSpacing::Length(l) => f.debug_tuple("Length").field(l).finish(),
        }
    }
}
impl Default for WordSpacing {
    /// [`WordSpacing::Auto`]
    fn default() -> Self {
        WordSpacing::Auto
    }
}
impl_from_and_into_var! {
    fn from(length: Length) -> WordSpacing {
        WordSpacing::Length(length)
    }

    /// Percentage of space advance (width).
    fn from(percent: FactorPercent) -> WordSpacing {
        WordSpacing::Length(percent.into())
    }
    /// Relative to the space advance (width).
    fn from(norm: Factor) -> WordSpacing {
        WordSpacing::Length(norm.into())
    }

    /// Exact space in layout pixels.
    fn from(f: f32) -> WordSpacing {
        WordSpacing::Length(f.into())
    }
    /// Exact space in layout pixels.
    fn from(i: i32) -> WordSpacing {
        WordSpacing::Length(i.into())
    }
}

/// Extra spacing in-between paragraphs.
///
/// The initial paragraph space is `line_height + line_spacing * 2`, this extra spacing is added to that.
///
/// A "paragraph" is a sequence of lines in-between blank lines (empty or spaces only). This extra space is applied per blank line
/// not per paragraph, if there are three blank lines between paragraphs the extra spacing is applied trice.
pub type ParagraphSpacing = Length;

/// Length of a `TAB` space.
///
/// Relative lengths are computed from the normal space character "advance" plus the [`WordSpacing`].
/// So a `400%` length is 4 spaces.
pub type TabLength = Length;
