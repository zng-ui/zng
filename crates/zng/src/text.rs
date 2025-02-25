//! Text widget, properties and other types.
//!
//! The [`Text!`] widget implements text layout and rendering, it is also the base widget for
//! [`SelectableText!`], [`TextInput!`] and [`label!`]. Text properties are largely contextual,
//! you can set `text::font_size` in any widget to affect all text inside that widget.
//!
//! The `Text!` widget provides *simple* text rendering, that is all text is of the same style and
//! different fonts are only used as fallback. You can implement *rich* text by combining multiple
//! `Text!` and `Wrap!` panels, see the [`wrap`] module docs for an example. Some widgets also parse
//! text and generate the rich text setup automatically, the [`Markdown!`] and [`AnsiText!`] widgets
//! are examples of this.
//!
//! The example below declares two text widgets, one displays a text that requires multiple fonts to render,
//! the other displays debug information about the first.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! let txt = "text テキスト 📋";
//! let font_use = var(vec![]);
//! # let _ =
//! Stack! {
//!     text::font_family = ["Segoe UI", "Yu Gothic UI", "Segoe Ui Emoji", "sans-serif"];
//!     children = ui_vec![
//!         Text! {
//!             font_size = 1.5.em();
//!             txt;
//!             get_font_use = font_use.clone();
//!         },
//!         Text! {
//!             font_size = 0.9.em();
//!             txt = font_use.map(|u| {
//!                 let mut r = Txt::from("");
//!                 for (font, range) in u {
//!                     use std::fmt::Write as _;
//!                     writeln!(&mut r, "{} = {:?}", font.face().family_name(), &txt[range.clone()]).unwrap();
//!                 }
//!                 r.end_mut();
//!                 r
//!             });
//!         },
//!     ];
//!     direction = StackDirection::top_to_bottom();
//!     spacing = 15;
//! }
//! # ;
//! ```
//!
//! Note that the [`font_family`](fn@font_family) is set on the parent widget, both texts have the same
//! font family value because of this, the [`font_size`](fn@font_size) on the other hand is set for
//! each text widget and only affects that widget.
//!
//! [`Text!`]: struct@Text
//! [`SelectableText!`]: struct@crate::selectable::SelectableText
//! [`TextInput!`]: struct@crate::text_input::TextInput
//! [`label!`]: struct@crate::label::Label
//! [`Markdown!`]: struct@crate::markdown::Markdown
//! [`AnsiText!`]: struct@crate::ansi_text::AnsiText
//! [`wrap`]: crate::wrap
//!
//! # Full API
//!
//! See [`zng_wgt_text`] for the full widget API.

pub use zng_txt::*;

pub use zng_wgt_text::{
    AutoSelection, CaretShape, CaretStatus, ChangeStopArgs, ChangeStopCause, Em, FONT_COLOR_VAR, InteractiveCaretMode, LangMix,
    LinesWrapCount, ParagraphMix, SelectionToolbarArgs, Strong, Text, TextOverflow, TxtParseValue, UnderlinePosition, UnderlineSkip,
    accepts_enter, accepts_tab, auto_selection, caret_color, change_stop_delay, cmd, direction, font_aa, font_annotation, font_caps,
    font_char_variant, font_cn_variant, font_color, font_common_lig, font_contextual_alt, font_discretionary_lig, font_ea_width,
    font_family, font_features, font_historical_forms, font_historical_lig, font_jp_variant, font_kerning, font_num_fraction,
    font_num_spacing, font_numeric, font_ornaments, font_palette, font_palette_colors, font_position, font_size, font_stretch, font_style,
    font_style_set, font_stylistic, font_swash, font_synthesis, font_variations, font_weight, get_caret_index, get_caret_status,
    get_chars_count, get_lines_len, get_lines_wrap_count, get_overflow, hyphen_char, hyphens, ime_underline, interactive_caret,
    interactive_caret_visual, is_line_overflown, is_overflown, is_parse_pending, justify_mode, lang, letter_spacing, line_break,
    line_height, line_spacing, max_chars_count,
    node::{TEXT, set_interactive_caret_spot},
    obscure_txt, obscuring_char, on_change_stop, overline, overline_color, paragraph_spacing, selection_color, selection_toolbar,
    selection_toolbar_anchor, selection_toolbar_fn, strikethrough, strikethrough_color, tab_length, txt_align, txt_editable, txt_overflow,
    txt_overflow_align, underline, underline_color, underline_skip, white_space, word_break, word_spacing,
};

#[allow(deprecated)] // avoid breaking change
pub use zng_wgt_text::justify;
