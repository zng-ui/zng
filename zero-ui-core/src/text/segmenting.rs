use std::ops;

use crate::context::LayoutDirection;

use super::Text;
use unicode_bidi::BidiInfo;
use xi_unicode::LineBreakIterator;

pub use unicode_bidi::{BidiClass, Level as BidiLevel};

/// The type of a [text segment](SegmentedText).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TextSegmentKind {
    /// A sequence of characters that cannot be separated by a line-break and are all in the same direction.
    Word,
    /// A sequence of characters that all have the `White_Space` Unicode property, except the [`Tab`](Self::Tab) and
    ///[`LineBreak`](Self::LineBreak) characters.
    Space,
    /// A sequence of `U+0009 TABULAR` characters.
    Tab,
    /// A single line-break, `\n` or `\r\n`.
    LineBreak,
}

/// Represents a single text segment in a [`SegmentedText`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSegment {
    /// Segment kind.
    pub kind: TextSegmentKind,
    /// Direction of the glyphs in the segment and how it advances given the context of .
    ///
    /// Segments iterate in the logical order, that is, the order the text is typed. If two segments
    /// in the same line have direction `RTL` they must be layout the first to the right of the second.
    pub direction: LayoutDirection,

    /// Exclusive end index on the source text.
    ///
    /// The segment range starts from the `end` of the previous segment, or `0`, e.g: `prev_seg.end..self.end`.
    pub end: usize,
}

/// A string segmented in sequences of words, spaces, tabs and separated line breaks.
///
/// Each segment is tagged with a [`TextSegmentKind`] and is represented as
/// an offset from the last segment.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SegmentedText {
    text: Text,
    segments: Vec<TextSegment>,
    base_direction: LayoutDirection,
}
impl SegmentedText {
    /// New segmented text from any text type.
    pub fn new(text: impl Into<Text>, base_direction: LayoutDirection) -> Self {
        Self::new_text(text.into(), base_direction)
    }
    fn new_text(text: Text, base_direction: LayoutDirection) -> Self {
        let mut segs: Vec<TextSegment> = vec![];
        let text_str: &str = &text;
        let bidi = BidiInfo::new(text_str, Some(base_direction.into()));

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
                    debug_assert!(seg.ends_with('\n') || seg.ends_with('\r'), "seg: {:#?}", seg);
                    // the break was a '\n' or just '\r'
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
                        direction: bidi.levels[break_start].into(),
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

        let mut kind = TextSegmentKind::Word;
        let mut direction = LayoutDirection::LTR;
        for (i, c) in text[start..end].char_indices() {
            let (c_kind, c_direction) = if c == '\t' {
                (TextSegmentKind::Tab, direction)
            } else if ['\u{0020}', '\u{000a}', '\u{000c}', '\u{000d}'].contains(&c) {
                (TextSegmentKind::Space, direction)
            } else {
                (TextSegmentKind::Word, bidi.levels[start + i].into())
            };

            if c_kind != kind || c_direction != direction {
                if i > 0 {
                    segs.push(TextSegment {
                        kind,
                        end: i + start,
                        direction,
                    });
                }
                direction = c_direction;
                kind = c_kind;
            }
        }
        segs.push(TextSegment { kind, end, direction });
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
    pub fn into_parts(self) -> (Text, Vec<TextSegment>, LayoutDirection) {
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
    pub fn from_parts(text: Text, segments: Vec<TextSegment>, base_direction: LayoutDirection) -> Self {
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
        let segs = &self.segments[segs_range.clone()];
        let txt_range = self.text_range(segs_range.clone());
        let txt = &self.text[txt_range.clone()];

        let bidi = BidiInfo::new(txt, Some(self.base_direction.into()));

        let mut r = Vec::with_capacity(segs_range.len());

        let (levels, ranges) = bidi.visual_runs(&bidi.paragraphs[0], bidi.paragraphs[0].range.clone());
        for vis_txt_range in ranges {
            let is_rtl = levels[vis_txt_range.start].is_rtl();
            let vis_txt_range = (txt_range.start + vis_txt_range.start)..(txt_range.start + vis_txt_range.end);
            let mut seg_txt_start = txt_range.start;

            let rtl_insert_i = r.len();
            for (seg_i, seg) in segs.iter().enumerate() {
                let seg_txt_range = seg_txt_start..seg.end;
                if vis_txt_range.contains(&seg_txt_range.start) {
                    if is_rtl {
                        r.insert(rtl_insert_i, segs_range.start + seg_i);
                    } else {
                        r.push(segs_range.start + seg_i);
                    }
                }
                seg_txt_start = seg.end;
            }
        }

        r
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
                direction: LayoutDirection::LTR,
            }
        }
        use TextSegmentKind::*;

        let expected = SegmentedText {
            text: test.to_text(),
            segments: vec![
                seg(Word, 1),
                seg(LineBreak, 2),
                seg(Word, 3),
                seg(LineBreak, 5),
                seg(Word, 6),
                seg(Tab, 7),
                seg(Word, 8),
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
