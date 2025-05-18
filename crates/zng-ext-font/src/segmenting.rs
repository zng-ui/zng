use std::{collections::HashMap, ops};

use crate::emoji_util;

use super::Txt;
use unicode_bidi::{BidiDataSource as _, BidiInfo};

use zng_layout::context::LayoutDirection;
pub use zng_layout::context::TextSegmentKind;

pub use unicode_bidi::Level as BidiLevel;

/// Represents a single text segment in a [`SegmentedText`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
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
        from_unic_level(self.level)
    }
}

/// A string segmented in sequences of words, spaces, tabs and separated line breaks.
///
/// Each segment is tagged with a [`TextSegmentKind`] and is defines as
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
        let bidi = BidiInfo::new(text_str, Some(into_unic_level(base_direction)));

        for (offset, kind) in unicode_linebreak::linebreaks(text_str) {
            // a hard-break is a '\n', '\r', "\r\n" or text end.
            if let unicode_linebreak::BreakOpportunity::Mandatory = kind {
                // start of this segment.
                let start = segs.last().map(|s| s.end).unwrap_or(0);

                // The segment can have other characters before the line-break character(s).

                let seg = &text_str[start..offset];

                let break_start = if seg.ends_with("\r\n") {
                    // the break was a "\r\n"
                    offset - 2
                } else if seg.ends_with('\n') || seg.ends_with('\r') || seg.ends_with('\u{85}') {
                    // the break was a '\n', '\r' or NEL
                    offset - 1
                } else {
                    // "break" at end of string
                    debug_assert_eq!(offset, text_str.len());
                    offset
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
            }
            // else soft break, handled by our own segmentation
        }
        SegmentedText {
            text,
            segments: segs,
            base_direction,
        }
    }

    fn push_seg(text: &str, bidi: &BidiInfo, segs: &mut Vec<TextSegment>, end: usize) {
        let start = segs.last().map(|s| s.end).unwrap_or(0);

        let mut char_indices = text[start..end].char_indices().peekable();

        let mut kind = TextSegmentKind::LeftToRight;
        let mut level = BidiLevel::ltr();
        for (i, c) in &mut char_indices {
            const ZWJ: char = '\u{200D}'; // ZERO WIDTH JOINER
            const VS16: char = '\u{FE0F}'; // VARIANT SELECTOR 16 - Emoji
            const CEK: char = '\u{20E3}'; // COMBINING ENCLOSING KEYCAP

            let is_emoji = (kind == TextSegmentKind::Emoji // maybe
                && (
                    c == VS16 // definitely, modifies prev. char into Emoji.
                    || c == CEK // definitely, modified prev. char into keycap style.
                    || c == ZWJ // definitely, ligature with the next Emoji or is ignored.
                    || emoji_util::is_modifier(c) // definitely, has same effect as VS16.
                    || emoji_util::is_component(c) // definitely, ligature data, like flag tags.
                ))
                || (emoji_util::maybe_emoji(c) // maybe
                    && (emoji_util::definitely_emoji(c) // definitely
                        // only if followed by VS16 or modifier
                        || (text[start+i..].chars().nth(1).map(|c| c == VS16 || emoji_util::is_modifier(c)).unwrap_or(false))));

            let (c_kind, c_level) = if is_emoji {
                (TextSegmentKind::Emoji, level)
            } else {
                let k = match TextSegmentKind::from(bidi.original_classes[start + i]) {
                    TextSegmentKind::OtherNeutral if unicode_bidi::HardcodedBidiData.bidi_matched_opening_bracket(c).is_some() => {
                        TextSegmentKind::Bracket(c)
                    }
                    k => k,
                };
                (k, bidi.levels[start + i])
            };

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
    pub fn text(&self) -> &Txt {
        &self.text
    }

    /// The text segments.
    pub fn segs(&self) -> &[TextSegment] {
        &self.segments
    }

    /// Get segment index from a char index.
    pub fn seg_from_char(&self, from: usize) -> usize {
        match self.segments.binary_search_by_key(&from, |s| s.end) {
            Ok(e) => e + 1,
            Err(s) => s,
        }
    }

    /// Contextual direction.
    ///
    /// Note that each segment can override the direction, and even the entire text can be a sequence in
    /// the opposite direction.
    pub fn base_direction(&self) -> LayoutDirection {
        self.base_direction
    }

    /// Gets if the text contains segments not in the base direction.
    pub fn is_bidi(&self) -> bool {
        for seg in self.segments.iter() {
            if seg.direction() != self.base_direction {
                return true;
            }
        }
        false
    }

    /// Returns the text segment if `index` is in bounds.
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
    /// # use zng_ext_font::SegmentedText;
    /// # use zng_layout::context::LayoutDirection;
    /// for (sub_str, seg) in SegmentedText::new("Foo bar!\nBaz.", LayoutDirection::LTR).iter() {
    ///     println!("s: {sub_str:?} is a `{:?}`", seg.kind);
    /// }
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
        let i = self.snap_char_boundary(i);
        if i == self.text.len() {
            i
        } else {
            let mut seg_start = 0;
            for seg in self.segments.iter() {
                if seg.end > i {
                    break;
                }
                seg_start = seg.end;
            }
            let s = &self.text[seg_start..];

            let seg_i = i - seg_start;
            let mut best_before = 0;
            let mut best_after = s.len();
            for (i, _) in unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true) {
                if i > seg_i {
                    best_after = i;
                    break;
                }
                best_before = i;
            }

            let best = if best_after - seg_i > seg_i - best_before {
                best_before
            } else {
                best_after
            };
            seg_start + best
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
            let inclusive_from = s[from..].char_indices().nth(1).map(|(b, _)| from + b).unwrap_or_else(|| s.len());

            let s = &self.text.as_str()[..inclusive_from];
            let mut iter = unicode_segmentation::UnicodeSegmentation::grapheme_indices(s, true)
                .map(|(i, _)| i)
                .rev();
            assert_eq!(iter.next(), Some(from), "`from` was not a grapheme boundary");
            iter.next().unwrap_or(0)
        }
    }

    /// Find the start of the next word or the next line-break segment, after `from`.
    ///
    /// This operation is saturating.
    pub fn next_word_index(&self, from: usize) -> usize {
        let mut segs = self.segments[self.seg_from_char(from)..].iter();

        if let Some(seg) = segs.next() {
            if seg.kind.is_line_break() {
                return seg.end;
            }
            let mut start = seg.end;
            for seg in segs {
                if seg.kind.is_word() || seg.kind.is_line_break() {
                    return start;
                }
                start = seg.end;
            }
        }
        self.text.len()
    }

    /// Find the next word segment end or the next line-break segment end, after `from`.
    ///
    /// This operation is saturating.
    pub fn next_word_end_index(&self, from: usize) -> usize {
        let mut segs = self.segments[self.seg_from_char(from)..].iter();
        if let Some(seg) = segs.next() {
            if seg.kind.is_word() || seg.kind.is_line_break() {
                return seg.end;
            }
            for seg in segs {
                if seg.kind.is_word() || seg.kind.is_line_break() {
                    return seg.end;
                }
            }
        }
        self.text.len()
    }

    /// Find the start of the previous word segment or the previous line-break segment, before `from`.
    ///
    /// This operation is saturating.
    pub fn prev_word_index(&self, from: usize) -> usize {
        let seg_i = self.seg_from_char(from);
        let mut segs = if seg_i < self.segments.len() {
            self.segments[..=seg_i].iter().rev()
        } else {
            self.segs().iter().rev()
        };
        let mut seg_kind = TextSegmentKind::Space;
        for seg in &mut segs {
            if seg.end < from {
                if seg_kind.is_word() || seg.kind.is_line_break() {
                    // last segment start or line-break end
                    return seg.end;
                }
                seg_kind = seg.kind;
                for seg in segs {
                    if seg_kind.is_word() || seg.kind.is_line_break() {
                        // last segment start or line-break end
                        return seg.end;
                    }
                    seg_kind = seg.kind;
                }
                break;
            } else if seg.end == from && seg.kind.is_line_break() {
                // line-break start
                return segs.next().map(|p| p.end).unwrap_or(0);
            }
            seg_kind = seg.kind;
        }
        0
    }

    /// Find the start of the line that contains `from`.
    ///
    /// # Panics
    ///
    /// Panics if `from` is larger than the text length, or is not a char boundary.
    pub fn line_start_index(&self, from: usize) -> usize {
        let line_break = self.text.as_str()[..from]
            .char_indices()
            .rev()
            .find(|(_, c)| "\n\r\u{85}".contains(*c));

        match line_break {
            Some((i, _)) => i + 1,
            None => 0,
        }
    }

    /// Find the end of the line that contains `from`.
    ///
    /// # Panics
    ///
    /// Panics if `from` is larger than the text length, or is not a char boundary.
    pub fn line_end_index(&self, from: usize) -> usize {
        if from == self.text.len() {
            return from;
        }

        let line_break = self.text.as_str()[from..].char_indices().find(|(_, c)| "\n\r\u{85}".contains(*c));

        match line_break {
            Some((i, _)) => from + i,
            None => self.text.len(),
        }
    }

    /// Find the range that must be removed to delete starting by `from` a `count` number of times.
    ///
    /// Delete **Del** action removes the next grapheme cluster, this is different from
    /// [`backspace_range`] that usually only removes one character.
    ///
    /// # Panics
    ///
    /// Panics if `from` is larger than the text length, or is not a grapheme boundary.
    ///
    /// [`backspace_range`]: Self::backspace_range
    pub fn delete_range(&self, from: usize, count: u32) -> std::ops::Range<usize> {
        let mut end = from;
        for _ in 0..count {
            let e = self.next_insert_index(end);
            if e == end {
                break;
            }
            end = e;
        }

        from..end
    }

    /// Find the range that must be removed to backspace before `from` a `count` number of times.
    ///
    /// The character at `from` is not included, only the previous char is selected, with some exceptions,
    /// the selection includes any char before zero-width-joiner (ZWJ), it also includes `\r` before `\n`
    /// and Emoji char before Emoji modifier or variation selector (VS16).
    ///
    /// # Panics
    ///
    /// Panics if `from` is larger than the text length, or is not a char boundary.
    pub fn backspace_range(&self, from: usize, count: u32) -> std::ops::Range<usize> {
        let mut start = from;
        for _ in 0..count {
            let s = self.backspace_start(start);
            if s == start {
                break;
            }
            start = s;
        }
        start..from
    }
    fn backspace_start(&self, from: usize) -> usize {
        let text = &self.text[..from];
        let mut start = from;
        for (i, c) in text.char_indices().rev() {
            start = i;
            match c {
                '\u{200D}' => continue, // ZWJ
                '\n' => {
                    if text[..i].ends_with('\r') {
                        start = i - 1;
                    }
                }
                c if c == '\u{FE0F}' || emoji_util::is_modifier(c) => {
                    // VS16 || Emoji-Modifier
                    if let Some((i, c)) = text[..i].char_indices().next_back() {
                        if emoji_util::maybe_emoji(c) {
                            start = i;
                        }
                    }
                }
                _ => {}
            }
            break;
        }
        start
    }

    /// Find the range that must be removed to backspace words before `from` a `count` number of times.
    ///
    /// The character at `from` is not included, only the previous word is selected.
    pub fn backspace_word_range(&self, from: usize, count: u32) -> std::ops::Range<usize> {
        let mut start = from;
        for _ in 0..count {
            let s = self.prev_word_index(start);
            if s == start {
                break;
            }
            start = s;
        }
        start..from
    }

    /// Find the range that must be removed to delete words starting by `from` a `count` number of times.
    pub fn delete_word_range(&self, from: usize, count: u32) -> std::ops::Range<usize> {
        let mut end = from;
        for _ in 0..count {
            let e = self.next_word_end_index(end);
            if e == end {
                break;
            }
            end = e;
        }

        from..end
    }
}

/// Compute initial bidirectional levels of each segment of a `line`.
///
/// The result is set in `levels`.
pub fn unicode_bidi_levels(base_direction: LayoutDirection, line: impl Iterator<Item = TextSegmentKind>, levels: &mut Vec<BidiLevel>) {
    let mut original_classes = Vec::with_capacity(line.size_hint().0);
    let mut brackets = HashMap::default();
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
    brackets: HashMap<usize, char>,
) {
    levels.clear();
    let para_level = into_unic_level(base_direction);
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
        let (directions, vis_ranges) = super::unicode_bidi_util::visual_runs(levels, line_classes, into_unic_level(base_direction));

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

fn from_unic_level(d: unicode_bidi::Level) -> LayoutDirection {
    if d.is_ltr() { LayoutDirection::LTR } else { LayoutDirection::RTL }
}
fn into_unic_level(d: LayoutDirection) -> unicode_bidi::Level {
    match d {
        LayoutDirection::LTR => unicode_bidi::Level::ltr(),
        LayoutDirection::RTL => unicode_bidi::Level::rtl(),
    }
}

#[cfg(test)]
mod tests {
    use zng_layout::context::{LayoutDirection, TextSegmentKind};
    use zng_txt::ToTxt;

    use crate::{BidiLevel, SegmentedText, TextSegment};

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
            text: test.to_txt(),
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

        assert_eq!(expected, actual);
    }

    #[test]
    fn reorder_line() {
        let test = "0 2 4";
        let txt = SegmentedText::new(test, LayoutDirection::RTL);

        let expected = vec![4, 3, 2, 1, 0];
        let actual = txt.reorder_line_to_ltr(0..test.len());

        assert_eq!(expected, actual);
    }

    #[test]
    fn reorder_line_issue() {
        let test = "      المادة 1";
        let txt = SegmentedText::new(test, LayoutDirection::RTL);

        let expected = vec![3, 2, 1, 0];
        let actual = txt.reorder_line_to_ltr(0..4);

        assert_eq!(expected, actual);
    }

    #[test]
    fn emoji_seg() {
        let test = "'🙎🏻‍♀️'1# 1️⃣#️⃣";
        let txt = SegmentedText::new(test, LayoutDirection::LTR);
        let k: Vec<_> = txt.segs().iter().map(|s| s.kind).collect();

        assert_eq!(
            vec![
                TextSegmentKind::OtherNeutral,       // '
                TextSegmentKind::Emoji,              // 🙎🏻‍♀️
                TextSegmentKind::OtherNeutral,       // '
                TextSegmentKind::EuropeanNumber,     // 1
                TextSegmentKind::EuropeanTerminator, // #
                TextSegmentKind::Space,
                TextSegmentKind::Emoji, // 1️⃣#️⃣
            ],
            k
        );
    }

    #[test]
    fn emoji_issues() {
        let test = "🏴󠁧󠁢󠁥󠁮󠁧󠁿";
        let txt = SegmentedText::new(test, LayoutDirection::LTR);
        for (t, seg) in txt.iter() {
            assert_eq!(seg.kind, TextSegmentKind::Emoji, "text: {t:?}");
        }
    }
}
