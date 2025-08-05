//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`node::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{any::Any, borrow::Cow, cmp, fmt, ops, sync::Arc};

use parking_lot::Mutex;
use zng_ext_font::*;
use zng_ext_l10n::l10n;
use zng_ext_undo::*;
use zng_layout::unit::DistanceKey;
use zng_wgt::prelude::*;

use crate::node::{RichText, RichTextWidgetInfoExt, notify_leaf_select_op};

use super::{node::TEXT, *};

command! {
    /// Applies the [`TextEditOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static EDIT_CMD;

    /// Applies the [`TextSelectOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static SELECT_CMD;

    /// Select all text.
    ///
    /// The request is the same as [`SELECT_CMD`] with [`TextSelectOp::select_all`].
    pub static SELECT_ALL_CMD = {
        l10n!: true,
        name: "Select All",
        shortcut: shortcut!(CTRL+'A'),
        shortcut_filter: ShortcutFilter::FOCUSED | ShortcutFilter::CMD_ENABLED,
    };

    /// Parse text and update value if [`txt_parse`] is pending.
    ///
    /// [`txt_parse`]: fn@super::txt_parse
    pub static PARSE_CMD;
}

struct SharedTextEditOp {
    data: Box<dyn Any + Send>,
    op: Box<dyn FnMut(&mut dyn Any, UndoFullOp) + Send>,
}

/// Represents a text edit operation that can be send to an editable text using [`EDIT_CMD`].
#[derive(Clone)]
pub struct TextEditOp(Arc<Mutex<SharedTextEditOp>>);
impl fmt::Debug for TextEditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextEditOp").finish_non_exhaustive()
    }
}
impl TextEditOp {
    /// New text edit operation.
    ///
    /// The editable text widget that handles [`EDIT_CMD`] will call `op` during event handling in
    /// the [`node::resolve_text`] context meaning the [`TEXT.resolved`] and [`TEXT.resolve_caret`] service is available in `op`.
    /// The text is edited by modifying [`ResolvedText::txt`]. The text widget will detect changes to the caret and react s
    /// accordingly (updating caret position and animation), the caret index is also snapped to the nearest grapheme start.
    ///
    /// The `op` arguments are a custom data `D` and what [`UndoFullOp`] to run, all
    /// text edit operations must be undoable, first [`UndoOp::Redo`] is called to "do", then undo and redo again
    /// if the user requests undo & redo. The text variable is always read-write when `op` is called, more than
    /// one op can be called before the text variable updates, and [`ResolvedText::pending_edit`] is always false.
    ///
    /// [`ResolvedText::txt`]: crate::node::ResolvedText::txt
    /// [`ResolvedText::caret`]: crate::node::ResolvedText::caret
    /// [`ResolvedText::pending_edit`]: crate::node::ResolvedText::pending_edit
    /// [`TEXT.resolved`]: crate::node::TEXT::resolved
    /// [`TEXT.resolve_caret`]: crate::node::TEXT::resolve_caret
    /// [`UndoFullOp`]: zng_ext_undo::UndoFullOp
    /// [`UndoOp::Redo`]: zng_ext_undo::UndoOp::Redo
    pub fn new<D>(data: D, mut op: impl FnMut(&mut D, UndoFullOp) + Send + 'static) -> Self
    where
        D: Send + Any + 'static,
    {
        Self(Arc::new(Mutex::new(SharedTextEditOp {
            data: Box::new(data),
            op: Box::new(move |data, o| op(data.downcast_mut().unwrap(), o)),
        })))
    }

    /// Insert operation.
    ///
    /// The `insert` text is inserted at the current caret index or at `0`, or replaces the current selection,
    /// after insert the caret is positioned after the inserted text.
    pub fn insert(insert: impl Into<Txt>) -> Self {
        struct InsertData {
            insert: Txt,
            selection_state: SelectionState,
            removed: Txt,
        }
        let data = InsertData {
            insert: insert.into(),
            selection_state: SelectionState::PreInit,
            removed: Txt::from_static(""),
        };

        Self::new(data, move |data, op| match op {
            UndoFullOp::Init { redo } => {
                let ctx = TEXT.resolved();
                let caret = &ctx.caret;

                let mut rmv_range = 0..0;

                if let Some(range) = caret.selection_range() {
                    rmv_range = range.start.index..range.end.index;

                    ctx.txt.with(|t| {
                        let r = &t[rmv_range.clone()];
                        if r != data.removed {
                            data.removed = Txt::from_str(r);
                        }
                    });

                    if range.start.index == caret.index.unwrap_or(CaretIndex::ZERO).index {
                        data.selection_state = SelectionState::CaretSelection(range.start, range.end);
                    } else {
                        data.selection_state = SelectionState::SelectionCaret(range.start, range.end);
                    }
                } else {
                    data.selection_state = SelectionState::Caret(caret.index.unwrap_or(CaretIndex::ZERO));
                }

                Self::apply_max_count(redo, &ctx.txt, rmv_range, &mut data.insert)
            }
            UndoFullOp::Op(UndoOp::Redo) => {
                let insert = &data.insert;

                match data.selection_state {
                    SelectionState::PreInit => unreachable!(),
                    SelectionState::Caret(insert_idx) => {
                        let i = insert_idx.index;
                        TEXT.resolved().txt.modify(clmv!(insert, |args| {
                            args.to_mut().insert_str(i, insert.as_str());
                        }));

                        let mut i = insert_idx;
                        i.index += insert.len();

                        let mut caret = TEXT.resolve_caret();
                        caret.set_index(i);
                        caret.clear_selection();
                    }
                    SelectionState::CaretSelection(start, end) | SelectionState::SelectionCaret(start, end) => {
                        let char_range = start.index..end.index;
                        TEXT.resolved().txt.modify(clmv!(insert, |args| {
                            args.to_mut().replace_range(char_range, insert.as_str());
                        }));

                        let mut caret = TEXT.resolve_caret();
                        caret.set_char_index(start.index + insert.len());
                        caret.clear_selection();
                    }
                }
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let len = data.insert.len();
                let (insert_idx, selection_idx, caret_idx) = match data.selection_state {
                    SelectionState::Caret(c) => (c, None, c),
                    SelectionState::CaretSelection(start, end) => (start, Some(end), start),
                    SelectionState::SelectionCaret(start, end) => (start, Some(start), end),
                    SelectionState::PreInit => unreachable!(),
                };
                let i = insert_idx.index;
                let removed = &data.removed;

                TEXT.resolved().txt.modify(clmv!(removed, |args| {
                    args.to_mut().replace_range(i..i + len, removed.as_str());
                }));

                let mut caret = TEXT.resolve_caret();
                caret.set_index(caret_idx);
                caret.selection_index = selection_idx;
            }
            UndoFullOp::Info { info } => {
                let mut label = Txt::from_static("\"");
                for (i, mut c) in data.insert.chars().take(21).enumerate() {
                    if i == 20 {
                        c = '…';
                    } else if c == '\n' {
                        c = '↵';
                    } else if c == '\t' {
                        c = '→';
                    } else if c == '\r' {
                        continue;
                    }
                    label.push(c);
                }
                label.push('"');
                *info = Some(Arc::new(label));
            }
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => {
                if within_undo_interval
                    && let Some(next_data) = next_data.downcast_mut::<InsertData>()
                    && let SelectionState::Caret(mut after_idx) = data.selection_state
                    && let SelectionState::Caret(caret) = next_data.selection_state
                {
                    after_idx.index += data.insert.len();

                    if after_idx.index == caret.index {
                        data.insert.push_str(&next_data.insert);
                        *merged = true;
                    }
                }
            }
        })
    }

    /// Remove one *backspace range* ending at the caret index, or removes the selection.
    ///
    /// See [`SegmentedText::backspace_range`] for more details about what is removed.
    ///
    /// [`SegmentedText::backspace_range`]: zng_ext_font::SegmentedText::backspace_range
    pub fn backspace() -> Self {
        Self::backspace_impl(SegmentedText::backspace_range)
    }
    /// Remove one *backspace word range* ending at the caret index, or removes the selection.
    ///
    /// See [`SegmentedText::backspace_word_range`] for more details about what is removed.
    ///
    /// [`SegmentedText::backspace_word_range`]: zng_ext_font::SegmentedText::backspace_word_range
    pub fn backspace_word() -> Self {
        Self::backspace_impl(SegmentedText::backspace_word_range)
    }
    fn backspace_impl(backspace_range: fn(&SegmentedText, usize, u32) -> std::ops::Range<usize>) -> Self {
        struct BackspaceData {
            selection_state: SelectionState,
            count: u32,
            removed: Txt,
        }
        let data = BackspaceData {
            selection_state: SelectionState::PreInit,
            count: 1,
            removed: Txt::from_static(""),
        };

        Self::new(data, move |data, op| match op {
            UndoFullOp::Init { .. } => {
                let ctx = TEXT.resolved();
                let caret = &ctx.caret;

                if let Some(range) = caret.selection_range() {
                    if range.start.index == caret.index.unwrap_or(CaretIndex::ZERO).index {
                        data.selection_state = SelectionState::CaretSelection(range.start, range.end);
                    } else {
                        data.selection_state = SelectionState::SelectionCaret(range.start, range.end);
                    }
                } else {
                    data.selection_state = SelectionState::Caret(caret.index.unwrap_or(CaretIndex::ZERO));
                }
            }
            UndoFullOp::Op(UndoOp::Redo) => {
                let rmv = match data.selection_state {
                    SelectionState::Caret(c) => backspace_range(&TEXT.resolved().segmented_text, c.index, data.count),
                    SelectionState::CaretSelection(s, e) | SelectionState::SelectionCaret(s, e) => s.index..e.index,
                    SelectionState::PreInit => unreachable!(),
                };
                if rmv.is_empty() {
                    data.removed = Txt::from_static("");
                    return;
                }

                {
                    let mut caret = TEXT.resolve_caret();
                    caret.set_char_index(rmv.start);
                    caret.clear_selection();
                }

                let ctx = TEXT.resolved();
                ctx.txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != data.removed {
                        data.removed = Txt::from_str(r);
                    }
                });

                ctx.txt.modify(move |args| {
                    args.to_mut().replace_range(rmv, "");
                });
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                if data.removed.is_empty() {
                    return;
                }

                let (insert_idx, selection_idx, caret_idx) = match data.selection_state {
                    SelectionState::Caret(c) => (c.index - data.removed.len(), None, c),
                    SelectionState::CaretSelection(s, e) => (s.index, Some(e), s),
                    SelectionState::SelectionCaret(s, e) => (s.index, Some(s), e),
                    SelectionState::PreInit => unreachable!(),
                };
                let removed = &data.removed;

                TEXT.resolved().txt.modify(clmv!(removed, |args| {
                    args.to_mut().insert_str(insert_idx, removed.as_str());
                }));

                let mut caret = TEXT.resolve_caret();
                caret.set_index(caret_idx);
                caret.selection_index = selection_idx;
            }
            UndoFullOp::Info { info } => {
                *info = Some(if data.count == 1 {
                    Arc::new("⌫")
                } else {
                    Arc::new(formatx!("⌫ (x{})", data.count))
                })
            }
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => {
                if within_undo_interval
                    && let Some(next_data) = next_data.downcast_mut::<BackspaceData>()
                    && let SelectionState::Caret(mut after_idx) = data.selection_state
                    && let SelectionState::Caret(caret) = next_data.selection_state
                {
                    after_idx.index -= data.removed.len();

                    if after_idx.index == caret.index {
                        data.count += next_data.count;

                        next_data.removed.push_str(&data.removed);
                        data.removed = std::mem::take(&mut next_data.removed);
                        *merged = true;
                    }
                }
            }
        })
    }

    /// Remove one *delete range* starting at the caret index, or removes the selection.
    ///
    /// See [`SegmentedText::delete_range`] for more details about what is removed.
    ///
    /// [`SegmentedText::delete_range`]: zng_ext_font::SegmentedText::delete_range
    pub fn delete() -> Self {
        Self::delete_impl(SegmentedText::delete_range)
    }
    /// Remove one *delete word range* starting at the caret index, or removes the selection.
    ///
    /// See [`SegmentedText::delete_word_range`] for more details about what is removed.
    ///
    /// [`SegmentedText::delete_word_range`]: zng_ext_font::SegmentedText::delete_word_range
    pub fn delete_word() -> Self {
        Self::delete_impl(SegmentedText::delete_word_range)
    }
    fn delete_impl(delete_range: fn(&SegmentedText, usize, u32) -> std::ops::Range<usize>) -> Self {
        struct DeleteData {
            selection_state: SelectionState,
            count: u32,
            removed: Txt,
        }
        let data = DeleteData {
            selection_state: SelectionState::PreInit,
            count: 1,
            removed: Txt::from_static(""),
        };

        Self::new(data, move |data, op| match op {
            UndoFullOp::Init { .. } => {
                let ctx = TEXT.resolved();
                let caret = &ctx.caret;

                if let Some(range) = caret.selection_range() {
                    if range.start.index == caret.index.unwrap_or(CaretIndex::ZERO).index {
                        data.selection_state = SelectionState::CaretSelection(range.start, range.end);
                    } else {
                        data.selection_state = SelectionState::SelectionCaret(range.start, range.end);
                    }
                } else {
                    data.selection_state = SelectionState::Caret(caret.index.unwrap_or(CaretIndex::ZERO));
                }
            }
            UndoFullOp::Op(UndoOp::Redo) => {
                let rmv = match data.selection_state {
                    SelectionState::CaretSelection(s, e) | SelectionState::SelectionCaret(s, e) => s.index..e.index,
                    SelectionState::Caret(c) => delete_range(&TEXT.resolved().segmented_text, c.index, data.count),
                    SelectionState::PreInit => unreachable!(),
                };

                if rmv.is_empty() {
                    data.removed = Txt::from_static("");
                    return;
                }

                {
                    let mut caret = TEXT.resolve_caret();
                    caret.set_char_index(rmv.start); // (re)start caret animation
                    caret.clear_selection();
                }

                let ctx = TEXT.resolved();
                ctx.txt.with(|t| {
                    let r = &t[rmv.clone()];
                    if r != data.removed {
                        data.removed = Txt::from_str(r);
                    }
                });
                ctx.txt.modify(move |args| {
                    args.to_mut().replace_range(rmv, "");
                });
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let removed = &data.removed;

                if data.removed.is_empty() {
                    return;
                }

                let (insert_idx, selection_idx, caret_idx) = match data.selection_state {
                    SelectionState::Caret(c) => (c.index, None, c),
                    SelectionState::CaretSelection(s, e) => (s.index, Some(e), s),
                    SelectionState::SelectionCaret(s, e) => (s.index, Some(s), e),
                    SelectionState::PreInit => unreachable!(),
                };

                TEXT.resolved().txt.modify(clmv!(removed, |args| {
                    args.to_mut().insert_str(insert_idx, removed.as_str());
                }));

                let mut caret = TEXT.resolve_caret();
                caret.set_index(caret_idx); // (re)start caret animation
                caret.selection_index = selection_idx;
            }
            UndoFullOp::Info { info } => {
                *info = Some(if data.count == 1 {
                    Arc::new("⌦")
                } else {
                    Arc::new(formatx!("⌦ (x{})", data.count))
                })
            }
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => {
                if within_undo_interval
                    && let Some(next_data) = next_data.downcast_ref::<DeleteData>()
                    && let SelectionState::Caret(after_idx) = data.selection_state
                    && let SelectionState::Caret(caret) = next_data.selection_state
                    && after_idx.index == caret.index
                {
                    data.count += next_data.count;
                    data.removed.push_str(&next_data.removed);
                    *merged = true;
                }
            }
        })
    }

    fn apply_max_count(redo: &mut bool, txt: &Var<Txt>, rmv_range: ops::Range<usize>, insert: &mut Txt) {
        let max_count = MAX_CHARS_COUNT_VAR.get();
        if max_count > 0 {
            // max count enabled
            let (txt_count, rmv_count) = txt.with(|t| (t.chars().count(), t[rmv_range].chars().count()));
            let ins_count = insert.chars().count();

            let final_count = txt_count - rmv_count + ins_count;
            if final_count > max_count {
                // need to truncate insert
                let ins_rmv = final_count - max_count;
                if ins_rmv < ins_count {
                    // can truncate insert
                    let i = insert.char_indices().nth(ins_count - ins_rmv).unwrap().0;
                    insert.truncate(i);
                } else {
                    // cannot insert
                    debug_assert!(txt_count >= max_count);
                    *redo = false;
                }
            }
        }
    }

    /// Remove all the text.
    pub fn clear() -> Self {
        #[derive(Default, Clone)]
        struct Cleared {
            txt: Txt,
            selection: SelectionState,
        }
        Self::new(Cleared::default(), |data, op| match op {
            UndoFullOp::Init { .. } => {
                let ctx = TEXT.resolved();
                data.txt = ctx.txt.get();
                if let Some(range) = ctx.caret.selection_range() {
                    if range.start.index == ctx.caret.index.unwrap_or(CaretIndex::ZERO).index {
                        data.selection = SelectionState::CaretSelection(range.start, range.end);
                    } else {
                        data.selection = SelectionState::SelectionCaret(range.start, range.end);
                    }
                } else {
                    data.selection = SelectionState::Caret(ctx.caret.index.unwrap_or(CaretIndex::ZERO));
                };
            }
            UndoFullOp::Op(UndoOp::Redo) => {
                TEXT.resolved().txt.set("");
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                TEXT.resolved().txt.set(data.txt.clone());

                let (selection_idx, caret_idx) = match data.selection {
                    SelectionState::Caret(c) => (None, c),
                    SelectionState::CaretSelection(s, e) => (Some(e), s),
                    SelectionState::SelectionCaret(s, e) => (Some(s), e),
                    SelectionState::PreInit => unreachable!(),
                };
                let mut caret = TEXT.resolve_caret();
                caret.set_index(caret_idx); // (re)start caret animation
                caret.selection_index = selection_idx;
            }
            UndoFullOp::Info { info } => *info = Some(Arc::new(l10n!("text-edit-op.clear", "clear").get())),
            UndoFullOp::Merge {
                next_data,
                within_undo_interval,
                merged,
                ..
            } => *merged = within_undo_interval && next_data.is::<Cleared>(),
        })
    }

    /// Replace operation.
    ///
    /// The `select_before` is removed, and `insert` inserted at the `select_before.start`, after insertion
    /// the `select_after` is applied, you can use an empty insert to just remove.
    ///
    /// All indexes are snapped to the nearest grapheme, you can use empty ranges to just position the caret.
    pub fn replace(mut select_before: ops::Range<usize>, insert: impl Into<Txt>, mut select_after: ops::Range<usize>) -> Self {
        let mut insert = insert.into();
        let mut removed = Txt::from_static("");

        Self::new((), move |_, op| match op {
            UndoFullOp::Init { redo } => {
                let ctx = TEXT.resolved();

                select_before.start = ctx.segmented_text.snap_grapheme_boundary(select_before.start);
                select_before.end = ctx.segmented_text.snap_grapheme_boundary(select_before.end);

                ctx.txt.with(|t| {
                    removed = Txt::from_str(&t[select_before.clone()]);
                });

                Self::apply_max_count(redo, &ctx.txt, select_before.clone(), &mut insert);
            }
            UndoFullOp::Op(UndoOp::Redo) => {
                TEXT.resolved().txt.modify(clmv!(select_before, insert, |args| {
                    args.to_mut().replace_range(select_before, insert.as_str());
                }));

                TEXT.resolve_caret().set_char_selection(select_after.start, select_after.end);
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let ctx = TEXT.resolved();

                select_after.start = ctx.segmented_text.snap_grapheme_boundary(select_after.start);
                select_after.end = ctx.segmented_text.snap_grapheme_boundary(select_after.end);

                ctx.txt.modify(clmv!(select_after, removed, |args| {
                    args.to_mut().replace_range(select_after, removed.as_str());
                }));

                drop(ctx);
                TEXT.resolve_caret().set_char_selection(select_before.start, select_before.end);
            }
            UndoFullOp::Info { info } => *info = Some(Arc::new(l10n!("text-edit-op.replace", "replace").get())),
            UndoFullOp::Merge { .. } => {}
        })
    }

    /// Applies [`TEXT_TRANSFORM_VAR`] and [`WHITE_SPACE_VAR`] to the text.
    pub fn apply_transforms() -> Self {
        let mut prev = Txt::from_static("");
        let mut transform = None::<(TextTransformFn, WhiteSpace)>;
        Self::new((), move |_, op| match op {
            UndoFullOp::Init { .. } => {}
            UndoFullOp::Op(UndoOp::Redo) => {
                let (t, w) = transform.get_or_insert_with(|| (TEXT_TRANSFORM_VAR.get(), WHITE_SPACE_VAR.get()));

                let ctx = TEXT.resolved();

                let new_txt = ctx.txt.with(|txt| {
                    let transformed = t.transform(txt);
                    let white_spaced = w.transform(transformed.as_ref());
                    if let Cow::Owned(w) = white_spaced {
                        Some(w)
                    } else if let Cow::Owned(t) = transformed {
                        Some(t)
                    } else {
                        None
                    }
                });

                if let Some(t) = new_txt {
                    if ctx.txt.with(|t| t != prev.as_str()) {
                        prev = ctx.txt.get();
                    }
                    ctx.txt.set(t);
                }
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let ctx = TEXT.resolved();

                if ctx.txt.with(|t| t != prev.as_str()) {
                    ctx.txt.set(prev.clone());
                }
            }
            UndoFullOp::Info { info } => *info = Some(Arc::new(l10n!("text-edit-op.transform", "transform").get())),
            UndoFullOp::Merge { .. } => {}
        })
    }

    fn call(self) -> bool {
        {
            let mut op = self.0.lock();
            let op = &mut *op;

            let mut redo = true;
            (op.op)(&mut *op.data, UndoFullOp::Init { redo: &mut redo });
            if !redo {
                return false;
            }

            (op.op)(&mut *op.data, UndoFullOp::Op(UndoOp::Redo));
        }

        if !OBSCURE_TXT_VAR.get() {
            UNDO.register(UndoTextEditOp::new(self));
        }
        true
    }

    pub(super) fn call_edit_op(self) {
        let registered = self.call();
        if registered && !TEXT.resolved().pending_edit {
            TEXT.resolve().pending_edit = true;
            WIDGET.update(); // in case the edit does not actually change the text.
        }
    }
}
/// Used by `TextEditOp::insert`, `backspace` and `delete`.
#[derive(Clone, Copy, Default)]
enum SelectionState {
    #[default]
    PreInit,
    Caret(CaretIndex),
    CaretSelection(CaretIndex, CaretIndex),
    SelectionCaret(CaretIndex, CaretIndex),
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

    pub(super) fn call(&self) {
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        (op.op)(&mut *op.data, UndoFullOp::Op(self.exec_op))
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

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        let mut info = None;
        (op.op)(&mut *op.data, UndoFullOp::Info { info: &mut info });

        info.unwrap_or_else(|| Arc::new(l10n!("text-edit-op.generic", "text edit").get()))
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn merge(self: Box<Self>, mut args: UndoActionMergeArgs) -> Result<Box<dyn UndoAction>, (Box<dyn UndoAction>, Box<dyn UndoAction>)> {
        if let Some(next) = args.next.as_any().downcast_mut::<Self>() {
            let mut merged = false;

            {
                let mut op = self.edit_op.0.lock();
                let op = &mut *op;

                let mut next_op = next.edit_op.0.lock();

                (op.op)(
                    &mut *op.data,
                    UndoFullOp::Merge {
                        next_data: &mut *next_op.data,
                        prev_timestamp: args.prev_timestamp,
                        within_undo_interval: args.within_undo_interval,
                        merged: &mut merged,
                    },
                );
            }

            if merged {
                return Ok(self);
            }
        }

        Err((self, args.next))
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

    fn info(&mut self) -> Arc<dyn UndoInfo> {
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        let mut info = None;
        (op.op)(&mut *op.data, UndoFullOp::Info { info: &mut info });

        info.unwrap_or_else(|| Arc::new(l10n!("text-edit-op.generic", "text edit").get()))
    }
}

/// Represents a text caret/selection operation that can be send to an editable text using [`SELECT_CMD`].
///
/// The provided operations work in rich texts by default, unless they are named with prefix `local_`. In
/// rich text contexts the operation may generate other `SELECT_CMD` requests as it propagates to all involved component texts.
/// The `local_` operations are for use by other operations only, direct use inside rich text requires updating the rich text state
/// to match.
#[derive(Clone)]
pub struct TextSelectOp {
    op: Arc<Mutex<dyn FnMut() + Send>>,
}
impl fmt::Debug for TextSelectOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSelectOp").finish_non_exhaustive()
    }
}
impl TextSelectOp {
    /// Clear selection and move the caret to the next insert index.
    ///
    /// This is the `Right` key operation.
    pub fn next() -> Self {
        rich_clear_next_prev(true, false)
    }

    /// Extend or shrink selection by moving the caret to the next insert index.
    ///
    /// This is the `SHIFT+Right` key operation.
    pub fn select_next() -> Self {
        rich_select_next_prev(true, false)
    }

    /// Clear selection and move the caret to the previous insert index.
    ///
    /// This is the `Left` key operation.
    pub fn prev() -> Self {
        rich_clear_next_prev(false, false)
    }

    /// Extend or shrink selection by moving the caret to the previous insert index.
    ///
    /// This is the `SHIFT+Left` key operation.
    pub fn select_prev() -> Self {
        rich_select_next_prev(false, false)
    }

    /// Clear selection and move the caret to the next word insert index.
    ///
    /// This is the `CTRL+Right` shortcut operation.
    pub fn next_word() -> Self {
        rich_clear_next_prev(true, true)
    }
    /// Extend or shrink selection by moving the caret to the next word insert index.
    ///
    /// This is the `CTRL+SHIFT+Right` shortcut operation.
    pub fn select_next_word() -> Self {
        rich_select_next_prev(true, true)
    }
    /// Clear selection and move the caret to the previous word insert index.
    ///
    /// This is the `CTRL+Left` shortcut operation.
    pub fn prev_word() -> Self {
        rich_clear_next_prev(false, true)
    }

    /// Extend or shrink selection by moving the caret to the previous word insert index.
    ///
    /// This is the `CTRL+SHIFT+Left` shortcut operation.
    pub fn select_prev_word() -> Self {
        rich_select_next_prev(false, true)
    }

    /// Clear selection and move the caret to the nearest insert index on the previous line.
    ///
    /// This is the `Up` key operation.
    pub fn line_up() -> Self {
        rich_up_down(true, false, false)
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the previous line.
    ///
    /// This is the `SHIFT+Up` key operation.
    pub fn select_line_up() -> Self {
        rich_up_down(false, false, false)
    }

    /// Clear selection and move the caret to the nearest insert index on the next line.
    ///
    /// This is the `Down` key operation.
    pub fn line_down() -> Self {
        rich_up_down(true, true, false)
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the next line.
    ///
    /// This is the `SHIFT+Down` key operation.
    pub fn select_line_down() -> Self {
        rich_up_down(false, true, false)
    }

    /// Clear selection and move the caret one viewport up.
    ///
    /// This is the `PageUp` key operation.
    pub fn page_up() -> Self {
        rich_up_down(true, false, true)
    }

    /// Extend or shrink selection by moving the caret one viewport up.
    ///
    /// This is the `SHIFT+PageUp` key operation.
    pub fn select_page_up() -> Self {
        rich_up_down(false, false, true)
    }

    /// Clear selection and move the caret one viewport down.
    ///
    /// This is the `PageDown` key operation.
    pub fn page_down() -> Self {
        rich_up_down(true, true, true)
    }

    /// Extend or shrink selection by moving the caret one viewport down.
    ///
    /// This is the `SHIFT+PageDown` key operation.
    pub fn select_page_down() -> Self {
        rich_up_down(false, true, true)
    }

    /// Clear selection and move the caret to the start of the line.
    ///
    /// This is the `Home` key operation.
    pub fn line_start() -> Self {
        rich_line_start_end(true, false)
    }

    /// Extend or shrink selection by moving the caret to the start of the line.
    ///
    /// This is the `SHIFT+Home` key operation.
    pub fn select_line_start() -> Self {
        rich_line_start_end(false, false)
    }

    /// Clear selection and move the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `End` key operation.
    pub fn line_end() -> Self {
        rich_line_start_end(true, true)
    }

    /// Extend or shrink selection by moving the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `SHIFT+End` key operation.
    pub fn select_line_end() -> Self {
        rich_line_start_end(false, true)
    }

    /// Clear selection and move the caret to the text start.
    ///
    /// This is the `CTRL+Home` shortcut operation.
    pub fn text_start() -> Self {
        rich_text_start_end(true, false)
    }

    /// Extend or shrink selection by moving the caret to the text start.
    ///
    /// This is the `CTRL+SHIFT+Home` shortcut operation.
    pub fn select_text_start() -> Self {
        rich_text_start_end(false, false)
    }

    /// Clear selection and move the caret to the text end.
    ///
    /// This is the `CTRL+End` shortcut operation.
    pub fn text_end() -> Self {
        rich_text_start_end(true, true)
    }

    /// Extend or shrink selection by moving the caret to the text end.
    ///
    /// This is the `CTRL+SHIFT+End` shortcut operation.
    pub fn select_text_end() -> Self {
        rich_text_start_end(false, true)
    }

    /// Clear selection and move the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down operation.
    pub fn nearest_to(window_point: DipPoint) -> Self {
        rich_nearest_char_word_to(true, window_point, false)
    }

    /// Extend or shrink selection by moving the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down when holding SHIFT operation.
    pub fn select_nearest_to(window_point: DipPoint) -> Self {
        rich_nearest_char_word_to(false, window_point, false)
    }

    /// Replace or extend selection with the word nearest to the `window_point`
    ///
    /// This is the mouse primary button double click.
    pub fn select_word_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        rich_nearest_char_word_to(replace_selection, window_point, true)
    }

    /// Replace or extend selection with the line nearest to the `window_point`
    ///
    /// This is the mouse primary button triple click.
    pub fn select_line_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        rich_nearest_line_to(replace_selection, window_point)
    }

    /// Extend or shrink selection by moving the caret index or caret selection index to the insert point nearest to `window_point`.
    ///
    /// This is the touch selection caret drag operation.
    pub fn select_index_nearest_to(window_point: DipPoint, move_selection_index: bool) -> Self {
        rich_selection_index_nearest_to(window_point, move_selection_index)
    }

    /// Select the full text.
    pub fn select_all() -> Self {
        Self::new_rich(
            |ctx| (ctx.leaves_rev().next().map(|w| w.id()).unwrap_or_else(|| WIDGET.id()), ()),
            |()| {
                (
                    CaretIndex {
                        index: TEXT.resolved().segmented_text.text().len(),
                        line: 0,
                    },
                    (),
                )
            },
            |ctx, ()| Some((ctx.leaves().next().map(|w| w.id()).unwrap_or_else(|| WIDGET.id()), ())),
            |()| Some(CaretIndex::ZERO),
        )
    }

    /// Clear selection and keep the caret at the same position.
    ///
    /// This is the `Esc` shortcut operation.
    pub fn clear_selection() -> Self {
        Self::new_rich(
            |ctx| (ctx.caret.index.unwrap_or_else(|| WIDGET.id()), ()),
            |()| (TEXT.resolved().caret.index.unwrap_or(CaretIndex::ZERO), ()),
            |_, ()| None,
            |()| None,
        )
    }
}

/// Operations that ignore the rich text context, for internal use only.
impl TextSelectOp {
    /// Like [`next`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`next`]: Self::next
    pub fn local_next() -> Self {
        Self::new(|| {
            local_clear_next_prev(true, false);
        })
    }

    /// Like [`select_next`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_next`]: Self::select_next
    pub fn local_select_next() -> Self {
        Self::new(|| {
            local_select_next_prev(true, false);
        })
    }

    /// Like [`prev`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`prev`]: Self::prev
    pub fn local_prev() -> Self {
        Self::new(|| {
            local_clear_next_prev(false, false);
        })
    }

    /// Like [`select_prev`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_prev`]: Self::select_prev
    pub fn local_select_prev() -> Self {
        Self::new(|| {
            local_select_next_prev(false, false);
        })
    }

    /// Like [`next_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`next_word`]: Self::next_word
    pub fn local_next_word() -> Self {
        Self::new(|| {
            local_clear_next_prev(true, true);
        })
    }

    /// Like [`select_next_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_next_word`]: Self::select_next_word
    pub fn local_select_next_word() -> Self {
        Self::new(|| {
            local_select_next_prev(true, true);
        })
    }

    /// Like [`prev_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`prev_word`]: Self::prev_word
    pub fn local_prev_word() -> Self {
        Self::new(|| {
            local_clear_next_prev(false, true);
        })
    }

    /// Like [`select_prev_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_prev_word`]: Self::select_prev_word
    pub fn local_select_prev_word() -> Self {
        Self::new(|| {
            local_select_next_prev(false, true);
        })
    }

    /// Like [`line_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`line_start`]: Self::line_start
    pub fn local_line_start() -> Self {
        Self::new(|| local_line_start_end(true, false))
    }

    /// Like [`select_line_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_start`]: Self::select_line_start
    pub fn local_select_line_start() -> Self {
        Self::new(|| local_line_start_end(false, false))
    }

    /// Like [`line_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`line_end`]: Self::line_end
    pub fn local_line_end() -> Self {
        Self::new(|| local_line_start_end(true, true))
    }

    /// Like [`select_line_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_end`]: Self::select_line_end
    pub fn local_select_line_end() -> Self {
        Self::new(|| local_line_start_end(false, true))
    }

    /// Like [`text_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`text_start`]: Self::text_start
    pub fn local_text_start() -> Self {
        Self::new(|| local_text_start_end(true, false))
    }

    /// Like [`select_text_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_text_start`]: Self::select_text_start
    pub fn local_select_text_start() -> Self {
        Self::new(|| local_text_start_end(false, false))
    }

    /// Like [`text_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`text_end`]: Self::text_end
    pub fn local_text_end() -> Self {
        Self::new(|| local_text_start_end(true, true))
    }

    /// Like [`select_text_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_text_end`]: Self::select_text_end
    pub fn local_select_text_end() -> Self {
        Self::new(|| local_text_start_end(false, true))
    }

    /// Like [`select_all`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_all`]: Self::select_all
    pub fn local_select_all() -> Self {
        Self::new(|| {
            let len = TEXT.resolved().segmented_text.text().len();
            let mut caret = TEXT.resolve_caret();
            caret.set_char_selection(0, len);
            caret.skip_next_scroll = true;
        })
    }

    /// Like [`clear_selection`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`clear_selection`]: Self::clear_selection
    pub fn local_clear_selection() -> Self {
        Self::new(|| {
            let mut ctx = TEXT.resolve_caret();
            ctx.clear_selection();
        })
    }

    /// Like [`line_up`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`line_up`]: Self::line_up
    pub fn local_line_up() -> Self {
        Self::new(|| local_line_up_down(true, -1))
    }

    /// Like [`select_line_up`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_up`]: Self::select_line_up
    pub fn local_select_line_up() -> Self {
        Self::new(|| local_line_up_down(false, -1))
    }

    /// Like [`line_down`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`line_down`]: Self::line_down
    pub fn local_line_down() -> Self {
        Self::new(|| local_line_up_down(true, 1))
    }

    /// Like [`select_line_down`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_down`]: Self::select_line_down
    pub fn local_select_line_down() -> Self {
        Self::new(|| local_line_up_down(false, 1))
    }

    /// Like [`page_up`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`page_up`]: Self::page_up
    pub fn local_page_up() -> Self {
        Self::new(|| local_page_up_down(true, -1))
    }

    /// Like [`select_page_up`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_page_up`]: Self::select_page_up
    pub fn local_select_page_up() -> Self {
        Self::new(|| local_page_up_down(false, -1))
    }

    /// Like [`page_down`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`page_down`]: Self::page_down
    pub fn local_page_down() -> Self {
        Self::new(|| local_page_up_down(true, 1))
    }

    /// Like [`select_page_down`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_page_down`]: Self::select_page_down
    pub fn local_select_page_down() -> Self {
        Self::new(|| local_page_up_down(false, 1))
    }

    /// Like [`nearest_to`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`nearest_to`]: Self::nearest_to
    pub fn local_nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            local_nearest_to(true, window_point);
        })
    }

    /// Like [`select_nearest_to`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_nearest_to`]: Self::select_nearest_to
    pub fn local_select_nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            local_nearest_to(false, window_point);
        })
    }

    /// Like [`select_index_nearest_to`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_index_nearest_to`]: Self::select_index_nearest_to
    pub fn local_select_index_nearest_to(window_point: DipPoint, move_selection_index: bool) -> Self {
        Self::new(move || {
            local_select_index_nearest_to(window_point, move_selection_index);
        })
    }

    /// Like [`select_word_nearest_to`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_word_nearest_to`]: Self::select_word_nearest_to
    pub fn local_select_word_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || local_select_line_word_nearest_to(replace_selection, true, window_point))
    }

    /// Like [`select_line_nearest_to`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_nearest_to`]: Self::select_line_nearest_to
    pub fn local_select_line_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || local_select_line_word_nearest_to(replace_selection, false, window_point))
    }
}

impl TextSelectOp {
    /// New text select operation.
    ///
    /// The editable text widget that handles [`SELECT_CMD`] will call `op` during event handling in
    /// the [`node::layout_text`] context. You can position the caret using [`TEXT.resolve_caret`] and [`TEXT.resolve_rich_caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    ///
    /// [`TEXT.resolve_caret`]: super::node::TEXT::resolve_caret
    /// [`TEXT.resolve_rich_caret`]: super::node::TEXT::resolve_rich_caret
    pub fn new(op: impl FnMut() + Send + 'static) -> Self {
        Self {
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// New text selection operation with helpers for implementing rich selection.
    ///
    /// The input closures are:
    ///
    /// * `rich_caret_index` - Called if the op executes inside a rich text, must return the leaf widget that will contain the rich text caret.
    /// * `local_caret_index` - Called in the caret widget context, must return the local caret index.
    /// * `rich_selection_index` - Called if the op executes inside a rich text, must return the leaf widget that will contain the rich text selection end.
    /// * `local_selection_index` - Called in selection end widget context, must return the local selection end index.
    ///
    /// Data can be passed between each stage with types `D0` from `rich_caret_index` to `local_caret_index`, `D1` from `local_caret_index` to
    /// `rich_selection_index` and `D2` from `rich_selection_index` to `local_selection_index`.
    ///
    /// If the op is not called inside a rich text only `local_caret_index` and `local_selection_index` are called with the default data values.
    ///
    /// The rich selection is updated if needed. If the local caret or selection index of an widget is set to 0(start) it is automatically corrected
    /// to the end of the previous rich leaf.
    pub fn new_rich<D0, D1, D2>(
        rich_caret_index: impl FnOnce(&RichText) -> (WidgetId, D0) + Send + 'static,
        local_caret_index: impl FnOnce(D0) -> (CaretIndex, D1) + Send + 'static,
        rich_selection_index: impl FnOnce(&RichText, D1) -> Option<(WidgetId, D2)> + Send + 'static,
        local_selection_index: impl FnOnce(D2) -> Option<CaretIndex> + Send + 'static,
    ) -> Self
    where
        D0: Default + Send + 'static,
        D1: Send + 'static,
        D2: Default + Send + 'static,
    {
        let mut f0 = Some(rich_caret_index);
        let mut f1 = Some(local_caret_index);
        let mut f2 = Some(rich_selection_index);
        let mut f3 = Some(local_selection_index);
        Self::new(move || {
            if let Some(ctx) = TEXT.try_rich() {
                rich_select_op_start(ctx, f0.take().unwrap(), f1.take().unwrap(), f2.take().unwrap(), f3.take().unwrap());
            } else {
                let (index, _) = f1.take().unwrap()(D0::default());
                let selection_index = f3.take().unwrap()(D2::default());
                let mut ctx = TEXT.resolve_caret();
                ctx.selection_index = selection_index;
                ctx.set_index(index);
            }
        })
    }

    pub(super) fn call(self) {
        (self.op.lock())();
    }
}

fn rich_select_op_start<D0: Send + 'static, D1: Send + 'static, D2: Send + 'static>(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: impl FnOnce(&RichText) -> (WidgetId, D0),
    local_caret_index: impl FnOnce(D0) -> (CaretIndex, D1) + Send + 'static,
    rich_selection_index: impl FnOnce(&RichText, D1) -> Option<(WidgetId, D2)> + Send + 'static,
    local_selection_index: impl FnOnce(D2) -> Option<CaretIndex> + Send + 'static,
) {
    let (index, d0) = rich_caret_index(&ctx);
    if index == WIDGET.id() {
        rich_select_op_get_caret(ctx, index, d0, local_caret_index, rich_selection_index, local_selection_index);
    } else {
        let mut d0 = Some(d0);
        let mut f0 = Some(local_caret_index);
        let mut f1 = Some(rich_selection_index);
        let mut f2 = Some(local_selection_index);
        notify_leaf_select_op(
            index,
            TextSelectOp::new(move || {
                if let Some(ctx) = TEXT.try_rich() {
                    if index == WIDGET.id() {
                        rich_select_op_get_caret(
                            ctx,
                            index,
                            d0.take().unwrap(),
                            f0.take().unwrap(),
                            f1.take().unwrap(),
                            f2.take().unwrap(),
                        );
                    }
                }
            }),
        );
    }
}
fn rich_select_op_get_caret<D0, D1, D2: Send + 'static>(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: WidgetId,
    d0: D0,
    local_caret_index: impl FnOnce(D0) -> (CaretIndex, D1),
    rich_selection_index: impl FnOnce(&RichText, D1) -> Option<(WidgetId, D2)>,
    local_selection_index: impl FnOnce(D2) -> Option<CaretIndex> + Send + 'static,
) {
    let (index, d1) = local_caret_index(d0);
    {
        let mut ctx = TEXT.resolve_caret();
        ctx.set_index(index);
    }

    match rich_selection_index(&ctx, d1) {
        Some((selection_index, d2)) => {
            if selection_index == WIDGET.id() {
                rich_select_op_get_selection(ctx, (rich_caret_index, index), selection_index, d2, local_selection_index);
            } else {
                let mut d2 = Some(d2);
                let mut f0 = Some(local_selection_index);
                notify_leaf_select_op(
                    selection_index,
                    TextSelectOp::new(move || {
                        if let Some(ctx) = TEXT.try_rich() {
                            if selection_index == WIDGET.id() {
                                rich_select_op_get_selection(
                                    ctx,
                                    (rich_caret_index, index),
                                    selection_index,
                                    d2.take().unwrap(),
                                    f0.take().unwrap(),
                                );
                            }
                        }
                    }),
                );
            }
        }
        None => rich_select_op_finish(ctx, (rich_caret_index, index), None),
    }
}
fn rich_select_op_get_selection<D2>(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: (WidgetId, CaretIndex),
    rich_selection_index: WidgetId,
    d2: D2,
    local_selection_index: impl FnOnce(D2) -> Option<CaretIndex>,
) {
    if let Some(index) = local_selection_index(d2) {
        let mut local_ctx = TEXT.resolve_caret();
        local_ctx.selection_index = Some(index);
        local_ctx.index_version += 1;
        rich_select_op_finish(ctx, rich_caret_index, Some((rich_selection_index, index)));
    } else {
        rich_select_op_finish(ctx, rich_caret_index, None);
    }
}
fn rich_select_op_finish(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: (WidgetId, CaretIndex),
    rich_selection_index: Option<(WidgetId, CaretIndex)>,
) {
    if let Some(mut index) = ctx.leaf_info(rich_caret_index.0) {
        if rich_caret_index.1.index == 0 {
            // index 0 is the end of previous leaf
            if let Some(prev) = index.rich_text_prev().next() {
                index = prev;
                notify_leaf_select_op(
                    index.id(),
                    TextSelectOp::new(move || {
                        let end = TEXT.resolved().segmented_text.text().len();
                        TEXT.resolve_caret().set_char_index(end);
                    }),
                );
            }
        }
        if let Some(rich_selection_index) = rich_selection_index {
            if let Some(mut selection) = ctx.leaf_info(rich_selection_index.0) {
                if rich_selection_index.1.index == 0 {
                    // selection 0 is the end of the previous leaf
                    if let Some(prev) = selection.rich_text_prev().next() {
                        selection = prev;
                        notify_leaf_select_op(
                            selection.id(),
                            TextSelectOp::new(move || {
                                let end = TEXT.resolved().segmented_text.text().len();
                                TEXT.resolve_caret().set_char_index(end);
                            }),
                        );
                    }
                }

                drop(ctx);
                TEXT.resolve_rich_caret().update_selection(&index, Some(&selection), false, false);
            }
        } else {
            // no selection

            drop(ctx);
            TEXT.resolve_rich_caret().update_selection(&index, None, false, false);
        }
    }
}

fn rich_clear_next_prev(is_next: bool, is_word: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        // get prev/next leaf widget
        move |ctx| {
            if let Some(i) = ctx.caret_index_info()
                && let Some(s) = ctx.caret_selection_index_info()
            {
                // clear selection, next places caret at end of selection, prev at start

                let (a, b) = match i.cmp_sibling_in(&s, &i.root()).unwrap() {
                    cmp::Ordering::Less | cmp::Ordering::Equal => (&i, &s),
                    cmp::Ordering::Greater => (&s, &i),
                };

                let c = if is_next { b } else { a };

                (c.id(), false) // false to just collapse to selection
            } else {
                // no selection, actually move caret

                let local_ctx = TEXT.resolved();
                if is_next {
                    let index = local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index;
                    if index == local_ctx.segmented_text.text().len() {
                        // next from end, check if has next sibling
                        if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                            if let Some(next) = info.rich_text_next().next() {
                                return (next.id(), true);
                            }
                        }
                    }

                    // caret stays inside
                    (WIDGET.id(), false)
                } else {
                    // !is_next

                    let cutout = if is_word { local_ctx.segmented_text.next_word_index(0) } else { 1 };
                    if local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index <= cutout {
                        // next moves to the start (or is already in start)

                        if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                            if let Some(prev) = info.rich_text_prev().next() {
                                return (prev.id(), true);
                            }
                        }
                    }

                    (WIDGET.id(), false)
                }
            }
        },
        // get caret in the prev/next widget
        move |is_from_sibling| {
            if is_from_sibling {
                if is_next {
                    (CaretIndex { index: 1, line: 0 }, ())
                } else {
                    let local_ctx = TEXT.resolved();
                    (
                        CaretIndex {
                            index: local_ctx.segmented_text.text().len(),
                            line: 0,
                        },
                        (),
                    )
                }
            } else {
                local_clear_next_prev(is_next, is_word);
                (TEXT.resolved().caret.index.unwrap_or(CaretIndex::ZERO), ())
            }
        },
        |_, _| None,
        |()| None,
    )
}
fn local_clear_next_prev(is_next: bool, is_word: bool) {
    // compute next caret position
    let ctx = TEXT.resolved();
    let current_index = ctx.caret.index.unwrap_or(CaretIndex::ZERO);
    let mut next_index = current_index;
    if let Some(selection) = ctx.caret.selection_range() {
        next_index.index = if is_next { selection.end.index } else { selection.start.index };
    } else {
        next_index.index = if is_next {
            let from = current_index.index;
            if is_word {
                ctx.segmented_text.next_word_index(from)
            } else {
                ctx.segmented_text.next_insert_index(from)
            }
        } else {
            let from = current_index.index;
            if is_word {
                ctx.segmented_text.prev_word_index(from)
            } else {
                ctx.segmented_text.prev_insert_index(from)
            }
        };
    }

    drop(ctx);

    let mut ctx = TEXT.resolve_caret();
    ctx.clear_selection();
    ctx.set_index(next_index);
    ctx.used_retained_x = false;
}

fn rich_select_next_prev(is_next: bool, is_word: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        // get prev/next leaf widget
        move |ctx| {
            let local_ctx = TEXT.resolved();

            let index = local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index;

            if is_next {
                if index == local_ctx.segmented_text.text().len() {
                    // next from end
                    if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                        if let Some(next) = info.rich_text_next().next() {
                            return (next.id(), true);
                        }
                    }
                }
            } else {
                // !is_next

                let cutout = if is_word { local_ctx.segmented_text.next_word_index(0) } else { 1 };
                if local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index <= cutout {
                    // next moves to the start (or is already in start)
                    if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                        if let Some(prev) = info.rich_text_prev().next() {
                            return (prev.id(), true);
                        }
                    }
                }
            }
            (WIDGET.id(), false)
        },
        // get caret in the prev/next widget
        move |is_from_sibling| {
            let id = WIDGET.id();
            if is_from_sibling {
                if is_next {
                    // caret was at sibling end
                    (CaretIndex { index: 1, line: 0 }, id)
                } else {
                    // caret was at sibling start or moves to sibling start (that is the same as our end)
                    let len = TEXT.resolved().segmented_text.text().len();
                    (CaretIndex { index: len, line: 0 }, id)
                }
            } else {
                local_select_next_prev(is_next, is_word);
                (TEXT.resolved().caret.index.unwrap_or(CaretIndex::ZERO), id)
            }
        },
        // get selection_index leaf widget
        |ctx, index| Some((ctx.caret.selection_index.unwrap_or(index), ())),
        // get local selection_index
        |()| {
            let local_ctx = TEXT.resolved();
            Some(
                local_ctx
                    .caret
                    .selection_index
                    .unwrap_or(local_ctx.caret.index.unwrap_or(CaretIndex::ZERO)),
            )
        },
    )
}
fn local_select_next_prev(is_next: bool, is_word: bool) {
    // compute next caret position
    let ctx = TEXT.resolved();
    let current_index = ctx.caret.index.unwrap_or(CaretIndex::ZERO);
    let mut next_index = current_index;
    next_index.index = if is_next {
        if is_word {
            ctx.segmented_text.next_word_index(current_index.index)
        } else {
            ctx.segmented_text.next_insert_index(current_index.index)
        }
    } else {
        // is_prev
        if is_word {
            ctx.segmented_text.prev_word_index(current_index.index)
        } else {
            ctx.segmented_text.prev_insert_index(current_index.index)
        }
    };
    drop(ctx);

    let mut ctx = TEXT.resolve_caret();
    if ctx.selection_index.is_none() {
        ctx.selection_index = Some(current_index);
    }
    ctx.set_index(next_index);
    ctx.used_retained_x = false;
}

fn rich_up_down(clear_selection: bool, is_down: bool, is_page: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        move |ctx| {
            let resolved = TEXT.resolved();
            let laidout = TEXT.laidout();

            let local_line_i = resolved.caret.index.unwrap_or(CaretIndex::ZERO).line;
            let last_line_i = laidout.shaped_text.lines_len().saturating_sub(1);
            let next_local_line_i = local_line_i.saturating_add_signed(if is_down { 1 } else { -1 }).min(last_line_i);

            let page_h = if is_page { laidout.viewport.height } else { Px(0) };

            let mut need_spatial_search = local_line_i == next_local_line_i; // if already at first/last line

            if !need_spatial_search {
                if is_page {
                    if let Some(local_line) = laidout.shaped_text.line(local_line_i) {
                        if is_down {
                            if let Some(last_line) = laidout.shaped_text.line(last_line_i) {
                                let max_local_y = last_line.rect().max_y() - local_line.rect().min_y();
                                need_spatial_search = max_local_y < page_h; // if page distance is greater than maximum down distance
                            }
                        } else if let Some(first_line) = laidout.shaped_text.line(0) {
                            let max_local_y = local_line.rect().max_y() - first_line.rect().min_y();
                            need_spatial_search = max_local_y < page_h; // if page distance is greater than maximum up distance
                        }
                    }
                } else if let Some(next_local_line) = laidout.shaped_text.line(next_local_line_i) {
                    let r = next_local_line.rect();
                    let x = laidout.caret_retained_x;
                    need_spatial_search = r.min_x() > x || r.max_x() < x; // if next local line does not contain ideal caret horizontally
                }
            }

            if need_spatial_search
                && let Some(local_line) = laidout.shaped_text.line(local_line_i)
                && let Some(root_info) = ctx.root_info()
            {
                // line ok, rich context ok
                let r = local_line.rect();
                let local_point = PxPoint::new(laidout.caret_retained_x, r.origin.y + r.size.height / Px(2));
                let local_info = WIDGET.info();
                let local_to_window = local_info.inner_transform();

                let local_cut_y = if is_down { local_point.y + page_h } else { local_point.y - page_h };
                let window_cut_y = local_to_window
                    .transform_point(PxPoint::new(Px(0), local_cut_y))
                    .unwrap_or_default()
                    .y;

                if let Some(window_point) = local_to_window.transform_point(local_point) {
                    // transform ok

                    // find the nearest sibling considering only the prev/next rich lines
                    let local_line_info = local_info.rich_text_line_info();
                    let filter = |other: &WidgetInfo, rect: PxRect, row_i, rows_len| {
                        if is_down {
                            if rect.max_y() < window_cut_y {
                                // rectangle is before the page y line or the the current line
                                return false;
                            }
                        } else if rect.min_y() > window_cut_y {
                            // rectangle is after the page y line or the current line
                            return false;
                        }

                        match local_info.cmp_sibling_in(other, &root_info).unwrap() {
                            cmp::Ordering::Less => {
                                // other is after local

                                if !is_down {
                                    return false;
                                }
                                if local_line_i < last_line_i {
                                    return true; // local started next line
                                }
                                for next in local_info.rich_text_next() {
                                    let line_info = next.rich_text_line_info();
                                    if line_info.starts_new_line {
                                        return true; // `other` starts new line or is after this line break
                                    }
                                    if line_info.ends_in_new_line {
                                        if &next == other {
                                            return row_i > 0; // `other` rect is not in the same line
                                        }
                                        return true; // `other` starts after this line break
                                    }
                                    if &next == other {
                                        return false; // `other` is in same line
                                    }
                                }
                                unreachable!() // filter only called if is sibling, cmp ensures that is next sibling
                            }
                            cmp::Ordering::Greater => {
                                // other is before local

                                if is_down {
                                    return false;
                                }
                                if local_line_i > 0 || local_line_info.starts_new_line {
                                    return true; // local started line, all prev wgt in prev lines
                                }
                                for prev in local_info.rich_text_prev() {
                                    let line_info = prev.rich_text_line_info();
                                    if line_info.ends_in_new_line {
                                        if &prev == other {
                                            return row_i < rows_len - 1; // `other` rect is not in the same line
                                        }
                                        return true; // `other` ends before this linebreak
                                    }
                                    if line_info.starts_new_line {
                                        return &prev != other; // `other` starts the line (same line) or not (is before)
                                    }

                                    if &prev == other {
                                        return false; // `other` is in same line
                                    }
                                }
                                unreachable!()
                            }
                            cmp::Ordering::Equal => false,
                        }
                    };
                    if let Some(next) = root_info.rich_text_nearest_leaf_filtered(window_point, filter) {
                        // found nearest sibling on the next/prev rich lines

                        let next_info = next.clone();

                        // get the next(wgt) local line that is in the next/prev rich line
                        let mut next_line = 0;
                        if let Some(next_inline_rows_len) = next_info.bounds_info().inline().map(|i| i.rows.len()) {
                            if next_inline_rows_len > 1 {
                                if is_down {
                                    // next is logical next

                                    if local_line_i == last_line_i {
                                        // local did not start next line

                                        for l_next in local_info.rich_text_next() {
                                            let line_info = l_next.rich_text_line_info();
                                            if line_info.starts_new_line || line_info.ends_in_new_line {
                                                // found rich line end
                                                if l_next == next {
                                                    // its inside the `next`, meaning it starts on the same rich line
                                                    next_line = 1;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    // next is up (logical prev)
                                    next_line = next_inline_rows_len - 1;

                                    if local_line_i == 0 && !local_line_info.starts_new_line {
                                        // local did not start current line

                                        for l_prev in local_info.rich_text_prev() {
                                            let line_info = l_prev.rich_text_line_info();
                                            if line_info.starts_new_line || line_info.ends_in_new_line {
                                                // found rich line start
                                                if l_prev == next {
                                                    // its inside the `next`, meaning it ends on the same rich line
                                                    next_line -= 1;
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        return (next.id(), Some((window_point.x, next_line)));
                    }
                }
            }

            // when can't go down within local goes to text start/end
            let mut cant_go_down_up = if is_down {
                // if already at last line
                local_line_i == last_line_i
            } else {
                // if already at first line
                local_line_i == 0
            };
            if is_page && !cant_go_down_up {
                if let Some(local_line) = laidout.shaped_text.line(local_line_i) {
                    if is_down {
                        if let Some(last_line) = laidout.shaped_text.line(last_line_i) {
                            // if page down distance greater than distance to last line
                            let max_local_y = last_line.rect().max_y() - local_line.rect().min_y();
                            cant_go_down_up = max_local_y < page_h;
                        }
                    } else if let Some(first_line) = laidout.shaped_text.line(0) {
                        // if page up distance greater than distance to first line
                        let max_local_y = local_line.rect().max_y() - first_line.rect().min_y();
                        cant_go_down_up = max_local_y < page_h;
                    }
                }
            }
            if cant_go_down_up {
                if is_down {
                    if let Some(end) = ctx.leaves_rev().next() {
                        return (end.id(), None);
                    }
                } else if let Some(start) = ctx.leaves().next() {
                    return (start.id(), None);
                }
            }

            (WIDGET.id(), None) // only local nav
        },
        move |rich_request| {
            if let Some((window_x, line_i)) = rich_request {
                let local_x = WIDGET
                    .info()
                    .inner_transform()
                    .inverse()
                    .and_then(|t| t.transform_point(PxPoint::new(window_x, Px(0))))
                    .unwrap_or_default()
                    .x;
                TEXT.set_caret_retained_x(local_x);
                let local_ctx = TEXT.laidout();
                if let Some(line) = local_ctx.shaped_text.line(line_i) {
                    let index = match line.nearest_seg(local_x) {
                        Some(s) => s.nearest_char_index(local_x, TEXT.resolved().segmented_text.text()),
                        None => line.text_range().end,
                    };
                    let index = CaretIndex { index, line: line_i };
                    TEXT.resolve_caret().used_retained_x = true; // new_rich does not set this
                    return (index, ());
                }
            }
            let diff = if is_down { 1 } else { -1 };
            if is_page {
                local_page_up_down(clear_selection, diff);
            } else {
                local_line_up_down(clear_selection, diff);
            }
            (TEXT.resolved().caret.index.unwrap(), ())
        },
        move |ctx, ()| {
            if clear_selection {
                None
            } else {
                Some((ctx.caret.selection_index.or(ctx.caret.index).unwrap_or_else(|| WIDGET.id()), ()))
            }
        },
        move |()| {
            if clear_selection {
                None
            } else {
                let local_ctx = TEXT.resolved();
                Some(
                    local_ctx
                        .caret
                        .selection_index
                        .or(local_ctx.caret.index)
                        .unwrap_or(CaretIndex::ZERO),
                )
            }
        },
    )
}
fn local_line_up_down(clear_selection: bool, diff: i8) {
    let diff = diff as isize;

    let mut caret = TEXT.resolve_caret();
    let mut i = caret.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        caret.clear_selection();
    } else if caret.selection_index.is_none() {
        caret.selection_index = Some(i);
    }
    caret.used_retained_x = true;

    let laidout = TEXT.laidout();

    if laidout.caret_origin.is_some() {
        let last_line = laidout.shaped_text.lines_len().saturating_sub(1);
        let li = i.line;
        let next_li = li.saturating_add_signed(diff).min(last_line);
        if li != next_li {
            drop(caret);
            let resolved = TEXT.resolved();
            match laidout.shaped_text.line(next_li) {
                Some(l) => {
                    i.line = next_li;
                    i.index = match l.nearest_seg(laidout.caret_retained_x) {
                        Some(s) => s.nearest_char_index(laidout.caret_retained_x, resolved.segmented_text.text()),
                        None => l.text_range().end,
                    }
                }
                None => i = CaretIndex::ZERO,
            };
            i.index = resolved.segmented_text.snap_grapheme_boundary(i.index);
            drop(resolved);
            caret = TEXT.resolve_caret();
            caret.set_index(i);
        } else if diff == -1 {
            caret.set_char_index(0);
        } else if diff == 1 {
            drop(caret);
            let len = TEXT.resolved().segmented_text.text().len();
            caret = TEXT.resolve_caret();
            caret.set_char_index(len);
        }
    }

    if caret.index.is_none() {
        caret.set_index(CaretIndex::ZERO);
        caret.clear_selection();
    }
}
fn local_page_up_down(clear_selection: bool, diff: i8) {
    let diff = diff as i32;

    let mut caret = TEXT.resolve_caret();
    let mut i = caret.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        caret.clear_selection();
    } else if caret.selection_index.is_none() {
        caret.selection_index = Some(i);
    }

    let laidout = TEXT.laidout();

    let page_y = laidout.viewport.height * Px(diff);
    caret.used_retained_x = true;
    if laidout.caret_origin.is_some() {
        let li = i.line;
        if diff == -1 && li == 0 {
            caret.set_char_index(0);
        } else if diff == 1 && li == laidout.shaped_text.lines_len() - 1 {
            drop(caret);
            let len = TEXT.resolved().segmented_text.text().len();
            caret = TEXT.resolve_caret();
            caret.set_char_index(len);
        } else if let Some(li) = laidout.shaped_text.line(li) {
            drop(caret);
            let resolved = TEXT.resolved();

            let target_line_y = li.rect().origin.y + page_y;
            match laidout.shaped_text.nearest_line(target_line_y) {
                Some(l) => {
                    i.line = l.index();
                    i.index = match l.nearest_seg(laidout.caret_retained_x) {
                        Some(s) => s.nearest_char_index(laidout.caret_retained_x, resolved.segmented_text.text()),
                        None => l.text_range().end,
                    }
                }
                None => i = CaretIndex::ZERO,
            };
            i.index = resolved.segmented_text.snap_grapheme_boundary(i.index);

            drop(resolved);
            caret = TEXT.resolve_caret();

            caret.set_index(i);
        }
    }

    if caret.index.is_none() {
        caret.set_index(CaretIndex::ZERO);
        caret.clear_selection();
    }
}

fn rich_line_start_end(clear_selection: bool, is_end: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        // get caret widget, rich line start/end
        move |ctx| {
            let from_id = WIDGET.id();
            if let Some(c) = ctx.leaf_info(WIDGET.id()) {
                let local_line = TEXT.resolved().caret.index.unwrap_or(CaretIndex::ZERO).line;
                if is_end {
                    let last_line = TEXT.laidout().shaped_text.lines_len() - 1;
                    if local_line == last_line {
                        // current line can end in a next sibling

                        let mut prev_id = c.id();
                        for c in c.rich_text_next() {
                            let line_info = c.rich_text_line_info();
                            if line_info.starts_new_line && !line_info.is_wrap_start {
                                return (prev_id, Some(from_id));
                            } else if line_info.ends_in_new_line {
                                return (c.id(), Some(from_id));
                            }
                            prev_id = c.id();
                        }

                        // text end
                        return (prev_id, Some(from_id));
                    }
                } else {
                    // !is_end

                    if local_line == 0 {
                        // current line can start in a prev sibling

                        let mut last_id = c.id();
                        let mut first = true;
                        for c in c.rich_text_self_and_prev() {
                            let line_info = c.rich_text_line_info();
                            if (line_info.starts_new_line && !line_info.is_wrap_start) || (line_info.ends_in_new_line && !first) {
                                return (c.id(), Some(from_id));
                            }
                            last_id = c.id();
                            first = false;
                        }

                        // text start
                        return (last_id, Some(from_id));
                    }
                }
            }
            (from_id, None)
        },
        // get local caret index in the rich line start/end widget
        move |from_id| {
            if let Some(from_id) = from_id {
                if from_id != WIDGET.id() {
                    // ensure the caret is at a start/end from the other sibling for `local_line_start_end`
                    if is_end {
                        TEXT.resolve_caret().index = Some(CaretIndex::ZERO);
                    } else {
                        let local_ctx = TEXT.laidout();
                        let line = local_ctx.shaped_text.lines_len() - 1;
                        let index = local_ctx.shaped_text.line(line).unwrap().text_caret_range().end;
                        drop(local_ctx);
                        TEXT.resolve_caret().index = Some(CaretIndex { index, line })
                    }
                }
            }
            local_line_start_end(clear_selection, is_end);

            (TEXT.resolved().caret.index.unwrap(), from_id)
        },
        // get the selection index widget, line selection always updates from the caret
        move |ctx, from_id| {
            if clear_selection {
                return None;
            }
            Some((ctx.caret.selection_index.or(from_id).unwrap_or_else(|| WIDGET.id()), ()))
        },
        // get the selection index
        move |()| {
            if clear_selection {
                return None;
            }
            let local_ctx = TEXT.resolved();
            Some(
                local_ctx
                    .caret
                    .selection_index
                    .or(local_ctx.caret.index)
                    .unwrap_or(CaretIndex::ZERO),
            )
        },
    )
}
fn local_line_start_end(clear_selection: bool, is_end: bool) {
    let mut ctx = TEXT.resolve_caret();
    let mut i = ctx.index.unwrap_or(CaretIndex::ZERO);

    if clear_selection {
        ctx.clear_selection();
    } else if ctx.selection_index.is_none() {
        ctx.selection_index = Some(i);
    }

    if let Some(li) = TEXT.laidout().shaped_text.line(i.line) {
        i.index = if is_end {
            li.actual_text_caret_range().end
        } else {
            li.actual_text_range().start
        };
        ctx.set_index(i);
        ctx.used_retained_x = false;
    }
}

fn rich_text_start_end(clear_selection: bool, is_end: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        move |ctx| {
            let from_id = WIDGET.id();
            let id = if is_end { ctx.leaves_rev().next() } else { ctx.leaves().next() }.map(|w| w.id());
            (id.unwrap_or(from_id), Some(from_id))
        },
        move |from_id| {
            local_text_start_end(clear_selection, is_end);
            (TEXT.resolved().caret.index.unwrap(), from_id)
        },
        // get the selection index widget, line selection always updates from the caret
        move |ctx, from_id| {
            if clear_selection {
                return None;
            }
            Some((ctx.caret.selection_index.or(from_id).unwrap_or_else(|| WIDGET.id()), ()))
        },
        // get the selection index
        move |()| {
            if clear_selection {
                return None;
            }
            let local_ctx = TEXT.resolved();
            Some(
                local_ctx
                    .caret
                    .selection_index
                    .or(local_ctx.caret.index)
                    .unwrap_or(CaretIndex::ZERO),
            )
        },
    )
}
fn local_text_start_end(clear_selection: bool, is_end: bool) {
    let idx = if is_end { TEXT.resolved().segmented_text.text().len() } else { 0 };

    let mut ctx = TEXT.resolve_caret();
    let mut i = ctx.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        ctx.clear_selection();
    } else if ctx.selection_index.is_none() {
        ctx.selection_index = Some(i);
    }
    i.index = idx;
    ctx.set_index(i);
    ctx.used_retained_x = false;
}

/// `clear_selection` is `replace_selection` for `is_word` mode.
fn rich_nearest_char_word_to(clear_selection: bool, window_point: DipPoint, is_word: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        move |ctx| {
            if let Some(root) = ctx.root_info() {
                if let Some(nearest_leaf) = root.rich_text_nearest_leaf(window_point.to_px(root.tree().scale_factor())) {
                    return (nearest_leaf.id(), ());
                }
            }
            (WIDGET.id(), ())
        },
        move |()| {
            if is_word {
                local_select_line_word_nearest_to(clear_selection, true, window_point)
            } else {
                local_nearest_to(clear_selection, window_point)
            }
            (TEXT.resolved().caret.index.unwrap(), ())
        },
        move |ctx, ()| {
            if clear_selection {
                if is_word && TEXT.resolved().caret.selection_index.is_some() {
                    Some((WIDGET.id(), ()))
                } else {
                    None
                }
            } else {
                Some((ctx.caret.selection_index.unwrap_or_else(|| WIDGET.id()), ()))
            }
        },
        move |()| {
            if clear_selection {
                if is_word { TEXT.resolved().caret.selection_index } else { None }
            } else {
                let local_ctx = TEXT.resolved();
                Some(
                    local_ctx
                        .caret
                        .selection_index
                        .or(local_ctx.caret.index)
                        .unwrap_or(CaretIndex::ZERO),
                )
            }
        },
    )
}
fn local_nearest_to(clear_selection: bool, window_point: DipPoint) {
    let mut caret = TEXT.resolve_caret();
    let mut i = caret.index.unwrap_or(CaretIndex::ZERO);

    if clear_selection {
        caret.clear_selection();
    } else if caret.selection_index.is_none() {
        caret.selection_index = Some(i);
    } else if let Some((_, is_word)) = caret.initial_selection.clone() {
        drop(caret);
        return local_select_line_word_nearest_to(false, is_word, window_point);
    }

    caret.used_retained_x = false;

    //if there was at least one layout
    let laidout = TEXT.laidout();
    if let Some(pos) = laidout
        .render_info
        .transform
        .inverse()
        .and_then(|t| t.project_point(window_point.to_px(laidout.render_info.scale_factor)))
    {
        drop(caret);
        let resolved = TEXT.resolved();

        //if has rendered
        i = match laidout.shaped_text.nearest_line(pos.y) {
            Some(l) => CaretIndex {
                line: l.index(),
                index: match l.nearest_seg(pos.x) {
                    Some(s) => s.nearest_char_index(pos.x, resolved.segmented_text.text()),
                    None => l.text_range().end,
                },
            },
            None => CaretIndex::ZERO,
        };
        i.index = resolved.segmented_text.snap_grapheme_boundary(i.index);

        drop(resolved);
        caret = TEXT.resolve_caret();

        caret.set_index(i);
    }

    if caret.index.is_none() {
        caret.set_index(CaretIndex::ZERO);
        caret.clear_selection();
    }
}

fn rich_selection_index_nearest_to(window_point: DipPoint, move_selection_index: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        move |ctx| {
            if move_selection_index {
                return (ctx.caret.index.unwrap_or_else(|| WIDGET.id()), ());
            }

            if let Some(root) = ctx.root_info() {
                if let Some(nearest_leaf) = root.rich_text_nearest_leaf(window_point.to_px(root.tree().scale_factor())) {
                    return (nearest_leaf.id(), ());
                }
            }
            (WIDGET.id(), ())
        },
        move |()| {
            if !move_selection_index {
                local_select_index_nearest_to(window_point, false);
            }
            (TEXT.resolved().caret.index.unwrap_or(CaretIndex::ZERO), ())
        },
        move |ctx, ()| {
            if !move_selection_index {
                return Some((ctx.caret.selection_index.unwrap_or_else(|| WIDGET.id()), ()));
            }

            if let Some(root) = ctx.root_info() {
                if let Some(nearest_leaf) = root.rich_text_nearest_leaf(window_point.to_px(root.tree().scale_factor())) {
                    return Some((nearest_leaf.id(), ()));
                }
            }
            Some((WIDGET.id(), ()))
        },
        move |()| {
            if move_selection_index {
                local_select_index_nearest_to(window_point, true);
            }
            Some(TEXT.resolved().caret.selection_index.unwrap_or(CaretIndex::ZERO))
        },
    )
}
fn local_select_index_nearest_to(window_point: DipPoint, move_selection_index: bool) {
    let mut caret = TEXT.resolve_caret();

    if caret.index.is_none() {
        caret.index = Some(CaretIndex::ZERO);
    }
    if caret.selection_index.is_none() {
        caret.selection_index = Some(caret.index.unwrap());
    }

    caret.used_retained_x = false;
    caret.index_version += 1;

    let laidout = TEXT.laidout();
    if let Some(pos) = laidout
        .render_info
        .transform
        .inverse()
        .and_then(|t| t.project_point(window_point.to_px(laidout.render_info.scale_factor)))
    {
        drop(caret);
        let resolved = TEXT.resolved();

        let mut i = match laidout.shaped_text.nearest_line(pos.y) {
            Some(l) => CaretIndex {
                line: l.index(),
                index: match l.nearest_seg(pos.x) {
                    Some(s) => s.nearest_char_index(pos.x, resolved.segmented_text.text()),
                    None => l.text_range().end,
                },
            },
            None => CaretIndex::ZERO,
        };
        i.index = resolved.segmented_text.snap_grapheme_boundary(i.index);

        drop(resolved);
        caret = TEXT.resolve_caret();

        if move_selection_index {
            caret.selection_index = Some(i);
        } else {
            caret.index = Some(i);
        }
    }
}

fn rich_nearest_line_to(replace_selection: bool, window_point: DipPoint) -> TextSelectOp {
    TextSelectOp::new_rich(
        move |ctx| {
            if let Some(root) = ctx.root_info() {
                let window_point = window_point.to_px(root.tree().scale_factor());
                if let Some(nearest_leaf) = root.rich_text_nearest_leaf(window_point) {
                    let mut nearest = usize::MAX;
                    let mut nearest_dist = DistanceKey::NONE_MAX;
                    let mut rows_len = 0;
                    nearest_leaf.bounds_info().visit_inner_rects::<()>(|r, i, l| {
                        rows_len = l;
                        let dist = DistanceKey::from_rect_to_point(r, window_point);
                        if dist < nearest_dist {
                            nearest_dist = dist;
                            nearest = i;
                        }
                        ops::ControlFlow::Continue(())
                    });

                    // returns the rich line end
                    if nearest < rows_len.saturating_sub(1) {
                        // rich line ends in the leaf widget
                        return (nearest_leaf.id(), Some(nearest));
                    } else {
                        // rich line starts in the leaf widget
                        let mut end = nearest_leaf.clone();
                        for next in nearest_leaf.rich_text_next() {
                            let line_info = next.rich_text_line_info();
                            if line_info.starts_new_line && !line_info.is_wrap_start {
                                return (
                                    end.id(),
                                    Some(end.bounds_info().inline().map(|i| i.rows.len().saturating_sub(1)).unwrap_or(0)),
                                );
                            }
                            end = next;
                            if line_info.ends_in_new_line {
                                break;
                            }
                        }
                        return (end.id(), Some(0));
                    }
                }
            }
            (WIDGET.id(), None)
        },
        move |rich_request| {
            if let Some(line_i) = rich_request {
                let local_ctx = TEXT.laidout();
                if let Some(line) = local_ctx.shaped_text.line(line_i) {
                    return (
                        CaretIndex {
                            index: line.actual_text_caret_range().end,
                            line: line_i,
                        },
                        line.actual_line_start().index() == 0,
                    );
                }
            }
            local_select_line_word_nearest_to(replace_selection, true, window_point);
            (TEXT.resolved().caret.index.unwrap(), false)
        },
        move |ctx, rich_select_line_start| {
            if rich_select_line_start {
                let id = WIDGET.id();
                if let Some(line_end) = ctx.leaf_info(id) {
                    let mut line_start = line_end;
                    let mut first = true;
                    for prev in line_start.rich_text_self_and_prev() {
                        let line_info = prev.rich_text_line_info();
                        line_start = prev;
                        if (line_info.starts_new_line && !line_info.is_wrap_start) || (line_info.ends_in_new_line && !first) {
                            break;
                        }
                        first = false;
                    }
                    if !replace_selection {
                        if let Some(sel) = ctx.caret_selection_index_info() {
                            if let Some(std::cmp::Ordering::Less) = sel.cmp_sibling_in(&line_start, &sel.root()) {
                                // rich line start already inside selection
                                return Some((sel.id(), false));
                            }
                        }
                    }
                    return Some((line_start.id(), line_start.id() != id));
                }
            }
            if replace_selection {
                return Some((WIDGET.id(), false));
            }
            Some((ctx.caret.selection_index.unwrap_or_else(|| WIDGET.id()), false))
        },
        move |start_of_last_line| {
            if replace_selection {
                let local_ctx = TEXT.laidout();
                let mut line_i = local_ctx.shaped_text.lines_len().saturating_sub(1);
                if !start_of_last_line {
                    if let Some(i) = TEXT.resolved().caret.index {
                        line_i = i.line;
                    }
                }
                if let Some(last_line) = local_ctx.shaped_text.line(line_i) {
                    return Some(CaretIndex {
                        index: last_line.actual_text_caret_range().start,
                        line: line_i,
                    });
                }
                None
            } else {
                let local_ctx = TEXT.resolved();
                Some(
                    local_ctx
                        .caret
                        .selection_index
                        .or(local_ctx.caret.index)
                        .unwrap_or(CaretIndex::ZERO),
                )
            }
        },
    )
}
fn local_select_line_word_nearest_to(replace_selection: bool, select_word: bool, window_point: DipPoint) {
    let mut caret = TEXT.resolve_caret();

    //if there was at least one laidout
    let laidout = TEXT.laidout();
    if let Some(pos) = laidout
        .render_info
        .transform
        .inverse()
        .and_then(|t| t.project_point(window_point.to_px(laidout.render_info.scale_factor)))
    {
        //if has rendered
        if let Some(l) = laidout.shaped_text.nearest_line(pos.y) {
            let range = if select_word {
                let max_char = l.actual_text_caret_range().end;
                let mut r = l.nearest_seg(pos.x).map(|seg| seg.text_range()).unwrap_or_else(|| l.text_range());
                // don't select line-break at end of line
                r.start = r.start.min(max_char);
                r.end = r.end.min(max_char);
                r
            } else {
                l.actual_text_caret_range()
            };

            let merge_with_selection = if replace_selection {
                None
            } else {
                caret.initial_selection.clone().map(|(s, _)| s).or_else(|| caret.selection_range())
            };
            if let Some(mut s) = merge_with_selection {
                let caret_at_start = range.start < s.start.index;
                s.start.index = s.start.index.min(range.start);
                s.end.index = s.end.index.max(range.end);

                if caret_at_start {
                    caret.selection_index = Some(s.end);
                    caret.set_index(s.start);
                } else {
                    caret.selection_index = Some(s.start);
                    caret.set_index(s.end);
                }
            } else {
                let start = CaretIndex {
                    line: l.index(),
                    index: range.start,
                };
                let end = CaretIndex {
                    line: l.index(),
                    index: range.end,
                };
                caret.selection_index = Some(start);
                caret.set_index(end);

                caret.initial_selection = Some((start..end, select_word));
            }

            return;
        };
    }

    if caret.index.is_none() {
        caret.set_index(CaretIndex::ZERO);
        caret.clear_selection();
    }
}
