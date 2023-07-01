//! Undo-redo app extension, service and commands.
//!

use std::{any::Any, time::Duration};

use crate::{
    app::AppExtension, command, context::StateMapRef, event::CommandNameExt, gesture::CommandShortcutExt, shortcut, var::*,
    widget_instance::WidgetId,
};

/// Undo-redo app extension.
///
/// # Services
///
/// Services provided by this extension.
///
/// * [`UNDO`]
///
/// # Default
///
/// This extension is included in the [default app].
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct UndoManager {}

impl AppExtension for UndoManager {}

/// Undo-redo service.
pub struct UNDO;
impl UNDO {
    /// Undo once in the current scope.
    pub fn undo(&self) {
        todo!()
    }

    /// Redo once in the current scope.
    pub fn redo(&self) {
        todo!()
    }

    /// Gets the parent ID that defines an undo scope, or `None` if undo is registered globally for
    /// the entire app.
    pub fn scope(&self) -> Option<WidgetId> {
        todo!()
    }

    /// Register the action for undo in the current scope.
    pub fn register(&self, action: impl UndoAction) {
        let _ = action;
        todo!()
    }

    /// Gets or sets the size limit of the undo stack.
    pub fn max_undo(&self) -> ArcVar<u32> {
        todo!()
    }

    /// Gets or sets the time interval that groups actions together.
    pub fn undo_group_interval(&self) -> ArcVar<Duration> {
        todo!()
    }
}

/// Represents associated metadata with an undo or redo action.
pub trait UndoRedoMeta: Send + Any {
    /// Any metadata associated with the action.
    fn meta(&self) -> StateMapRef<UNDO> {
        StateMapRef::empty()
    }
}

/// Represents a single undo action.
pub trait UndoAction: UndoRedoMeta {
    /// Undo action and returns a [`RedoAction`] that redoes it.
    fn undo(self: Box<Self>) -> Box<dyn RedoAction>;
}

/// Represents a single redo action.
pub trait RedoAction: UndoRedoMeta {
    /// Redo action and returns a [`UndoAction`] that undoes it.
    fn redo(self: Box<Self>) -> Box<dyn UndoAction>;
}

command! {
    /// Represents the **undo** action.
    pub static UNDO_CMD = {
        name: "Undo",
        shortcut: [shortcut!(CTRL+Z)],
    };

    /// Represents the clipboard **redo** action.
    pub static REDO_CMD = {
        name: "Redo",
        shortcut: [shortcut!(CTRL+Y)],
    };
}
