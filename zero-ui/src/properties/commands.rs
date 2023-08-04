//! Common commands.
//!

use crate::core::clipboard::*;
use crate::core::event::*;
use crate::core::gesture::{shortcut, CommandShortcutExt};
use crate::prelude::new_property::*;

command! {
    /// Represents the **new** action.
    ///
    /// The command parameter can identify the new item type, otherwise the default (or single) type
    /// must be used.
    pub static NEW_CMD = {
        name: "New",
        shortcut: [shortcut!(CTRL+'N')],
    };

    /// Represents the **open** action.
    ///
    /// The command parameter can be an item path to open (like a `PathBuf`), otherwise the
    /// command implementer must identify the item, either by context or by prompting the user.
    pub static OPEN_CMD = {
        name: "Open…",
        shortcut: [shortcut!(CTRL+'O')],
    };

    /// Represents the **save** action.
    ///
    /// Usually this saves to the already defined item path (open or previous save path),
    /// otherwise the user is prompted like [`SAVE_AS_CMD`].
    pub static SAVE_CMD = {
        name: "Save",
        shortcut: [shortcut!(CTRL+'S')],
    };

    /// Represents the **save-as** action.
    ///
    /// Usually this prompts the user for a save path, even if a previous path is already known.
    pub static SAVE_AS_CMD = {
        name: "Save As…",
        shortcut: [shortcut!(CTRL|SHIFT+'S')],
    };

    /// Represents the **context menu open** action.
    pub static CONTEXT_MENU_CMD = {
        shortcut: [shortcut!(SHIFT+F10), shortcut!(ContextMenu)],
    };
}

command_property! {
    /// New command handler.
    ///
    /// Receives [`NEW_CMD`] command events scoped on the widget. The command parameter can be
    /// the new item type identifier.
    pub fn new {
        cmd: NEW_CMD.scoped(WIDGET.id()),
    }

    /// Open command handler.
    ///
    /// Receives [`OPEN_CMD`] command events scoped on the widget. The command parameter can be
    /// a path to open, otherwise the path must be derived from context or the user prompted.
    ///
    /// You can use [`WINDOWS.native_file_dialog`] to prompt the user for a file or folder path.
    ///
    /// [`WINDOWS.native_file_dialog`]: WINDOWS::native_file_dialog
    pub fn open {
        cmd: OPEN_CMD.scoped(WIDGET.id()),
    }

    /// Save command handler.
    ///
    /// Receives [`SAVE_CMD`] command events scoped on the widget. Usually saves to the last
    /// open or save path, otherwise prompt the user like [`on_save_as`].
    ///
    /// You can use [`WINDOWS.native_file_dialog`] to prompt the user for a file or folder path.
    ///
    /// [`WINDOWS.native_file_dialog`]: WINDOWS::native_file_dialog
    /// [`on_save_as`]: fn@on_save_as
    pub fn save {
        cmd: SAVE_CMD.scoped(WIDGET.id()),
    }

    /// Save-As command handler.
    ///
    /// Receives [`SAVE_AS_CMD`] command events scoped on the widget. Usually prompts the user for
    /// a new save path.
    ///
    /// You can use [`WINDOWS.native_file_dialog`] to prompt the user for a file or folder path.
    ///
    /// [`WINDOWS.native_file_dialog`]: WINDOWS::native_file_dialog
    pub fn save_as {
        cmd: SAVE_AS_CMD.scoped(WIDGET.id()),
    }

    /// Cut command handler.
    ///
    /// Receives [`CUT_CMD`] command events scoped on the widget. You can use the [`CLIPBOARD`] service
    /// to send data to the clipboard.
    pub fn cut {
        cmd: CUT_CMD.scoped(WIDGET.id()),
    }

    /// Paste command handler.
    ///
    /// Receives [`COPY_CMD`] command events scoped on the widget. You can use the [`CLIPBOARD`] service
    /// to send data to the clipboard.
    pub fn copy {
        cmd: COPY_CMD.scoped(WIDGET.id()),
    }

    /// Paste command handler.
    ///
    /// Receives [`PASTE_CMD`] command events scoped on the widget. You can use the [`CLIPBOARD`] service
    /// to receive data from the clipboard.
    pub fn paste {
        cmd: PASTE_CMD.scoped(WIDGET.id()),
    }
}
