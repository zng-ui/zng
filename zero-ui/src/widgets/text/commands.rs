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
    info: Arc<dyn UndoInfo>,
    op: Arc<Mutex<dyn FnMut(&BoxedVar<Txt>, UndoOp) + Send>>,
}
impl fmt::Debug for TextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextEditOp")
            .field("info", &self.info.description())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for TextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info.description())
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
    pub fn new(undo_info: impl UndoInfo, op: impl FnMut(&BoxedVar<Txt>, UndoOp) + Send + 'static) -> Self {
        Self {
            info: undo_info.into_dyn(),
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// Insert operation.
    ///
    /// The `insert` text is inserted at the current caret index or at `0`, or replaces the current selection,
    /// after insert the caret is positioned after the inserted text.
    pub fn insert(undo_info: impl UndoInfo, insert: impl Into<Txt>) -> Self {
        let insert = insert.into();
        let mut insert_idx = CaretIndex {
            index: usize::MAX,
            line: 0,
        };
        Self::new(undo_info, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                if insert_idx.index == usize::MAX {
                    insert_idx = caret.index.unwrap_or(CaretIndex::ZERO);
                }

                let i = insert_idx.index;
                txt.modify(clmv!(insert, |args| {
                    args.to_mut().to_mut().insert_str(i, insert.as_str());
                }))
                .unwrap();

                let mut i = insert_idx;
                i.index += insert.len();
                caret.set_index(i);
            }
            UndoOp::Undo => {
                let len = insert.len();
                let i = insert_idx.index;
                txt.modify(move |args| {
                    args.to_mut().to_mut().replace_range(i..i + len, "");
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
    pub fn backspace(undo_info: impl UndoInfo) -> Self {
        let mut removed = Txt::from_static("");
        let mut undo_idx = CaretIndex {
            index: usize::MAX,
            line: 0,
        };

        Self::new(undo_info, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = caret.index.unwrap_or(CaretIndex::ZERO);
                let rmv = ctx.text.backspace_range(caret_idx.index);
                if rmv.is_empty() {
                    removed = Txt::from_static("");
                    return;
                }

                txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != removed {
                        removed = Txt::from_str(r);
                        undo_idx.index = caret_idx.index - removed.len();
                        undo_idx.line = caret_idx.line;
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

                let i = undo_idx.index;
                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().insert_str(i, removed.as_str());
                }))
                .unwrap();

                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();
                let mut i = undo_idx;
                i.index += removed.len();
                caret.set_index(i);
            }
        })
    }

    /// Remove one *delete range* starting at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::delete_range`] for more details about what is removed.
    pub fn delete(undo_info: impl UndoInfo) -> Self {
        let mut removed = Txt::from_static("");

        Self::new(undo_info, move |txt, op| match op {
            UndoOp::Redo => {
                let ctx = ResolvedText::get();
                let mut caret = ctx.caret.lock();

                let caret_idx = caret.index.unwrap_or(CaretIndex::ZERO);

                let rmv = ctx.text.delete_range(caret_idx.index);

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

                let caret_idx = caret.index.unwrap_or(CaretIndex::ZERO);

                let i = caret_idx.index;
                txt.modify(clmv!(removed, |args| {
                    args.to_mut().to_mut().insert_str(i, removed.as_str());
                }))
                .unwrap();

                let mut i = caret_idx;
                i.index += removed.len();
                caret.set_index(i);
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
        undo_info: impl UndoInfo,
        mut select_before: ops::Range<usize>,
        insert: impl Into<Txt>,
        mut select_after: ops::Range<usize>,
    ) -> Self {
        let insert = insert.into();
        let mut removed = Txt::from_static("");

        Self::new(undo_info, move |txt, op| match op {
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

                ctx.caret.lock().set_char_index(select_after.start); // TODO, selection
            }
            UndoOp::Undo => {
                let ctx = ResolvedText::get();

                select_after.start = ctx.text.snap_grapheme_boundary(select_after.start);
                select_after.end = ctx.text.snap_grapheme_boundary(select_after.end);

                txt.modify(clmv!(select_after, removed, |args| {
                    args.to_mut().to_mut().replace_range(select_after, removed.as_str());
                }))
                .unwrap();

                ctx.caret.lock().set_char_index(select_before.start); // TODO, selection
            }
        })
    }

    pub(super) fn call(self, text: &BoxedVar<Txt>) {
        (self.op.lock())(text, UndoOp::Redo);
        UNDO.register(self.info.clone(), UndoTextEditOp::new(self))
    }
}

/// Parameter for [`EDIT_CMD`], apply the request and don't register undo.
#[derive(Debug, Clone)]
pub(super) struct UndoTextEditOp {
    pub target: WidgetId,
    edit_op: TextEditOp,
    exec_op: UndoOp,
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
