//! Common commands
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
