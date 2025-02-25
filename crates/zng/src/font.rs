//! Fonts service and text shaping.
//!
//! The most common types in this module are used through the [`Text!`] widget properties related to font configuration.
//!
//! ```
//! use zng::{prelude::*, font::FontName};
//!
//! # let _scope = APP.defaults();
//! # let _ =
//! Text! {
//!     txt = "hello";
//!     font_family = FontName::monospace();
//! }
//! # ;
//! ```
//!
//! Internally the [`Text!`] widget implements text segmenting and shaping using the types provided by this module,
//! but you only need to interact with these types directly if you are authoring new text properties or a new custom
//! text rendering widget.
//!
//! The second most common type used is the [`FONTS`] service. The service can be used to register custom fonts, query system fonts and
//! manage the font cache.
//!
//! # Fonts Service
//!
//! The example below demonstrates a font query and custom embedded font installation.
//!
//! ```
//! # macro_rules! include_bytes { ($tt:tt) => { &[] } }
//! # use zng::{prelude::*, font::*, l10n::*};
//! # fn main() { }
//! /// set custom fallback font for the ⌫ symbol.
//! async fn set_fallback_font() {
//!     use zng::font::*;
//!     let und = lang!(und);
//!
//!     let shaped_icon = FONTS
//!         .list(
//!             &FontNames::system_ui(&und),
//!             FontStyle::Normal,
//!             FontWeight::NORMAL,
//!             FontStretch::NORMAL,
//!             &und,
//!         )
//!         .wait_rsp()
//!         .await
//!         .sized(layout::Px(11), vec![])
//!         .shape_text(&SegmentedText::new("⌫", layout::LayoutDirection::LTR), &TextShapingArgs::default());
//!
//!     if shaped_icon.is_empty() || shaped_icon.glyphs().flat_map(|g| g.1).any(|g| g.index == 0) {
//!         // OS UI and fallback fonts do not support `⌫`, load custom font that does.
//!
//!         static FALLBACK: &[u8] = include_bytes!("res/calculator/notosanssymbols2-regular-subset.ttf");
//!         let fallback = CustomFont::from_bytes("fallback", FontDataRef::from_static(FALLBACK), 0);
//!
//!         FONTS.register(fallback).wait_rsp().await.unwrap();
//!         FONTS.generics().set_fallback(und, "fallback");
//!     }
//! }
//! ```
//!
//! This code is taken from the `examples/calculator.rs` example,
//! it uses [`FONTS.list`](FONTS::list) to get the font [`system_ui`](FontNames::system_ui) fonts that are used by default. The code
//! then checks if any of system fonts has a glyph for the `⌫` character, if none of the fonts support it a [`CustomFont`] is
//! loaded from an embedded font and installed using [`FONTS.register`](FONTS::register). Finally the [`FONTS.generics`](FONTS::generics)
//! is used to override the fallback font.
//!
//! The `FONTS.generics` can also be used to change what font is used for the specially named fonts like [`FontName::sans_serif`].
//!
//! # Text Segmenting and Shaping
//!
//! The most advance feature provided by this module is text segmenting and shaping. Text segmenting is the process of analyzing
//! raw text and splitting it into distinct segments that define things like the layout direction of text runs, words and spaces,
//! points where text can be inserted and where wrap line-breaks can happen, this is defined the type [`SegmentedText`].
//! A segmented text can then be shaped, that is actual glyphs resolved for each segment and positioned according to available space,
//! this is defined by the [`ShapedText`] type.
//!
//! The example below segments and shapes a text, generating a markdown report from some of the data computed.
//!
//! ```
//! # fn main() { }
//! use std::fmt::Write as _;
//! use zng::{font::*, l10n::Lang, prelude_wgt::Px, text::*, var::Var};
//!
//! async fn report_segment_and_glyphs(txt: &str, lang: &Lang) -> Txt {
//!     let mut report = formatx!("# Shape & Segment\n\n{txt}\n\n");
//!
//!     // start font query in parallel
//!     let font_face = FONTS.list(
//!         &FontNames::system_ui(lang),
//!         FontStyle::Normal,
//!         FontWeight::NORMAL,
//!         FontStretch::NORMAL,
//!         lang,
//!     );
//!
//!     // segment text
//!     let segmented_txt = SegmentedText::new(Txt::from_str(txt), lang.direction());
//!
//!     write!(&mut report, "### Segments\n\n|text|kind|\n|--|--|\n").unwrap();
//!     for (txt, seg) in segmented_txt.iter() {
//!         writeln!(&mut report, "|{txt:?}|{:?}|", seg.kind).unwrap();
//!     }
//!
//!     // wait font query
//!     let font = font_face.wait_into_rsp().await;
//!     // gets the best font for the size
//!     let font = font.sized(Px(20), vec![]);
//!
//!     write!(&mut report, "### Fonts\n\n").unwrap();
//!     let mut sep = "";
//!     for f in font.iter() {
//!         write!(&mut report, "{sep}{}", f.face().family_name()).unwrap();
//!         sep = ", ";
//!     }
//!     writeln!(&mut report, "\n").unwrap();
//!
//!     // shape text
//!     let shaped_txt = font.shape_text(
//!         &segmented_txt,
//!         &TextShapingArgs {
//!             lang: lang.clone(),
//!             direction: segmented_txt.base_direction(),
//!             line_height: font.best().metrics().line_height(),
//!             ..TextShapingArgs::default()
//!         },
//!     );
//!
//!     write!(&mut report, "### Glyphs\n\n|text|glyphs|\n|--|--|\n").unwrap();
//!     for line in shaped_txt.lines() {
//!         for seg in line.segs() {
//!             let txt = seg.text(txt);
//!             write!(&mut report, "|{txt:?}|").unwrap();
//!             let mut sep = "";
//!             for (font, glyphs) in seg.glyphs() {
//!                 write!(&mut report, "{sep}**{}** ", font.face().family_name(),).unwrap();
//!                 sep = " | ";
//!
//!                 let mut sep = "";
//!                 for g in glyphs {
//!                     write!(&mut report, "{sep}{}", g.index).unwrap();
//!                     sep = ", ";
//!                 }
//!             }
//!             writeln!(&mut report).unwrap();
//!         }
//!     }
//!
//!     report
//! }
//! ```
//!
//! Note that you can access the segmented and shaped text of a [`Text!`] widget using the [`TEXT`] service.
//!
//! [`Text!`]: struct@crate::text::Text
//! [`TEXT`]: struct@crate::text::TEXT
//!
//! # Full API
//!
//! See [`zng_ext_font`] for the full font and shaping API.

pub use zng_ext_font::{
    BidiLevel, CaretIndex, ColorGlyph, ColorGlyphs, ColorPalette, ColorPaletteType, ColorPalettes, CustomFont, FONT_CHANGED_EVENT, FONTS,
    Font, FontChange, FontChangedArgs, FontColorPalette, FontDataRef, FontFace, FontFaceList, FontFaceMetrics, FontList, FontMetrics,
    FontName, FontNames, FontSize, FontStretch, FontStyle, FontWeight, HYPHENATION, HyphenationDataDir, HyphenationDataSource, Hyphens,
    Justify, LayoutDirections, LetterSpacing, LineBreak, LineHeight, LineSpacing, OutlineSink, ParagraphSpacing, SegmentedText,
    SegmentedTextIter, ShapedColoredGlyphs, ShapedLine, ShapedSegment, ShapedText, TabLength, TextLineThickness, TextOverflowInfo,
    TextSegment, TextSegmentKind, TextShapingArgs, TextTransformFn, UnderlineThickness, WhiteSpace, WordBreak, WordSpacing, font_features,
    unicode_bidi_levels, unicode_bidi_sort,
};
