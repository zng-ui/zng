//! Common commands.
//!

use zng_ext_clipboard::{COPY_CMD, CUT_CMD, PASTE_CMD};
use zng_wgt::{ICONS, prelude::*};

command! {
    /// Represents the **new** action.
    ///
    /// The command parameter can identify the new item type, otherwise the default (or single) type
    /// must be used.
    pub static NEW_CMD = {
        l10n!: true,
        name: "New",
        shortcut: [shortcut!(CTRL+'N'), shortcut!(New)],
    };

    /// Represents the **open** action.
    ///
    /// The command parameter can be an item path to open (like a `PathBuf`), otherwise the
    /// command implementer must identify the item, either by context or by prompting the user.
    pub static OPEN_CMD = {
        l10n!: true,
        name: "Open…",
        shortcut: [shortcut!(CTRL+'O'), shortcut!(Open)],
        icon: wgt_fn!(|_| ICONS.get("file-open")),
    };

    /// Represents the **save** action.
    ///
    /// Usually this saves to the already defined item path (open or previous save path),
    /// otherwise the user is prompted like [`SAVE_AS_CMD`].
    pub static SAVE_CMD = {
        l10n!: true,
        name: "Save",
        shortcut: [shortcut!(CTRL+'S'), shortcut!(Save)],
        icon: wgt_fn!(|_| ICONS.get("save")),
    };

    /// Represents the **save-as** action.
    ///
    /// Usually this prompts the user for a save path, even if a previous path is already known.
    pub static SAVE_AS_CMD = {
        l10n!: true,
        name: "Save As…",
        shortcut: [shortcut!(CTRL|SHIFT+'S')],
    };

    /// Represents the **context menu open** action.
    pub static CONTEXT_MENU_CMD = {
        shortcut: [shortcut!(SHIFT+F10), shortcut!(ContextMenu)],
        icon: wgt_fn!(|_| ICONS.get(["context-menu", "menu-open"])),
    };

    /// Represents the **open settings** action.
    ///
    /// Settings is an editor for a selection of app configs.
    ///
    /// # Parameter
    ///
    /// The parameter can be a `Txt` that can match a `ConfigKey` or config metadata
    /// such as the display name or description.
    pub static SETTINGS_CMD = {
        l10n!: true,
        name: "Settings",
        shortcut: [shortcut!(CTRL+',')],
        icon: wgt_fn!(|_| ICONS.get("settings")),
    };
}

command_property! {
    /// On new command.
    ///
    /// Receives [`NEW_CMD`] command events scoped on the widget. The command parameter can be
    /// the new item type identifier.
    pub fn new {
        cmd: NEW_CMD.scoped(WIDGET.id()),
    }

    /// On open command.
    ///
    /// Receives [`OPEN_CMD`] command events scoped on the widget. The command parameter can be
    /// a path to open, otherwise the path must be derived from context or the user prompted.
    ///
    /// You can use [`WINDOWS.native_file_dialog`] to prompt the user for a file or folder path.
    ///
    /// [`WINDOWS.native_file_dialog`]: zng_ext_window::WINDOWS::native_file_dialog
    pub fn open {
        cmd: OPEN_CMD.scoped(WIDGET.id()),
    }

    /// On save command.
    ///
    /// Receives [`SAVE_CMD`] command events scoped on the widget. Usually saves to the last
    /// open or save path, otherwise prompt the user like [`on_save_as`].
    ///
    /// You can use [`WINDOWS.native_file_dialog`] to prompt the user for a file or folder path.
    ///
    /// [`WINDOWS.native_file_dialog`]: zng_ext_window::WINDOWS::native_file_dialog
    /// [`on_save_as`]: fn@on_save_as
    pub fn save {
        cmd: SAVE_CMD.scoped(WIDGET.id()),
    }

    /// On save-as command.
    ///
    /// Receives [`SAVE_AS_CMD`] command events scoped on the widget. Usually prompts the user for
    /// a new save path.
    ///
    /// You can use [`WINDOWS.native_file_dialog`] to prompt the user for a file or folder path.
    ///
    /// [`WINDOWS.native_file_dialog`]: zng_ext_window::WINDOWS::native_file_dialog
    pub fn save_as {
        cmd: SAVE_AS_CMD.scoped(WIDGET.id()),
    }

    /// On cut command.
    ///
    /// Receives [`CUT_CMD`] command events scoped on the widget. You can use the `CLIPBOARD` service
    /// to send data to the clipboard.
    ///
    /// [`CUT_CMD`]: zng_ext_clipboard::CUT_CMD
    pub fn cut {
        cmd: CUT_CMD.scoped(WIDGET.id()),
    }

    /// On copy command.
    ///
    /// Receives [`COPY_CMD`] command events scoped on the widget. You can use the `CLIPBOARD` service
    /// to send data to the clipboard.
    ///
    /// [`COPY_CMD`]: zng_ext_clipboard::COPY_CMD
    pub fn copy {
        cmd: COPY_CMD.scoped(WIDGET.id()),
    }

    /// On paste command.
    ///
    /// Receives [`PASTE_CMD`] command events scoped on the widget. You can use the `CLIPBOARD` service
    /// to receive data from the clipboard.
    ///
    /// [`PASTE_CMD`]: zng_ext_clipboard::PASTE_CMD
    pub fn paste {
        cmd: PASTE_CMD.scoped(WIDGET.id()),
    }

    /// On settings command.
    ///
    /// Receives [`SETTINGS_CMD`] command events scoped on the widget.
    pub fn settings {
        cmd: SETTINGS_CMD.scoped(WIDGET.id()),
    }
}
