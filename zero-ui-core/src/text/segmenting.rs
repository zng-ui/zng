use std::ops;

use crate::{context::LayoutDirection, crate_util::FxHashMap};

use super::Txt;
use unicode_bidi::BidiInfo;
use xi_unicode::LineBreakIterator;

/// The type of a text segment.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextSegmentKind {
    /// Any strong left-to-right character.
    LeftToRight,
    /// Any strong right-to-left (non-Arabic-type) character.
    RightToLeft,
    /// Any strong right-to-left (Arabic-type) character.
    ArabicLetter,

    /// Any ASCII digit or Eastern Arabic-Indic digit.
    EuropeanNumber,
    /// Plus and minus signs.
    EuropeanSeparator,
    /// A terminator in a numeric format context, includes currency signs.
    EuropeanTerminator,
    /// Any Arabic-Indic digit.
    ArabicNumber,
    /// Commas, colons, and slashes.
    CommonSeparator,
    /// Any non-spacing mark.
    NonSpacingMark,
    /// Most format characters, control codes, or noncharacters.
    BoundaryNeutral,

    /// Various newline characters.
    LineBreak,
    /// A sequence of `'\t', '\v'` or `'\u{1F}'`.
    Tab,
    /// Spaces.
    Space,
    /// Most other symbols and punctuation marks.
    OtherNeutral,
    /// Open or close bidi bracket.
    ///
    /// Can be any chars in <https://unicode.org/Public/UNIDATA/BidiBrackets.txt>.
    Bracket(char),

    /// Bidi control character.
    ///
    /// Chars can be:
    ///
    /// * `\u{202A}`: The LR embedding control.
    /// * `\u{202D}`: The LR override control.
    /// * `\u{202B}`: The RL embedding control.
    /// * `\u{202E}`: The RL override control.
    /// * `\u{202C}`: Terminates an embedding or override control.
    ///
    /// * `\u{2066}`: The LR isolate control.
    /// * `\u{2067}`: The RL isolate control.
    /// * `\u{2068}`: The first strong isolate control.
    /// * `\u{2069}`: Terminates an isolate control.
    BidiCtrl(char),
}
impl TextSegmentKind {
    /// Returns `true` if the segment can be considered part of a word for the purpose of inserting letter spacing.
    pub fn is_word(self) -> bool {
        use TextSegmentKind::*;
        matches!(
            self,
            LeftToRight
                | RightToLeft
                | ArabicLetter
                | EuropeanNumber
                | EuropeanSeparator
                | EuropeanTerminator
                | ArabicNumber
                | CommonSeparator
                | NonSpacingMark
                | BoundaryNeutral
                | OtherNeutral
                | Bracket(_)
        )
    }

    /// Returns `true` if the segment can be considered part of space between words for the purpose of inserting word spacing.
    pub fn is_space(self) -> bool {
        matches!(self, Self::Space | Self::Tab)
    }

    /// Returns `true` if the segment terminates the current line.
    ///
    /// Line break segments are the last segment of their line and explicitly start a new line.
    pub fn is_line_break(self) -> bool {
        matches!(self, Self::LineBreak)
    }

    /// If multiple segments of this same kind can be represented by a single segment in the Unicode bidi algorithm.
    pub fn can_merge(self) -> bool {
        use TextSegmentKind::*;
        !matches!(self, Bracket(_) | BidiCtrl(_))
    }

    /// Get more info about the bracket char if `self` is `Bracket(_)` with a valid char.
    pub fn bracket_info(self) -> Option<unicode_bidi::data_source::BidiMatchedOpeningBracket> {
        if let TextSegmentKind::Bracket(c) = self {
            super::unicode_bidi_util::bidi_bracket_data(c)
        } else {
            None
        }
    }

    /// Gets the layout direction this segment will always be in, independent of the base direction.
    ///
    /// Returns `None` if the segment direction depends on the line context.
    pub fn strong_direction(self) -> Option<LayoutDirection> {
        use TextSegmentKind::*;

        match self {
            LeftToRight => Some(LayoutDirection::LTR),
            RightToLeft | ArabicLetter => Some(LayoutDirection::RTL),
            BidiCtrl(_) => {
                use unicode_bidi::BidiClass::*;
                match unicode_bidi::BidiClass::from(self) {
                    LRE | LRO | LRI => Some(LayoutDirection::LTR),
                    RLE | RLO | RLI => Some(LayoutDirection::RTL),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
impl From<char> for TextSegmentKind {
    fn from(c: char) -> Self {
        use unicode_bidi::*;

        unicode_bidi::HardcodedBidiData.bidi_class(c).into()
    }
}

pub use unicode_bidi::Level as BidiLevel;

/// Represents a single text segment in a [`SegmentedText`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSegment {
    /// Segment kind.
    pub kind: TextSegmentKind,
    /// Direction of the segment in the context of other segments of the line.
    pub level: BidiLevel,

    /// Exclusive end index on the source text.
    ///
    /// The segment range starts from the `end` of the previous segment, or `0`, e.g: `prev_seg.end..self.end`.
    pub end: usize,
}
impl TextSegment {
    /// Direction of the glyphs in the segment.
    ///
    /// Segments iterate in the logical order, that is, the order the text is typed. If two segments
    /// in the same line have direction `RTL` they must be layout the first to the right of the second.
    pub fn direction(self) -> LayoutDirection {
        self.level.into()
    }
}

/// A string segmented in sequences of words, spaces, tabs and separated line breaks.
///
/// Each segment is tagged with a [`TextSegmentKind`] and is represented as
/// an offset from the last segment.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SegmentedText {
    text: Txt,
    segments: Vec<TextSegment>,
    base_direction: LayoutDirection,
}
impl SegmentedText {
    /// New segmented text from any text type.
    pub fn new(text: impl Into<Txt>, base_direction: LayoutDirection) -> Self {
        Self::new_text(text.into(), base_direction)
    }
    fn new_text(text: Txt, base_direction: LayoutDirection) -> Self {
        let mut segs: Vec<TextSegment> = vec![];
        let text_str: &str = &text;
        let bidi = BidiInfo::new(text_str, Some(base_direction.into()));

        for (offset, hard_break) in LineBreakIterator::new(text_str) {
            // a hard-break is a '\n', '\r', "\r\n".
            if hard_break {
                // start of this segment.
                let start = segs.last().map(|s| s.end).unwrap_or(0);

                // The segment can have other characters before the line-break character(s).

                let seg = &text_str[start..offset];
                let break_start = if seg.ends_with("\r\n") {
                    // the break was a "\r\n"
                    offset - 2
                } else {
                    debug_assert!(
                        seg.ends_with('\n') || seg.ends_with('\r') || seg.ends_with('\u{85}'),
                        "seg: {seg:#?}"
                    );
                    // the break was a '\n', '\r' or NEL
                    offset - 1
                };

                if break_start > start {
                    // the segment has more characters than the line-break character(s).
                    Self::push_seg(text_str, &bidi, &mut segs, break_start);
                }
                if break_start < offset {
                    // the line break character(s).
                    segs.push(TextSegment {
                        kind: TextSegmentKind::LineBreak,
                        end: offset,
                        level: bidi.levels[break_start],
                    })
                }
            } else {
                // is a soft-break, an opportunity to break the line if needed
                Self::push_seg(text_str, &bidi, &mut segs, offset);
            }
        }
        SegmentedText {
            text,
            segments: segs,
            base_direction,
        }
    }
    fn push_seg(text: &str, bidi: &BidiInfo, segs: &mut Vec<TextSegment>, end: usize) {
        let start = segs.last().map(|s| s.end).unwrap_or(0);

        let mut kind = TextSegmentKind::LeftToRight;
        let mut level = BidiLevel::ltr();
        for (i, c) in text[start..end].char_indices() {
            let c_kind = match TextSegmentKind::from(bidi.original_classes[start + i]) {
                TextSegmentKind::OtherNeutral if super::unicode_bidi_util::bidi_bracket_data(c).is_some() => TextSegmentKind::Bracket(c),
                k => k,
            };
            let c_level = bidi.levels[start + i];

            if c_kind != kind || c_level != level || !c_kind.can_merge() {
                if i > 0 {
                    segs.push(TextSegment {
                        kind,
                        end: i + start,
                        level,
                    });
                }
                level = c_level;
                kind = c_kind;
            }
        }
        segs.push(TextSegment { kind, end, level });
    }

    /// The text string.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The raw segment data.
    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
    }

    /// Contextual direction.
    ///
    /// Note that each segment can override the direction, and even the entire text can be a sequence in
    /// the opposite direction.
    pub fn base_direction(&self) -> LayoutDirection {
        self.base_direction
    }

    /// Returns the text segment and kind if `index` is in bounds.
    pub fn get(&self, index: usize) -> Option<(&str, TextSegment)> {
        if let Some(&seg) = self.segments.get(index) {
            let text = if index == 0 {
                &self.text[..seg.end]
            } else {
                &self.text[self.segments[index - 1].end..seg.end]
            };

            Some((text, seg))
        } else {
            None
        }
    }

    /// Returns a clone of the text segment if `index` is in bounds.
    pub fn get_clone(&self, index: usize) -> Option<SegmentedText> {
        self.get(index).map(|(txt, seg)| SegmentedText {
            text: txt.to_owned().into(),
            segments: vec![TextSegment { end: txt.len(), ..seg }],
            base_direction: self.base_direction,
        })
    }

    /// Returns the number of segments.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Returns `true` if text and segments are empty.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Destructs `self` into the text and segments.
    pub fn into_parts(self) -> (Txt, Vec<TextSegment>, LayoutDirection) {
        (self.text, self.segments, self.base_direction)
    }

    /// New segmented text from [parts](Self::into_parts).
    ///
    /// # Panics
    ///
    /// Some basic validation is done on the input:
    ///
    /// * If one of the inputs is empty but the other is not.
    /// * If text is not empty and the last segment does not end with the text.
    pub fn from_parts(text: Txt, segments: Vec<TextSegment>, base_direction: LayoutDirection) -> Self {
        assert_eq!(text.is_empty(), segments.is_empty());
        if !text.is_empty() {
            assert!(segments.last().unwrap().end == text.len());
        }

        SegmentedText {
            text,
            segments,
            base_direction,
        }
    }

    /// Segments iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui_core::text::SegmentedText;
    /// # use zero_ui_core::context::LayoutDirection;
    /// for (sub_str, seg) in SegmentedText::new("Foo bar!\nBaz.", LayoutDirection::LTR).iter() {
    ///     println!("s: {sub_str:?} is a `{:?}`", seg.kind);
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
    pub fn iter(&self) -> SegmentedTextIter {
        SegmentedTextIter {
            text: &self.text,
            start: 0,
            segs_iter: self.segments.iter(),
        }
    }

    /// Convert a segments range to a text bytes range.
    pub fn text_range(&self, segs_range: ops::Range<usize>) -> ops::Range<usize> {
        let start = if segs_range.start == 0 {
            0
        } else {
            self.segments[segs_range.start - 1].end
        };
        let end = self.segments[..segs_range.end].last().map(|s| s.end).unwrap_or(0);
        start..end
    }

    /// Compute a map of segments in `segs_range` to their final LTR display order.
    ///
    /// The `segs_range` must be the segments of a line after line wrap.
    pub fn reorder_line_to_ltr(&self, segs_range: ops::Range<usize>) -> Vec<usize> {
        let mut r = Vec::with_capacity(segs_range.len());
        let offset = segs_range.start;
        unicode_bidi_sort(
            self.base_direction,
            self.segments[segs_range].iter().map(|s| (s.kind, s.level)),
            offset,
            &mut r,
        );
        r
    }

    /// Find the nearest next char boundary from the byte index `i`.
    ///
    /// If `i` is larger than the text length, returns the text length, if `i` is
    /// already a char boundary, returns `i`.
    pub fn snap_char_boundary(&self, i: usize) -> usize {
        if i >= self.text.len() {
            self.text.len()
        } else {
            let mut next = i;
            while !self.text.is_char_boundary(next) {
                next += 1;
            }
            next
        }
    }

    /// Find the nearest grapheme cluster boundary from the byte index `i`.
    ///
    /// If `i` is larger than the text length, returns the text length, if `i` is
    /// already a grapheme boundary, returns `i`.
    pub fn snap_grapheme_boundary(&self, i: usize) -> usize {
        let from = self.snap_char_boundary(i);
        if from == self.text.len() {
            from
        } else {
            let s = &self.text.as_str()[from..];
            let mut iter = unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true).map(|(i, _)| i + from);
            iter.next().unwrap_or(self.text.len())
        }
    }

    /// Find the next grapheme cluster, after `from`.
    ///
    /// The `from` must be in a grapheme boundary or `0` or `len`. This operation is saturating.
    ///
    /// # Panics
    ///
    /// Panics if `from` is larger than the text length, or is not at a grapheme boundary.
    pub fn next_insert_index(&self, from: usize) -> usize {
        if from == self.text.len() {
            from
        } else {
            let s = &self.text.as_str()[from..];
            let mut iter = unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true).map(|(i, _)| i + from);
            assert_eq!(iter.next(), Some(from), "`from` was not a grapheme boundary");
            iter.next().unwrap_or(self.text.len())
        }
    }

    /// Find the previous grapheme cluster, before `from`.
    ///
    /// The `from` must be in a grapheme boundary or `0` or `len`. This operation is saturating.
    ///
    /// # Panics
    ///
    /// Panics if `from` is larger than the text length, or is not at a grapheme boundary.
    pub fn prev_insert_index(&self, from: usize) -> usize {
        if from == self.text.len() {
            let s = &self.text.as_str()[..from];
            let mut iter = unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true)
                .map(|(i, _)| i)
                .rev();
            iter.next().unwrap_or(0)
        } else {
            let s = self.text.as_str();

            // from + 1_char, so that the `from` is the first yield in reverse if it is a valid grapheme boundary
            let inclusive_from = s[from..]
                .char_indices()
                .skip(1)
                .next()
                .map(|(b, _)| from + b)
                .unwrap_or_else(|| s.len());

            let s = &self.text.as_str()[..inclusive_from];
            let mut iter = unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true)
                .map(|(i, _)| i)
                .rev();
            assert_eq!(iter.next(), Some(from), "`from` was not a grapheme boundary");
            iter.next().unwrap_or(0)
        }
    }
}

/// Compute initial bidirectional levels of each segment of a `line`.
///
/// The result is set in `levels`.
pub fn unicode_bidi_levels(base_direction: LayoutDirection, line: impl Iterator<Item = TextSegmentKind>, levels: &mut Vec<BidiLevel>) {
    let mut original_classes = Vec::with_capacity(line.size_hint().0);
    let mut brackets = FxHashMap::default();
    for (i, k) in line.enumerate() {
        original_classes.push(k.into());
        if let TextSegmentKind::Bracket(c) = k {
            brackets.insert(i, c);
        }
    }

    unicode_bidi_levels_impl(levels, base_direction, original_classes, brackets);
}
fn unicode_bidi_levels_impl(
    levels: &mut Vec<BidiLevel>,
    base_direction: LayoutDirection,
    original_classes: Vec<unicode_bidi::BidiClass>,
    brackets: FxHashMap<usize, char>,
) {
    levels.clear();
    let para_level = BidiLevel::from(base_direction);
    levels.resize(original_classes.len(), para_level);

    if !original_classes.is_empty() {
        let mut processing_classes = original_classes.clone();

        super::unicode_bidi_util::explicit_compute(para_level, &original_classes, levels, &mut processing_classes);

        let sequences = super::unicode_bidi_util::prepare_isolating_run_sequences(para_level, &original_classes, levels);
        for sequence in &sequences {
            super::unicode_bidi_util::implicit_resolve_weak(sequence, &mut processing_classes);
            super::unicode_bidi_util::implicit_resolve_neutral(sequence, levels, &original_classes, &mut processing_classes, &brackets);
        }
        super::unicode_bidi_util::implicit_resolve_levels(&processing_classes, levels);

        super::unicode_bidi_util::assign_levels_to_removed_chars(para_level, &original_classes, levels);
    }
}

/// Compute a map of segments in `line` to their final LTR display order.
///
/// The result is set in `sort_map`.
pub fn unicode_bidi_sort(
    base_direction: LayoutDirection,
    line: impl Iterator<Item = (TextSegmentKind, BidiLevel)>,
    idx_offset: usize,
    sort_map: &mut Vec<usize>,
) {
    sort_map.clear();

    let cap = line.size_hint().0;
    let mut line_classes = Vec::with_capacity(cap);
    let mut levels = Vec::with_capacity(cap);
    for (kind, level) in line {
        line_classes.push(kind.into());
        levels.push(level);
    }

    if !levels.is_empty() {
        let (directions, vis_ranges) = super::unicode_bidi_util::visual_runs(levels, line_classes, base_direction.into());

        for vis_range in vis_ranges {
            if directions[vis_range.start].is_rtl() {
                for i in vis_range.rev() {
                    sort_map.push(idx_offset + i);
                }
            } else {
                for i in vis_range {
                    sort_map.push(idx_offset + i);
                }
            }
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
    type Item = (&'a str, TextSegment);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&seg) = self.segs_iter.next() {
            let r = Some((&self.text[self.start..seg.end], seg));
            self.start = seg.end;
            r
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{context::LayoutDirection, text::*};

    #[test]
    fn segments() {
        let test = "a\nb\r\nc\td ";
        let actual = SegmentedText::new(test, LayoutDirection::LTR);

        fn seg(kind: TextSegmentKind, end: usize) -> TextSegment {
            TextSegment {
                kind,
                end,
                level: BidiLevel::ltr(),
            }
        }
        use TextSegmentKind::*;

        let expected = SegmentedText {
            text: test.to_text(),
            segments: vec![
                seg(LeftToRight, 1),
                seg(LineBreak, 2),
                seg(LeftToRight, 3),
                seg(LineBreak, 5),
                seg(LeftToRight, 6),
                seg(Tab, 7),
                seg(LeftToRight, 8),
                seg(Space, 9),
            ],
            base_direction: LayoutDirection::LTR,
        };

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn reorder_line() {
        let test = "0 2 4";
        let txt = SegmentedText::new(test, LayoutDirection::RTL);

        let expected = vec![4, 3, 2, 1, 0];
        let actual = txt.reorder_line_to_ltr(0..test.len());

        pretty_assertions::assert_eq!(expected, actual);
    }

    #[test]
    fn reorder_line_issue() {
        let test = "      المادة 1";
        let txt = SegmentedText::new(test, LayoutDirection::RTL);

        let expected = vec![3, 2, 1, 0];
        let actual = txt.reorder_line_to_ltr(0..4);

        assert_eq!(expected, actual);
    }
}
