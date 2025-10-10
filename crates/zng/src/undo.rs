#![cfg(feature = "undo")]

//! Undo service, commands and other types.
//!
//! The [`UNDO`] service can be used to operate the contextual undo stack, you can also use the service
//! to implement undo/redo for any variable using [`UNDO.watch_var`]. The [`UNDO_CMD`] and [`REDO_CMD`]
//! commands can be used with [`undo_scoped`] to control the focused undo scope. The [`history::UndoHistory!`]
//! widget visualizes the undo or redo stack of the focused undo scope. The example below demonstrates all
//! of this together to define two widgets that undo and redo and shows the history in a drop-down.
//!
//! [`UNDO.watch_var`]: UNDO::watch_var
//! [`undo_scoped`]: CommandUndoExt::undo_scoped
//! [`history::UndoHistory!`]: struct@history::UndoHistory
//!
//! ```
//! use zng::prelude::*;
//!
//! fn undo_combo(op: zng::undo::UndoOp) -> UiNode {
//!     let cmd = op.cmd().undo_scoped();
//!
//!     Toggle! {
//!         style_fn = toggle::ComboStyle!();
//!
//!         widget::enabled = cmd.flat_map(|c| c.is_enabled());
//!
//!         child = Button! {
//!             child = cmd.flat_map(|c| c.icon()).present_data(());
//!             child_right = Text!(cmd.flat_map(|c| c.name())), 4;
//!             tooltip = Tip!(Text!(cmd.flat_map(|c| c.name_with_shortcut())));
//!             on_click = hn!(|a| {
//!                 a.propagation().stop();
//!                 cmd.get().notify();
//!             });
//!         };
//!
//!         checked_popup = wgt_fn!(|_| popup::Popup! {
//!             child = zng::undo::history::UndoHistory!(op);
//!         });
//!     }
//! }
//!
//! # fn example() {
//! # let _ =
//! Wrap! {
//!     spacing = 5;
//!     zng::focus::alt_focus_scope = true;
//!     children = ui_vec![undo_combo(zng::undo::UndoOp::Undo), undo_combo(zng::undo::UndoOp::Redo),];
//! }
//! # ; }
//! ```
//!
//! # Full API
//!
//! See [`zng_ext_undo`] for the full undo API.

pub use zng_ext_undo::{
    CLEAR_HISTORY_CMD, CommandUndoExt, REDO_CMD, RedoAction, UNDO, UNDO_CMD, UndoAction, UndoActionMergeArgs, UndoFullOp, UndoInfo, UndoOp,
    UndoSelect, UndoSelectInterval, UndoSelectLtEq, UndoSelector, UndoStackInfo, UndoTransaction, UndoVarModifyTag, WidgetInfoUndoExt,
    WidgetUndoScope,
};

pub use zng_wgt_undo::{UndoMix, undo_enabled, undo_interval, undo_limit, undo_scope};

/// Undo history widget.
///
/// # Full API
///
/// See [`zng_wgt_undo_history`] for the full undo API.
pub mod history {
    pub use zng_wgt_undo_history::{
        UndoEntryArgs, UndoHistory, UndoPanelArgs, UndoRedoButtonStyle, UndoStackArgs, group_by_undo_interval, is_cap_hovered_timestamp,
        undo_redo_button_style_fn,
    };
}
