//! Common commands.
//!

use crate::core::clipboard::*;
use crate::core::event::*;
use crate::core::gesture::{shortcut, CommandShortcutExt};
use crate::prelude::new_property::*;

command! {

    /// Represents the context menu **open** action.
    pub static CONTEXT_MENU_CMD = {
        name: "Context Menu",
        info: "Open the context menu.",
        shortcut: [shortcut!(SHIFT+F10), shortcut!(Apps)],
    };
}

command_property! {
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
