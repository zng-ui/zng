//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`nodes::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{fmt, ops, sync::Arc};

use crate::core::{task::parking_lot::Mutex, undo::*};

use super::{nodes::ResolvedText, *};

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
    op: Arc<Mutex<dyn FnMut(&BoxedVar<Txt>, UndoOp) + Send>>,
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
    /// the [`nodes::resolve_text`] context. You can position the caret using [`ResolvedText::caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    ///
    /// The `op` arguments are the text variable and what [`UndoOp`] operation must be applied to it, all
    /// text edit operations must be undoable, first [`UndoOp::Redo`] is called to "do", then undo and redo again
    /// if the user requests undo & redo. The text variable is always read-write when `op` is called, more than
    /// one op can be called before the text variable updates, and [`ResolvedText::pending_edit`] is always false.
    ///
    /// The `description` must be a short display name for the undo/redo action.
    pub fn new(description: impl Into<Txt>, op: impl FnMut(&BoxedVar<Txt>, UndoOp) + Send + 'static) -> Self {
        Self {
            description: description.into(),
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// Insert operation.
    ///
    /// The `insert` text is inserted at the current caret index or at `0`, or replaces the current selection,
    /// after insert the caret is positioned after the inserted text.
    pub fn insert(description: impl Into<Txt>, insert: impl Into<Txt>) -> Self {
        let insert = insert.into();
        let mut insert_idx = usize::MAX;
        Self::new(description, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                if insert_idx == usize::MAX {
                    insert_idx = caret.index.unwrap_or(0);
                }

                txt.modify(clmv!(insert, |args| {
                    args.to_mut().to_mut().insert_str(insert_idx, insert.as_str());
                }))
                .unwrap();

                caret.set_index(insert_idx + insert.len());
            }
            UndoOp::Undo => {
                let len = insert.len();
                txt.modify(move |args| {
                    args.to_mut().to_mut().replace_range(insert_idx..insert_idx + len, "");
                })
                .unwrap();

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                caret.set_index(insert_idx);
            }
        })
    }

    /// Remove one *backspace range* ending at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::backspace_range`] for more details about what is removed.
    pub fn backspace(description: impl Into<Txt>) -> Self {
        let mut removed = Txt::from_static("");
        let mut undo_idx = usize::MAX;

        Self::new(description, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = caret.index.unwrap_or(0);
                let rmv = ctx.text.backspace_range(caret_idx);
                if rmv.is_empty() {
                    removed = Txt::from_static("");
                    return;
                }

                txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != removed {
                        removed = Txt::from_str(r);
                        undo_idx = caret_idx - removed.len();
                    }
                });

                txt.modify(move |args| {
                    args.to_mut().to_mut().replace_range(rmv, "");
                })
                .unwrap();

                caret.set_index(undo_idx);
            }
            UndoOp::Undo => {
                if removed.is_empty() {
                    return;
                }

                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().insert_str(undo_idx, removed.as_str());
                }))
                .unwrap();

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                caret.set_index(undo_idx + removed.len());
            }
        })
    }

    /// Remove one *delete range* starting at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::delete_range`] for more details about what is removed.
    pub fn delete(description: impl Into<Txt>) -> Self {
        let mut removed = Txt::from_static("");

        Self::new(description, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = caret.index.unwrap_or(0);

                let rmv = ctx.text.delete_range(caret_idx);

                if rmv.is_empty() {
                    removed = Txt::from_static("");
                    return;
                }

                txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != removed {
                        removed = Txt::from_str(r);
                    }
                });
                txt.modify(move |args| {
                    args.to_mut().to_mut().replace_range(rmv, "");
                })
                .unwrap();

                caret.set_index(caret_idx); // (re)start caret animation
            }
            UndoOp::Undo => {
                if removed.is_empty() {
                    return;
                }

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = caret.index.unwrap_or(0);

                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().insert_str(caret_idx, removed.as_str());
                }))
                .unwrap();

                caret.set_index(caret_idx + removed.len());
            }
        })
    }

    /// Replace operation.
    ///
    /// The `select_before` is removed, and `insert` inserted at the `select_before.start`, after insertion
    /// the `select_after` is applied, you can use an empty insert to just remove.
    ///
    /// All indexes are snapped to the nearest grapheme, you can use empty ranges to just position the caret.
    pub fn replace(
        description: impl Into<Txt>,
        mut select_before: ops::Range<usize>,
        insert: impl Into<Txt>,
        mut select_after: ops::Range<usize>,
    ) -> Self {
        let insert = insert.into();
        let mut removed = Txt::from_static("");

        Self::new(description, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();

                select_before.start = ctx.text.snap_grapheme_boundary(select_before.start);
                select_before.end = ctx.text.snap_grapheme_boundary(select_before.end);

                txt.with(|t| {
                    let r = &t[select_before.clone()];
                    if r != removed {
                        removed = Txt::from_str(r);
                    }
                });

                txt.modify(clmv!(select_before, insert, |args| {
                    args.to_mut().to_mut().replace_range(select_before, insert.as_str());
                }))
                .unwrap();

                ctx.caret.lock().set_index(select_after.start); // TODO, selection
            }
            UndoOp::Undo => {
                let ctx = ResolvedText::get();

                select_after.start = ctx.text.snap_grapheme_boundary(select_after.start);
                select_after.end = ctx.text.snap_grapheme_boundary(select_after.end);

                txt.modify(clmv!(select_after, removed, |args| {
                    args.to_mut().to_mut().replace_range(select_after, removed.as_str());
                }))
                .unwrap();

                ctx.caret.lock().set_index(select_before.start); // TODO, selection
            }
        })
    }

    pub(super) fn call(self, text: &BoxedVar<Txt>) {
        (self.op.lock())(text, UndoOp::Redo);
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
        (self.edit_op.op.lock())(text, self.exec_op)
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
