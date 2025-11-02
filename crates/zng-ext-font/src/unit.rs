use zng_layout::unit::*;
use zng_var::impl_from_and_into_var;

/// Text font size.
///
/// The [`Default`] value is the *root* font size, usually the one set in the window widget.
///
/// [`Default`]: Length::Default
pub type FontSize = Length;

/// Text line height.
///
/// The [`Default`] value is computed from the font metrics, `ascent - descent + line_gap`, this is
/// usually similar to `1.2.em()`. Relative values are computed from the default value, so `200.pct()` is double
/// the default line height.
///
/// [`Default`]: Length::Default
pub type LineHeight = Length;

/// Extra spacing added in between text letters.
///
/// Letter spacing is computed using the font data, this unit represents
/// extra space added to the computed spacing.
///
/// A "letter" is a character glyph cluster, e.g.: `a`, `â`, `1`, `-`, `漢`.
///
/// The [`Default`] value signals that letter spacing can be tweaked when text *justification* is enabled, all other
/// values disable automatic adjustments for justification. Relative values are computed from the length of the space `' '` character.
///
/// [`Default`]: Length::Default
pub type LetterSpacing = Length;

/// Extra spacing added to the Unicode `U+0020 SPACE` character.
///
/// Word spacing is done using the space character "advance" as defined in the font,
/// this unit represents extra spacing added to that default spacing.
///
/// A "word" is the sequence of characters in-between space characters. This extra
/// spacing is applied per space character not per word, if there are three spaces between words
/// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
/// see [`WhiteSpace`](crate::WhiteSpace).
///
/// The [`Default`] value signals that word spacing can be tweaked when text *justification* is enabled, all other
/// values disable automatic adjustments for justification. Relative values are computed from the length of the space `' '` character,
/// so a word spacing of `100.pct()` visually adds *another* space in between words.
///
/// [`Default`]: Length::Default
pub type WordSpacing = Length;

/// Extra spacing in-between text lines.
///
/// The [`Default`] value is zero. Relative values are calculated from the [`LineHeight`], so `50.pct()` is half
/// the computed line height.
///
/// [`Default`]: Length::Default
pub type LineSpacing = Length;

/// Extra spacing in-between paragraphs.
///
/// The initial paragraph space is `line_height + line_spacing * 2`, this extra spacing is added to that.
///
/// A "paragraph" is a sequence of lines in-between wgt lines (empty or spaces only). This extra space is applied per wgt line
/// not per paragraph, if there are three wgt lines between paragraphs the extra spacing is applied trice.
///
/// The [`Default`] value is zero.
///
/// [`Default`]: Length::Default
pub type ParagraphSpacing = Length;

/// Length of a `TAB` space.
///
/// Relative lengths are computed from the normal space character "advance" plus the [`WordSpacing`].
/// So a `200%` length is 2 spaces.
///
/// The [`Default`] value is `400.pct()`, 4 spaces.
///
/// [`Default`]: Length::Default
pub type TabLength = Length;

/// Height of the text underline decoration.
///
/// Relative lengths are computed from `1.em()`, with a minimum of one pixel.
///
/// The [`Default`] value is defined by the font.
///
/// [`Default`]: Length::Default
pub type UnderlineThickness = Length;

/// Height of the text overline or strikethrough decoration.
///
/// Relative lengths are computed from `1.em()`, with a minimum of one pixel.
///
/// The [`Default`] value is `10.pct()`.
pub type TextLineThickness = Length;

/// Extra spacing at the start of lines.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Indentation {
    /// The ident space width.
    pub spacing: Length,
    /// If `false` indents only the first lines after a line break.
    ///
    /// If `true` indent all lines except the first lines (hang).
    pub invert: bool,
}
impl_from_and_into_var! {
    fn from(percent: FactorPercent) -> Indentation {
        Length::from(percent).into()
    }
    fn from(norm: Factor) -> Indentation {
        Length::from(norm).into()
    }
    fn from(f: f32) -> Indentation {
        Length::from(f).into()
    }
    fn from(i: i32) -> Indentation {
        Length::from(i).into()
    }
    fn from(l: Px) -> Indentation {
        Length::from(l).into()
    }
    fn from(l: Dip) -> Indentation {
        Length::from(l).into()
    }
    fn from(expr: LengthExpr) -> Indentation {
        Length::from(expr).into()
    }
    fn from(spacing: Length) -> Indentation {
        Indentation { spacing, invert: false }
    }

    fn from<S: Into<Length>>(spacing_invert: (S, bool)) -> Indentation {
        Indentation {
            spacing: spacing_invert.0.into(),
            invert: spacing_invert.1,
        }
    }
}
