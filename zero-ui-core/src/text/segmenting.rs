use super::Text;
use xi_unicode::LineBreakIterator;

/// The type of a [text segment](SegmentedText).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TextSegmentKind {
    /// A sequence of characters that cannot be separated by a line-break.
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
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SegmentedText {
    text: Text,
    segments: Vec<TextSegment>,
}
impl SegmentedText {
    /// New segmented text from any text type.
    #[inline]
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
        SegmentedText { text, segments: segs }
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
    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
    }

    /// Returns the text segment and kind if `index` is in bounds.
    pub fn get(&self, index: usize) -> Option<(&str, TextSegmentKind)> {
        if let Some(seg) = self.segments.get(index) {
            let text = if index == 0 {
                &self.text[..seg.end]
            } else {
                &self.text[self.segments[index - 1].end..seg.end]
            };

            Some((text, seg.kind))
        } else {
            None
        }
    }

    /// Returns a clone of the text segment if `index` is in bounds.
    pub fn get_clone(&self, index: usize) -> Option<SegmentedText> {
        self.get(index).map(|(s, k)| SegmentedText {
            text: s.to_owned().into(),
            segments: vec![TextSegment { kind: k, end: s.len() }],
        })
    }

    /// Returns the number of segments.
    #[inline]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Returns `true` if text and segments are empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Destructs `self` into the text and segments.
    #[inline]
    pub fn into_parts(self) -> (Text, Vec<TextSegment>) {
        (self.text, self.segments)
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

        SegmentedText { text, segments }
    }

    /// Segments iterator.
    ///
    /// # Example
    ///
    /// ```
    /// # use zero_ui_core::text::SegmentedText;
    /// for (sub_str, segment_kind) in SegmentedText::new("Foo bar!\nBaz.").iter() {
    ///     println!("s: {sub_str:?} is a `{segment_kind:?}`");
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
            segs_iter: self.segments.iter(),
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

#[cfg(test)]
mod tests {
    use crate::text::*;

    #[test]
    fn segments() {
        let test = "a\nb\r\nc\td ";
        let actual = SegmentedText::new(test);

        fn seg(kind: TextSegmentKind, end: usize) -> TextSegment {
            TextSegment { kind, end }
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
        };

        pretty_assertions::assert_eq!(expected, actual);
    }
}
