//! Undo-redo app extension, service and commands.
//!

use std::{any::Any, fmt, time::Duration};

use atomic::{Atomic, Ordering};
use parking_lot::Mutex;

use crate::{
    app::AppExtension, app_local, command, context::WIDGET, context_local, event::CommandNameExt, gesture::CommandShortcutExt, shortcut,
    units::*, var::*, widget_instance::WidgetId,
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
    /// Undo `count` times in the current scope.
    pub fn undo(&self, count: u32) {
        UNDO_SCOPE_CTX.get().undo(count);
    }

    /// Redo `count` times in the current scope.
    pub fn redo(&self, count: u32) {
        UNDO_SCOPE_CTX.get().redo(count);
    }

    /// Gets the parent ID that defines an undo scope, or `None` if undo is registered globally for
    /// the entire app.
    pub fn scope(&self) -> Option<WidgetId> {
        UNDO_SCOPE_CTX.get().id()
    }

    /// Register the action for undo in the current scope.
    pub fn register(&self, action: impl UndoAction) {
        UNDO_SCOPE_CTX.get().register(Box::new(action))
    }

    /// Gets or sets the size limit of each undo stack in all scopes.
    pub fn max_undo(&self) -> ArcVar<u32> {
        UNDO_SV.read().max_undo.clone()
    }

    /// Gets or sets the time interval that groups actions together in all scopes.
    pub fn undo_interval(&self) -> ArcVar<Duration> {
        UNDO_SV.read().undo_interval.clone()
    }
}

/// Represents an undo or redo action.
///
/// If formatted to display it should provide a short description of the action
/// that will be undone or redone.
pub trait UndoRedoItem: fmt::Debug + fmt::Display + Send + Any {}

/// Represents a single undo action.
pub trait UndoAction: UndoRedoItem {
    /// Undo action and returns a [`RedoAction`] that redoes it.
    fn undo(self: Box<Self>) -> Box<dyn RedoAction>;
}

/// Represents a single redo action.
pub trait RedoAction: UndoRedoItem {
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

struct UndoScope {
    id: Atomic<Option<WidgetId>>,
    undo: Mutex<Vec<Box<dyn UndoAction>>>,
    redo: Mutex<Vec<Box<dyn RedoAction>>>,
}
impl Default for UndoScope {
    fn default() -> Self {
        Self {
            id: Atomic::new(WIDGET.try_id()),
            undo: Mutex::new(vec![]),
            redo: Mutex::new(vec![]),
        }
    }
}
impl UndoScope {
    fn register(&self, action: Box<dyn UndoAction>) {
        self.undo.lock().push(action);
        self.redo.lock().clear();
    }

    fn undo(&self, mut count: u32) {
        while count > 0 {
            count -= 1;

            if let Some(undo) = self.undo.lock().pop() {
                let redo = undo.undo();
                self.redo.lock().push(redo);
            }
        }
    }

    fn redo(&self, mut count: u32) {
        while count > 0 {
            count -= 1;

            if let Some(redo) = self.redo.lock().pop() {
                let undo = redo.redo();
                self.undo.lock().push(undo);
            }
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.id.load(Ordering::Relaxed)
    }
}

struct UndoService {
    max_undo: ArcVar<u32>,
    undo_interval: ArcVar<Duration>,
}

impl Default for UndoService {
    fn default() -> Self {
        Self {
            max_undo: var(u32::MAX),
            undo_interval: var(100.ms()),
        }
    }
}

context_local! {
    static UNDO_SCOPE_CTX: UndoScope = UndoScope::default();
}
app_local! {
    static UNDO_SV: UndoService = UndoService::default();
}
