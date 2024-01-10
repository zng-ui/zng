//! Text widget, properties and types.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_text`] for the full widget API.

pub use zero_ui_txt::*;

pub use zero_ui_wgt_text::{
    accepts_enter, accepts_tab, auto_selection, caret_color, change_stop_delay, cmd, direction, font_aa, font_annotation, font_caps,
    font_char_variant, font_cn_variant, font_color, font_common_lig, font_contextual_alt, font_discretionary_lig, font_ea_width,
    font_family, font_features, font_historical_forms, font_historical_lig, font_jp_variant, font_kerning, font_num_fraction,
    font_num_spacing, font_numeric, font_ornaments, font_palette, font_palette_colors, font_position, font_size, font_stretch, font_style,
    font_style_set, font_stylistic, font_swash, font_synthesis, font_variations, font_weight, get_caret_index, get_caret_status,
    get_chars_count, get_lines_len, get_lines_wrap_count, get_overflow, hyphen_char, hyphens, ime_underline, interactive_caret,
    interactive_caret_visual, is_line_overflown, is_overflown, is_parse_pending, justify, lang, letter_spacing, line_break, line_height,
    line_spacing, max_chars_count,
    node::{set_interactive_caret_spot, TEXT},
    obscure_txt, obscuring_char, on_change_stop, overline, overline_color, paragraph_spacing, selection_color, selection_toolbar,
    selection_toolbar_anchor, selection_toolbar_fn, strikethrough, strikethrough_color, tab_length, txt_align, txt_editable, txt_highlight,
    txt_overflow, txt_overflow_align, underline, underline_color, underline_skip, white_space, word_break, word_spacing, AutoSelection,
    CaretShape, CaretStatus, ChangeStopArgs, ChangeStopCause, Em, FontFeaturesMix, FontMix, InteractiveCaretMode, LangMix, LinesWrapCount,
    ParagraphMix, SelectionToolbarArgs, Strong, Text, TextAlignMix, TextDecorationMix, TextEditMix, TextFillMix, TextOverflow,
    TextSpacingMix, TextTransformMix, TextWrapMix, TxtParseValue, UnderlinePosition, UnderlineSkip, FONT_COLOR_VAR,
};
