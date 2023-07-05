//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`nodes::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{fmt, sync::Arc};

use crate::core::undo::*;

use super::*;

command! {
    /// Applies the [`TextEditOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static EDIT_CMD;
}

/// Represents a text edit operation that can be send to an editable text using [`EDIT_CMD`].
#[derive(Clone)]
pub struct TextEditOp {
    description: Txt,
    op: Arc<dyn Fn(&BoxedVar<Txt>, UndoOp) + Send + Sync>,
}
impl fmt::Debug for TextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextEditOp")
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for TextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}
impl TextEditOp {
    /// New text edit op.
    ///
    /// The editable text widget that handles [`EDIT_CMD`] will call `op` during event handling in
    /// the [`nodes::resolve_text`] context.
    ///
    /// The `op` arguments are the text variable and what [`UndoOp`] operation must be applied to it, all
    /// text edit operations must be undoable, first [`UndoOp::Redo`] is called to "do", then undo and redo again
    /// if the user requests undo & redo.
    ///
    /// The `description` must be a short display name for the undo/redo action.
    pub fn new(description: impl Into<Txt>, op: impl Fn(&BoxedVar<Txt>, UndoOp) + Send + Sync + 'static) -> Self {
        Self {
            description: description.into(),
            op: Arc::new(op),
        }
    }

    pub(super) fn call(self, text: &BoxedVar<Txt>) {
        (self.op)(text, UndoOp::Redo);
        UNDO.register(UndoTextEditOp::new(self))
    }
}

/// Parameter for [`EDIT_CMD`], apply the request and don't register undo.
#[derive(Debug, Clone)]
pub(super) struct UndoTextEditOp {
    pub target: WidgetId,
    edit_op: TextEditOp,
    exec_op: UndoOp,
}
impl fmt::Display for UndoTextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.edit_op)
    }
}
impl UndoTextEditOp {
    fn new(edit_op: TextEditOp) -> Self {
        Self {
            target: WIDGET.id(),
            edit_op,
            exec_op: UndoOp::Undo,
        }
    }

    pub(super) fn call(&self, text: &BoxedVar<Txt>) {
        (self.edit_op.op)(text, self.exec_op)
    }
}
impl UndoRedoItem for UndoTextEditOp {}
impl UndoAction for UndoTextEditOp {
    fn undo(self: Box<Self>) -> Box<dyn RedoAction> {
        EDIT_CMD.scoped(self.target).notify_param(Self {
            target: self.target,
            edit_op: self.edit_op.clone(),
            exec_op: UndoOp::Undo,
        });
        self
    }
}

impl RedoAction for UndoTextEditOp {
    fn redo(self: Box<Self>) -> Box<dyn UndoAction> {
        EDIT_CMD.scoped(self.target).notify_param(Self {
            target: self.target,
            edit_op: self.edit_op.clone(),
            exec_op: UndoOp::Redo,
        });
        self
    }
}
