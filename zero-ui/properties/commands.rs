//! Common commands.
//!
//! Commands are special events that represent app actions. They do not implement the action invocation or fulfillment, but
//! allows for *binding states* between a widget that fulfills the command and a widget that invokes the command. For
//! example, the [`CopyCommand`] represents the action of "placing a copy of the selection in the clipboard", a widget can implement
//! this command by handling its event using TODO, a different widget can invoke the command using [`CopyCommand::notify`] and can bind 
//! its enabled property to [`CopyCommand::enabled`]. The widgets **don't need to known each other** when the first widget can copy 
//! it enables the command, this in turn enables the second widget that will invoke the command on an user interaction.
//!
//! Commands can also have any number of metadata associated with then, this metadata is implemented using extension traits,
//! [`CommandNameExt`] adds a `name` text that can be user as a button content for example, [`CommandInfoExt`] adds a longer
//! `info` text that can be used as a *tool-tip*. Some metadata enable new forms of interaction, [`CommandShortcutExt`] adds
//! a `shortcut` value **and** causes the [`GestureManager`](crate::core::gesture::GestureManager) to start invoking enabled 
//! commands when the shortcut is pressed.

use crate::core::command::*;

command! {
    /// Represents the clipboard **cut** action.
    pub CutCommand
        .init_name("Cut")
        .init_info("Remove the selection and place it in the clipboard.");

    /// Represents the clipboard **copy** action.
    pub CopyCommand
        .init_name("Copy")
        .init_info("Place a copy of the selection in the clipboard.");

    /// Represents the clipboard **paste** action.
    pub PasteCommand
        .init_name("Paste")
        .init_info("Insert content from the clipboard.");
}
