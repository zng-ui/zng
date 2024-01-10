//! Undo service, commands and types.
//!
//! # Full API
//!
//! See [`zero_ui_ext_undo`] for the full undo API.

pub use zero_ui_ext_undo::{
    CommandUndoExt, RedoAction, UndoAction, UndoActionMergeArgs, UndoFullOp, UndoInfo, UndoOp, UndoSelect, UndoSelectInterval,
    UndoSelectLtEq, UndoSelector, UndoStackInfo, UndoTransaction, UndoVarModifyTag, WidgetInfoUndoExt, WidgetUndoScope, CLEAR_HISTORY_CMD,
    REDO_CMD, UNDO, UNDO_CMD, UNDO_INTERVAL_VAR, UNDO_LIMIT_VAR,
};

pub use zero_ui_wgt_undo::{undo_enabled, undo_interval, undo_limit, undo_scope, UndoMix};

/// Undo history widget.
///
/// See [`zero_ui_wgt_undo_history`] for the full undo API.
pub mod history {
    pub use zero_ui_wgt_undo_history::{
        extend_undo_button_style, group_by_undo_interval, is_cap_hovered_timestamp, replace_undo_button_style, UndoEntryArgs, UndoHistory,
        UndoPanelArgs, UndoRedoButtonStyle, UndoStackArgs,
    };
}
