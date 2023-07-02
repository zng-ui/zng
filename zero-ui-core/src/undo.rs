//! Undo-redo app extension, service and commands.
//!

use std::{any::Any, fmt, sync::atomic::AtomicBool, time::Duration};

use atomic::{Atomic, Ordering};
use parking_lot::Mutex;

use crate::{
    app::AppExtension,
    app_local, command,
    context::WIDGET,
    context_local,
    event::{AnyEventArgs, CommandNameExt},
    gesture::CommandShortcutExt,
    shortcut,
    units::*,
    var::*,
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

impl AppExtension for UndoManager {
    fn event(&mut self, update: &mut crate::event::EventUpdate) {
        // app scope handler
        if let Some(args) = UNDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            UNDO.undo(args.param::<u32>().copied().unwrap_or(1));
        } else if let Some(args) = UNDO_CMD.on_unhandled(update) {
            args.propagation().stop();
            UNDO.redo(args.param::<u32>().copied().unwrap_or(1));
        }
    }
}

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
    ///
    /// # Param
    ///
    /// If the command parameter is a `u32` it is the count of undo actions to run, otherwise runs `1` action.
    pub static UNDO_CMD = {
        name: "Undo",
        shortcut: [shortcut!(CTRL+Z)],
    };

    /// Represents the clipboard **redo** action.
    ///
    /// # Param
    ///
    /// If the command parameter is a `u32` it is the count of redo actions to run, otherwise runs `1` action.
    pub static REDO_CMD = {
        name: "Redo",
        shortcut: [shortcut!(CTRL+Y)],
    };
}
#[derive(Default)]
struct UndoScope {
    id: Atomic<Option<WidgetId>>,
    undo: Mutex<Vec<Box<dyn UndoAction>>>,
    redo: Mutex<Vec<Box<dyn RedoAction>>>,
    enabled: AtomicBool,
}
impl UndoScope {
    fn register(&self, action: Box<dyn UndoAction>) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        self.undo.lock().push(action);
        self.redo.lock().clear();
    }

    fn undo(&self, mut count: u32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        while count > 0 {
            count -= 1;

            if let Some(undo) = self.undo.lock().pop() {
                let redo = undo.undo();
                self.redo.lock().push(redo);
            }
        }
    }

    fn redo(&self, mut count: u32) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        while count > 0 {
            count -= 1;

            if let Some(redo) = self.redo.lock().pop() {
                let undo = redo.redo();
                self.undo.lock().push(undo);
            }
        }
    }

    fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.undo.lock().clear();
            self.redo.lock().clear();
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

mod properties {
    use std::sync::Arc;

    use super::*;
    use crate::{event::CommandHandle, widget_instance::*, *};

    /// Defines an undo/redo scope in the widget.
    ///
    /// If `enabled` is `false` undo/redo is disabled for the widget and descendants, if it is
    /// `true` all undo/redo actions
    #[property(WIDGET)]
    pub fn undo_scope(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        let mut scope = None;
        let mut undo_cmd = CommandHandle::dummy();
        let mut redo_cmd = CommandHandle::dummy();
        let enabled = enabled.into_var();
        match_node(child, move |c, op| {
            match op {
                UiNodeOp::Init => {
                    let id = WIDGET.id();
                    let s = UndoScope {
                        id: Atomic::new(Some(id)),
                        enabled: AtomicBool::new(enabled.get()),
                        ..Default::default()
                    };
                    scope = Some(Arc::new(s));

                    undo_cmd = UNDO_CMD.scoped(id).subscribe(false);
                    redo_cmd = REDO_CMD.scoped(id).subscribe(false);

                    WIDGET.sub_var(&enabled);
                }
                UiNodeOp::Deinit => {
                    UNDO_SCOPE_CTX.with_context(&mut scope, || c.deinit());
                    scope = None;
                    undo_cmd = CommandHandle::dummy();
                    redo_cmd = CommandHandle::dummy();
                    return;
                }
                UiNodeOp::Event { update } => {
                    let id = WIDGET.id();
                    if let Some(args) = UNDO_CMD.scoped(id).on_unhandled(update) {
                        args.propagation().stop();
                        let scope = scope.as_ref().unwrap();
                        scope.undo(args.param::<u32>().copied().unwrap_or(1));
                    } else if let Some(args) = REDO_CMD.scoped(id).on_unhandled(update) {
                        args.propagation().stop();
                        let scope = scope.as_ref().unwrap();
                        scope.redo(args.param::<u32>().copied().unwrap_or(1));
                    }
                }
                UiNodeOp::Update { .. } => {
                    if let Some(enabled) = enabled.get_new() {
                        scope.as_ref().unwrap().set_enabled(enabled);
                    }
                }
                _ => {}
            }

            UNDO_SCOPE_CTX.with_context(&mut scope, || c.op(op));

            let scope = scope.as_ref().unwrap();
            undo_cmd.set_enabled(!scope.undo.lock().is_empty());
            redo_cmd.set_enabled(!scope.redo.lock().is_empty());
        })
    }
}
pub use properties::undo_scope;