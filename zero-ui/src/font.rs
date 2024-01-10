//! Fonts service and text shaping.
//!
//! # Full API
//!
//! See [`zero_ui_ext_font`] for the full font and shaping API.

pub use zero_ui_ext_font::{
    font_features, unicode_bidi_levels, unicode_bidi_sort, BidiLevel, CaretIndex, ColorGlyph, ColorGlyphs, ColorPalette, ColorPaletteType,
    ColorPalettes, CustomFont, Font, FontChange, FontChangedArgs, FontColorPalette, FontDataRef, FontFace, FontFaceList, FontFaceMetrics,
    FontList, FontMetrics, FontName, FontNames, FontSize, FontStretch, FontStyle, FontWeight, Hyphenation, HyphenationDataDir,
    HyphenationDataSource, Hyphens, Justify, LayoutDirections, LetterSpacing, LineBreak, LineHeight, LineSpacing, OutlineHintingOptions,
    OutlineSink, ParagraphSpacing, SegmentedText, SegmentedTextIter, ShapedColoredGlyphs, ShapedLine, ShapedSegment, ShapedText, TabLength,
    TextLineThickness, TextOverflowInfo, TextSegment, TextSegmentKind, TextShapingArgs, TextTransformFn, UnderlineThickness, WhiteSpace,
    WordBreak, WordSpacing, FONTS, FONT_CHANGED_EVENT,
};
