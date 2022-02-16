use super::Length;

/// Text font size.
///
/// The [`Default`] value is the *root* font size, usually the one set in the window widget.
///
/// [`Default`]: Length::Default
pub type FontSize = Length;

/// Text line height.
///
/// The [`Default`] value is computed from the font metrics, `ascent - descent + line_gap`, this is
/// usually similar to `1.2.em()`.
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
/// values disable automatic adjustments for justification.
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
/// see [`WhiteSpace`](crate::text::WhiteSpace).
///
/// The [`Default`] value signals that word spacing can be tweaked when text *justification* is enabled, all other
/// values disable automatic adjustments for justification.
///
/// [`Default`]: Length::Default
pub type WordSpacing = Length;

/// Extra spacing in-between text lines.
///
/// The [`Default`] value is zero.
///
/// [`Default`]: Length::Default
pub type LineSpacing = Length;

/// Extra spacing in-between paragraphs.
///
/// The initial paragraph space is `line_height + line_spacing * 2`, this extra spacing is added to that.
///
/// A "paragraph" is a sequence of lines in-between blank lines (empty or spaces only). This extra space is applied per blank line
/// not per paragraph, if there are three blank lines between paragraphs the extra spacing is applied trice.
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
