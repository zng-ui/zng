//! Common commands.
//!
//! Commands are special events that represent app actions. They do not implement the action invocation or fulfillment, but
//! allows for *binding states* between a widget that fulfills the command and a widget that invokes the command. For
//! example, the [`COPY_CMD`] represents the action of "placing a copy of the selection in the clipboard", a widget can implement
//! this command by handling its event, a different widget can invoke the command using [`Command::notify`] and can bind
//! its enabled property to [`Command::is_enabled`]. The widgets **don't need to known each other** when the first widget can copy
//! it enables the command, this in turn enables the second widget that will invoke the command on an user interaction.
//!
//! Commands can also have any number of metadata associated with then, this metadata is implemented using extension traits,
//! [`CommandNameExt`] adds a `name` text that can be user as a button content for example, [`CommandInfoExt`] adds a longer
//! `info` text that can be used as a *tool-tip*. Some metadata enable new forms of interaction, [`CommandShortcutExt`] adds
//! a `shortcut` value **and** causes the [`GestureManager`](crate::core::gesture::GestureManager) to start invoking enabled
//! commands when the shortcut is pressed.

use crate::core::event::*;
use crate::core::gesture::{shortcut, CommandShortcutExt};

command! {
    /// Represents the clipboard **cut** action.
    pub static CUT_CMD = {
        name: "Cut",
        info: "Remove the selection and place it in the clipboard.",
        shortcut: [shortcut!(CTRL+X), shortcut!(SHIFT+Delete)],
    };

    /// Represents the clipboard **copy** action.
    pub static COPY_CMD = {
        name: "Copy",
        info: "Place a copy of the selection in the clipboard.",
        shortcut: [shortcut!(CTRL+C), shortcut!(CTRL+Insert)],
    };

    /// Represents the clipboard **paste** action.
    pub static PASTE_CMD = {
        name: "Paste",
        info: "Insert content from the clipboard.",
        shortcut: [shortcut!(CTRL+V), shortcut!(SHIFT+Insert)],
    };

    /// Represents the context menu **open** action.
    pub static CONTEXT_MENU_CMD = {
        name: "Context Menu",
        info: "Open the context menu.",
        shortcut: [shortcut!(SHIFT+F10), shortcut!(Apps)],
    };
}
