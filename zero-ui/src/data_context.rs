//! Data context types.
//!
//! # Full API
//!
//! See [`zero_ui_wgt_data`] for the full API.

pub use zero_ui_wgt_data::{
    data, data_error, data_error_color, data_info, data_info_color, data_note, data_warn, data_warn_color, extend_data_note_colors,
    get_data_error, get_data_error_txt, get_data_info, get_data_info_txt, get_data_notes, get_data_notes_top, get_data_warn,
    get_data_warn_txt, has_data_error, has_data_info, has_data_notes, has_data_warn, replace_data_note_colors, with_data_note_color,
    DataNote, DataNoteHandle, DataNoteLevel, DataNoteValue, DataNotes, DATA, DATA_NOTE_COLORS_VAR,
};
