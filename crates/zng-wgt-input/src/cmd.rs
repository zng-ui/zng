//! Common commands.
//!

use zng_app::event::CommandArgs;
use zng_ext_clipboard::{COPY_CMD, CUT_CMD, PASTE_CMD};
use zng_wgt::{ICONS, prelude::*};

command! {
    /// Represents the **new** action.
    ///
    /// The command parameter can identify the new item type, otherwise the default (or single) type
    /// must be used.
    pub static NEW_CMD {
        l10n!: true,
        name: "New",
        shortcut: [shortcut!(CTRL + 'N'), shortcut!(New)],
    };

    /// Represents the **open** action.
    ///
    /// The command parameter can be an item path to open (like a `PathBuf`), otherwise the
    /// command implementer must identify the item, either by context or by prompting the user.
    pub static OPEN_CMD {
        l10n!: true,
        name: "Open…",
        shortcut: [shortcut!(CTRL + 'O'), shortcut!(Open)],
        icon: wgt_fn!(|_| ICONS.get("file-open")),
    };

    /// Represents the **save** action.
    ///
    /// Usually this saves to the already defined item path (open or previous save path),
    /// otherwise the user is prompted like [`SAVE_AS_CMD`].
    pub static SAVE_CMD {
        l10n!: true,
        name: "Save",
        shortcut: [shortcut!(CTRL + 'S'), shortcut!(Save)],
        icon: wgt_fn!(|_| ICONS.get("save")),
    };

    /// Represents the **save-as** action.
    ///
    /// Usually this prompts the user for a save path, even if a previous path is already known.
    pub static SAVE_AS_CMD {
        l10n!: true,
        name: "Save As…",
        shortcut: [shortcut!(CTRL | SHIFT + 'S')],
    };

    /// Represents the **context menu open** action.
    pub static CONTEXT_MENU_CMD {
        shortcut: [shortcut!(SHIFT + F10), shortcut!(ContextMenu)],
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
    pub static SETTINGS_CMD {
        l10n!: true,
        name: "Settings",
        shortcut: [shortcut!(CTRL + ',')],
        icon: wgt_fn!(|_| ICONS.get("settings")),
    };
}

command_property! {
    /// On new command.
    ///
    /// Receives [`NEW_CMD`] command events scoped on the widget. The command parameter can be
    /// the new item type identifier.
    #[property(EVENT)]
    pub fn on_new<on_pre_new, can_new>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        NEW_CMD
    }

    /// On open command.
    ///
    /// Receives [`OPEN_CMD`] command events scoped on the widget. The command parameter can be
    /// a path to open, otherwise the path must be derived from context or the user prompted.
    #[property(EVENT)]
    pub fn on_open<on_pre_open, can_open>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        OPEN_CMD
    }

    /// On save command.
    ///
    /// Receives [`SAVE_CMD`] command events scoped on the widget. Usually saves to the last
    /// open or save path, otherwise prompt the user like [`on_save_as`].
    ///
    /// [`on_save_as`]: fn@on_save_as
    #[property(EVENT)]
    pub fn on_save<on_pre_save, can_save>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        SAVE_CMD
    }

    /// On save-as command.
    ///
    /// Receives [`SAVE_AS_CMD`] command events scoped on the widget. Usually prompts the user for
    /// a new save path.
    #[property(EVENT)]
    pub fn on_save_as<on_pre_save_as, can_save_as>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        SAVE_AS_CMD
    }

    /// On cut command.
    ///
    /// Receives [`CUT_CMD`] command events scoped on the widget. You can use the `CLIPBOARD` service
    /// to send data to the clipboard.
    ///
    /// [`CUT_CMD`]: zng_ext_clipboard::CUT_CMD
    #[property(EVENT)]
    pub fn on_cut<on_pre_cut, can_cut>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        CUT_CMD
    }

    /// On copy command.
    ///
    /// Receives [`COPY_CMD`] command events scoped on the widget. You can use the `CLIPBOARD` service
    /// to send data to the clipboard.
    ///
    /// [`COPY_CMD`]: zng_ext_clipboard::COPY_CMD
    #[property(EVENT)]
    pub fn on_copy<on_pre_copy, can_copy>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        COPY_CMD
    }

    /// On paste command.
    ///
    /// Receives [`PASTE_CMD`] command events scoped on the widget. You can use the `CLIPBOARD` service
    /// to receive data from the clipboard.
    ///
    /// [`PASTE_CMD`]: zng_ext_clipboard::PASTE_CMD
    #[property(EVENT)]
    pub fn on_paste<on_pre_paste, can_paste>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        PASTE_CMD
    }

    /// On settings command.
    ///
    /// Receives [`SETTINGS_CMD`] command events scoped on the widget.
    #[property(EVENT)]
    pub fn on_settings<on_pre_settings, can_settings>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        SETTINGS_CMD
    }
}
