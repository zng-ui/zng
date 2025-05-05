//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`node::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{any::Any, borrow::Cow, cmp, fmt, ops, sync::Arc};

use parking_lot::Mutex;
use zng_ext_font::*;
use zng_ext_input::focus::{FOCUS, WidgetFocusInfo};
use zng_ext_l10n::l10n;
use zng_ext_undo::*;
use zng_wgt::prelude::*;

use crate::node::{RichText, RichTextWidgetInfoExt};

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
                        TEXT.resolved()
                            .txt
                            .modify(clmv!(insert, |args| {
                                args.to_mut().to_mut().insert_str(i, insert.as_str());
                            }))
                            .unwrap();

                        let mut i = insert_idx;
                        i.index += insert.len();

                        let mut caret = TEXT.resolve_caret();
                        caret.set_index(i);
                        caret.clear_selection();
                    }
                    SelectionState::CaretSelection(start, end) | SelectionState::SelectionCaret(start, end) => {
                        let char_range = start.index..end.index;
                        TEXT.resolved()
                            .txt
                            .modify(clmv!(insert, |args| {
                                args.to_mut().to_mut().replace_range(char_range, insert.as_str());
                            }))
                            .unwrap();

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

                TEXT.resolved()
                    .txt
                    .modify(clmv!(removed, |args| {
                        args.to_mut().to_mut().replace_range(i..i + len, removed.as_str());
                    }))
                    .unwrap();

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
                if within_undo_interval {
                    if let Some(next_data) = next_data.downcast_mut::<InsertData>() {
                        if let (SelectionState::Caret(mut after_idx), SelectionState::Caret(caret)) =
                            (data.selection_state, next_data.selection_state)
                        {
                            after_idx.index += data.insert.len();

                            if after_idx.index == caret.index {
                                data.insert.push_str(&next_data.insert);
                                *merged = true;
                            }
                        }
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

                ctx.txt
                    .modify(move |args| {
                        args.to_mut().to_mut().replace_range(rmv, "");
                    })
                    .unwrap();
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

                TEXT.resolved()
                    .txt
                    .modify(clmv!(removed, |args| {
                        args.to_mut().to_mut().insert_str(insert_idx, removed.as_str());
                    }))
                    .unwrap();

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
                if within_undo_interval {
                    if let Some(next_data) = next_data.downcast_mut::<BackspaceData>() {
                        if let (SelectionState::Caret(mut after_idx), SelectionState::Caret(caret)) =
                            (data.selection_state, next_data.selection_state)
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
                ctx.txt
                    .modify(move |args| {
                        args.to_mut().to_mut().replace_range(rmv, "");
                    })
                    .unwrap();
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

                TEXT.resolved()
                    .txt
                    .modify(clmv!(removed, |args| {
                        args.to_mut().to_mut().insert_str(insert_idx, removed.as_str());
                    }))
                    .unwrap();

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
                if within_undo_interval {
                    if let Some(next_data) = next_data.downcast_ref::<DeleteData>() {
                        if let (SelectionState::Caret(after_idx), SelectionState::Caret(caret)) =
                            (data.selection_state, next_data.selection_state)
                        {
                            if after_idx.index == caret.index {
                                data.count += next_data.count;
                                data.removed.push_str(&next_data.removed);
                                *merged = true;
                            }
                        }
                    }
                }
            }
        })
    }

    fn apply_max_count(redo: &mut bool, txt: &BoxedVar<Txt>, rmv_range: ops::Range<usize>, insert: &mut Txt) {
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
                let _ = TEXT.resolved().txt.set("");
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let _ = TEXT.resolved().txt.set(data.txt.clone());

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
                TEXT.resolved()
                    .txt
                    .modify(clmv!(select_before, insert, |args| {
                        args.to_mut().to_mut().replace_range(select_before, insert.as_str());
                    }))
                    .unwrap();

                TEXT.resolve_caret().set_char_selection(select_after.start, select_after.end);
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let ctx = TEXT.resolved();

                select_after.start = ctx.segmented_text.snap_grapheme_boundary(select_after.start);
                select_after.end = ctx.segmented_text.snap_grapheme_boundary(select_after.end);

                ctx.txt
                    .modify(clmv!(select_after, removed, |args| {
                        args.to_mut().to_mut().replace_range(select_after, removed.as_str());
                    }))
                    .unwrap();

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
                    let _ = ctx.txt.set(t);
                }
            }
            UndoFullOp::Op(UndoOp::Undo) => {
                let ctx = TEXT.resolved();

                if ctx.txt.with(|t| t != prev.as_str()) {
                    let _ = ctx.txt.set(prev.clone());
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
        Self::new(|| rich_line_up_down(true, -1))
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the previous line.
    ///
    /// This is the `SHIFT+Up` key operation.
    pub fn select_line_up() -> Self {
        Self::new(|| rich_line_up_down(false, -1))
    }

    /// Clear selection and move the caret to the nearest insert index on the next line.
    ///
    /// This is the `Down` key operation.
    pub fn line_down() -> Self {
        Self::new(|| rich_line_up_down(true, 1))
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the next line.
    ///
    /// This is the `SHIFT+Down` key operation.
    pub fn select_line_down() -> Self {
        Self::new(|| rich_line_up_down(false, 1))
    }

    /// Clear selection and move the caret one viewport up.
    ///
    /// This is the `PageUp` key operation.
    pub fn page_up() -> Self {
        Self::new(|| rich_page_up_down(true, -1))
    }

    /// Extend or shrink selection by moving the caret one viewport up.
    ///
    /// This is the `SHIFT+PageUp` key operation.
    pub fn select_page_up() -> Self {
        Self::new(|| rich_page_up_down(false, -1))
    }

    /// Clear selection and move the caret one viewport down.
    ///
    /// This is the `PageDown` key operation.
    pub fn page_down() -> Self {
        Self::new(|| rich_page_up_down(true, 1))
    }

    /// Extend or shrink selection by moving the caret one viewport down.
    ///
    /// This is the `SHIFT+PageDown` key operation.
    pub fn select_page_down() -> Self {
        Self::new(|| rich_page_up_down(false, 1))
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
        Self::new(|| rich_clear_text_start_end(false))
    }

    /// Extend or shrink selection by moving the caret to the text start.
    ///
    /// This is the `CTRL+SHIFT+Home` shortcut operation.
    pub fn select_text_start() -> Self {
        Self::new(|| rich_select_text_start_end(false))
    }

    /// Clear selection and move the caret to the text end.
    ///
    /// This is the `CTRL+End` shortcut operation.
    pub fn text_end() -> Self {
        Self::new(|| rich_clear_text_start_end(true))
    }

    /// Extend or shrink selection by moving the caret to the text end.
    ///
    /// This is the `CTRL+SHIFT+End` shortcut operation.
    pub fn select_text_end() -> Self {
        Self::new(|| rich_select_text_start_end(true))
    }

    /// Clear selection and move the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down operation.
    pub fn nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            rich_nearest_to(true, window_point);
        })
    }

    /// Extend or shrink selection by moving the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down when holding SHIFT operation.
    pub fn select_nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            rich_nearest_to(false, window_point);
        })
    }

    /// Extend or shrink selection by moving the caret index or caret selection index to the insert point nearest to `window_point`.
    ///
    /// This is the touch selection caret drag operation.
    pub fn select_index_nearest_to(window_point: DipPoint, move_selection_index: bool) -> Self {
        Self::new(move || {
            rich_index_nearest_to(window_point, move_selection_index);
        })
    }

    /// Replace or extend selection with the word nearest to the `window_point`
    ///
    /// This is the mouse primary button double click.
    pub fn select_word_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || rich_select_line_word_nearest_to(replace_selection, true, window_point))
    }

    /// Replace or extend selection with the line nearest to the `window_point`
    ///
    /// This is the mouse primary button triple click.
    pub fn select_line_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || rich_select_line_word_nearest_to(replace_selection, false, window_point))
    }

    /// Select the full text.
    pub fn select_all() -> Self {
        Self::new(|| {
            if let Some(ctx) = TEXT.try_rich() {
                if let Some(info) = ctx.root_info() {
                    let mut first_id = None;
                    let mut last_id = None;
                    for leaf in info.rich_text_leaves() {
                        let leaf_id = leaf.info().id();
                        SELECT_CMD.scoped(leaf_id).notify_param(Self::local_select_all());

                        if first_id.is_none() {
                            first_id = Some(leaf_id);
                        }
                        last_id = Some(leaf_id);
                    }
                    let root_id = ctx.root_id;
                    drop(ctx);
                    let mut ctx = TEXT.resolve_rich_caret();
                    ctx.selection_index = first_id;
                    ctx.index = last_id;

                    if let Some(last_id) = last_id {
                        let current_id = WIDGET.id();
                        if last_id != current_id && FOCUS.is_focus_within(root_id).get() {
                            FOCUS.focus_widget(last_id, false);
                        }
                        return;
                    }
                }
            }
            Self::local_select_all().call();
        })
    }

    /// Clear selection and keep the caret at the same position.
    ///
    /// This is the `Esc` shortcut operation.
    pub fn clear_selection() -> Self {
        Self::new(|| {
            let op = Self::local_clear_selection();
            if let Some(ctx) = TEXT.try_rich() {
                for leaf in ctx.selection() {
                    SELECT_CMD.scoped(leaf.info().id()).notify_param(op.clone());
                }
                drop(ctx);
                let mut ctx = TEXT.resolve_rich_caret();
                ctx.selection_index = None;
            } else {
                op.call();
            }
        })
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
        Self::new(|| local_clear_text_start_end(false))
    }

    /// Like [`select_text_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_text_start`]: Self::select_text_start
    pub fn local_select_text_start() -> Self {
        Self::new(|| local_select_text_start_end(false))
    }

    /// Like [`text_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`text_end`]: Self::text_end
    pub fn local_text_end() -> Self {
        Self::new(|| local_clear_text_start_end(true))
    }

    /// Like [`select_text_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_text_end`]: Self::select_text_end
    pub fn local_select_text_end() -> Self {
        Self::new(|| local_select_text_start_end(true))
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
        Self::new(|| {
            local_line_up_down(true, -1);
        })
    }

    /// Like [`select_line_up`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_up`]: Self::select_line_up
    pub fn local_select_line_up() -> Self {
        Self::new(|| {
            local_line_up_down(false, -1);
        })
    }

    /// Like [`line_down`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`line_down`]: Self::line_down
    pub fn local_line_down() -> Self {
        Self::new(|| {
            local_line_up_down(true, 1);
        })
    }

    /// Like [`select_line_down`]  but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_line_down`]: Self::select_line_down
    pub fn local_select_line_down() -> Self {
        Self::new(|| {
            local_line_up_down(false, 1);
        })
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
            local_index_nearest_to(window_point, move_selection_index);
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
    /// If the op is not called inside a rich text only `local_caret_index` and `local_selection_index` are called.
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
                rich_select_op(ctx, f0.take().unwrap(), f1.take().unwrap(), f2.take().unwrap(), f3.take().unwrap());
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

fn rich_select_op<D0: Send + 'static, D1: Send + 'static, D2: Send + 'static>(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: impl FnOnce(&RichText) -> (WidgetId, D0),
    local_caret_index: impl FnOnce(D0) -> (CaretIndex, D1) + Send + 'static,
    rich_selection_index: impl FnOnce(&RichText, D1) -> Option<(WidgetId, D2)> + Send + 'static,
    local_selection_index: impl FnOnce(D2) -> Option<CaretIndex> + Send + 'static,
) {
    let (index, d0) = rich_caret_index(&ctx);
    if index == WIDGET.id() {
        rich_select_op_0_get_caret(ctx, index, d0, local_caret_index, rich_selection_index, local_selection_index);
    } else {
        let mut d0 = Some(d0);
        let mut f0 = Some(local_caret_index);
        let mut f1 = Some(rich_selection_index);
        let mut f2 = Some(local_selection_index);
        SELECT_CMD.scoped(index).notify_param(TextSelectOp::new(move || {
            if let Some(ctx) = TEXT.try_rich() {
                if index == WIDGET.id() {
                    rich_select_op_0_get_caret(
                        ctx,
                        index,
                        d0.take().unwrap(),
                        f0.take().unwrap(),
                        f1.take().unwrap(),
                        f2.take().unwrap(),
                    );
                }
            }
        }));
    }
}
fn rich_select_op_0_get_caret<D0, D1, D2: Send + 'static>(
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
                rich_select_op_1_get_selection(ctx, rich_caret_index, selection_index, d2, local_selection_index);
            } else {
                let mut d2 = Some(d2);
                let mut f0 = Some(local_selection_index);
                SELECT_CMD.scoped(selection_index).notify_param(TextSelectOp::new(move || {
                    if let Some(ctx) = TEXT.try_rich() {
                        if selection_index == WIDGET.id() {
                            rich_select_op_1_get_selection(ctx, rich_caret_index, selection_index, d2.take().unwrap(), f0.take().unwrap());
                        }
                    }
                }));
            }
        }
        None => rich_select_op_2_finish(ctx, rich_caret_index, None),
    }
}
fn rich_select_op_1_get_selection<D2>(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: WidgetId,
    rich_selection_index: WidgetId,
    d2: D2,
    local_selection_index: impl FnOnce(D2) -> Option<CaretIndex>,
) {
    if let Some(index) = local_selection_index(d2) {
        let mut local_ctx = TEXT.resolve_caret();
        local_ctx.selection_index = Some(index);
        local_ctx.index_version += 1;
        rich_select_op_2_finish(ctx, rich_caret_index, Some(rich_selection_index));
    } else {
        rich_select_op_2_finish(ctx, rich_caret_index, None);
    }
}
fn rich_select_op_2_finish(
    ctx: zng_app_context::RwLockReadGuardOwned<RichText>,
    rich_caret_index: WidgetId,
    rich_selection_index: Option<WidgetId>,
) {
    if let Some(index) = ctx.leaf_info(rich_caret_index) {
        let selection_index = rich_selection_index.and_then(|id| ctx.leaf_info(id));
        if selection_index.is_some() == rich_selection_index.is_some() {
            drop(ctx);
            TEXT.resolve_rich_caret()
                .update_selection(index.info(), selection_index.as_ref().map(|s| s.info()), false, false);
        }
    }
}

fn rich_clear_next_prev(is_next: bool, is_word: bool) -> TextSelectOp {
    TextSelectOp::new_rich(
        // get prev/next leaf widget
        move |ctx| {
            let local_ctx = TEXT.resolved();
            if is_next {
                if local_ctx.caret.selection_index.is_some() {
                    // just clears selection, caret does not move outside selection
                    return (WIDGET.id(), false);
                }

                let index = local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index;
                if index == local_ctx.segmented_text.text().len() {
                    // next from end, check if has next sibling
                    if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                        if let Some(next) = info.info().rich_text_next().next() {
                            return (next.info().id(), true);
                        }
                    }
                }

                // caret stays inside
                (WIDGET.id(), false)
            } else {
                // !is_next

                let mut prev_wgt = false;
                if let Some(sel_i) = local_ctx.caret.selection_index {
                    let i = local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index.min(sel_i.index);
                    if i == 0 {
                        // next clears to start, start is the prev end when there is a prev
                        prev_wgt = true;
                    }
                } else {
                    let cutout = if is_word { local_ctx.segmented_text.next_word_index(0) } else { 1 };
                    if local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index <= cutout {
                        // next moves to the start (or is already in start)
                        prev_wgt = true;
                    }
                }

                if prev_wgt {
                    if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                        if let Some(prev) = info.info().rich_text_prev().next() {
                            return (prev.info().id(), true);
                        }
                    }
                }

                (WIDGET.id(), false)
            }
        },
        // get caret in the prev/next widget
        move |is_from_sibling| {
            if is_from_sibling {
                if is_next {
                    // caret was at sibling end without selection
                    (CaretIndex { index: 1, line: 0 }, ())
                } else {
                    // caret was at sibling start with selection or moves to sibling start (that is the same as our end)
                    let len = TEXT.resolved().segmented_text.text().len();
                    (CaretIndex { index: len, line: 0 }, ())
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
                        if let Some(next) = info.info().rich_text_next().next() {
                            return (next.info().id(), true);
                        }
                    }
                }
            } else {
                // !is_next

                let cutout = if is_word { local_ctx.segmented_text.next_word_index(0) } else { 1 };
                if local_ctx.caret.index.unwrap_or(CaretIndex::ZERO).index <= cutout {
                    // next moves to the start (or is already in start)
                    if let Some(info) = ctx.leaf_info(WIDGET.id()) {
                        if let Some(prev) = info.info().rich_text_prev().next() {
                            return (prev.info().id(), true);
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

fn rich_line_up_down(clear_selection: bool, diff: i8) {
    if let Some(ctx) = TEXT.try_rich() {
        let resolved = TEXT.resolved();
        let laidout = TEXT.laidout();

        let local_line_i = resolved.caret.index.unwrap_or(CaretIndex::ZERO).line;
        let last_line_i = laidout.shaped_text.lines_len().saturating_sub(1);
        let next_local_line_i = local_line_i.saturating_add_signed(diff as isize).min(last_line_i);

        let mut need_rich_query = local_line_i == next_local_line_i; // if already at first/last line

        if !need_rich_query {
            if let Some(next_local_line) = laidout.shaped_text.line(next_local_line_i) {
                let r = next_local_line.rect();
                let x = laidout.caret_retained_x;
                need_rich_query = r.min_x() > x || r.max_x() < x; // if next local line does not contain ideal caret horizontally
            }
        }

        if need_rich_query {
            if let Some(local_line) = laidout.shaped_text.line(local_line_i) {
                // line ok
                let r = local_line.rect();
                let local_point = PxPoint::new(laidout.caret_retained_x, r.origin.y + r.size.height / Px(2));
                let local_info = WIDGET.info();
                let local_to_window = local_info.inner_transform();
                if let Some(mut window_point) = local_to_window.transform_point(local_point) {
                    // transform ok

                    if let Some(root_info) = ctx.root_info() {
                        // rich context ok

                        // find the nearest sibling considering only the prev/next rich lines
                        let local_line_info = local_info.rich_text_line_info();
                        let filter = |other: &WidgetFocusInfo, _, row_i, rows_len| {
                            match local_info.cmp_sibling_in(other.info(), &root_info).unwrap() {
                                cmp::Ordering::Less => {
                                    // other is after local
                                    if diff < 0 {
                                        return false; // navigating up
                                    }

                                    if local_line_i < last_line_i {
                                        return true; // local started next line
                                    }

                                    for next in local_info.rich_text_next() {
                                        let line_info = next.info().rich_text_line_info();
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
                                    if diff > 0 {
                                        return false; // navigation down
                                    }

                                    if local_line_i > 0 || local_line_info.starts_new_line {
                                        return true; // local started line, all prev wgt in prev lines
                                    }

                                    for prev in local_info.rich_text_prev() {
                                        let line_info = prev.info().rich_text_line_info();
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
                            // found sibling
                            let next_info = next.info().clone();
                            let next_to_window = next_info.inner_transform();
                            let window_to_next = next_to_window.inverse().unwrap_or_default();

                            if let Some(next_inline_rows_len) = next_info.bounds_info().inline().map(|i| i.rows.len()) {
                                // local_nearest_to uses "nearest_line(y)", need to adjust the y to match the next rich line

                                let mut next_line = 0;
                                if next_inline_rows_len > 1 {
                                    if diff > 0 {
                                        // next is down (logical next)

                                        if local_line_i == last_line_i {
                                            // local did not start next line

                                            for l_next in local_info.rich_text_next() {
                                                let line_info = l_next.info().rich_text_line_info();
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
                                                let line_info = l_prev.info().rich_text_line_info();
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

                                let next_line = next_info.bounds_info().inline().unwrap().rows[next_line];
                                let next_line_y = next_line.origin.y + next_line.size.height / Px(2);

                                window_point.y = next_to_window.transform_point(PxPoint::new(Px(0), next_line_y)).unwrap().y;
                            }
                            let window_x = local_to_window
                                .transform_point(PxPoint::new(laidout.caret_retained_x, Px(0)))
                                .unwrap_or_default();
                            let next_x = window_to_next.transform_point(window_x).unwrap_or_default().x;

                            // send request
                            let window_point = window_point.to_dip(root_info.tree().scale_factor());
                            let root_id = root_info.id();
                            let next_id = next_info.id();
                            let local_id = local_info.id();
                            SELECT_CMD.scoped(next_id).notify_param(TextSelectOp::new(move || {
                                let ctx = match TEXT.try_rich() {
                                    Some(c) => c,
                                    None => return,
                                };

                                TEXT.set_caret_retained_x(next_x);

                                local_nearest_to(clear_selection, window_point);

                                TEXT.resolve_caret().used_retained_x = true;

                                // !!: TODO prev selection min/max, new selection min/max, only send the needed messages
                                for sel in ctx.selection() {
                                    let sel_id = sel.info().id();
                                    if sel_id == next_id {
                                        continue;
                                    }
                                    SELECT_CMD.scoped(sel_id).notify_param(TextSelectOp::clear_selection());
                                }

                                let mut ctx = TEXT.resolve_rich_caret();
                                if clear_selection {
                                    ctx.selection_index = None;
                                } else {
                                    if ctx.selection_index.is_none() {
                                        ctx.selection_index = Some(local_id);
                                    }

                                    if let Some(cmp) = local_info.cmp_sibling_in(&next_info, &root_info) {
                                        match cmp {
                                            cmp::Ordering::Less => todo!(),
                                            cmp::Ordering::Equal => todo!(),
                                            cmp::Ordering::Greater => todo!(),
                                        }
                                    }
                                }
                                ctx.index = Some(next_id);

                                if FOCUS.is_focus_within(root_id).get() {
                                    FOCUS.focus_widget(next_id, false);
                                }
                            }));
                            return;
                        }
                    }
                }
            }
        }
    }

    if local_line_up_down(clear_selection, diff) {
        if clear_selection {
            rich_clear_text_start_end(diff > 0);
        } else {
            rich_select_text_start_end(diff > 0);
        }
    }
}
/// Returns `true` if caret was moved to start/end because cannot go up/down.
fn local_line_up_down(clear_selection: bool, diff: i8) -> bool {
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
    let mut caret_to_start_end = false;

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
            caret_to_start_end = true;
        } else if diff == 1 {
            drop(caret);
            let len = TEXT.resolved().segmented_text.text().len();
            caret = TEXT.resolve_caret();
            caret.set_char_index(len);
            caret_to_start_end = true;
        }
    }

    if caret.index.is_none() {
        caret.set_index(CaretIndex::ZERO);
        caret.clear_selection();
    }

    caret_to_start_end
}

fn rich_page_up_down(clear_selection: bool, diff: i8) {
    if let Some(_ctx) = TEXT.try_rich() {
        // !!: TODO
    } else {
        local_page_up_down(clear_selection, diff);
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
            if let Some(c) = ctx.leaf_info(WIDGET.id()) {
                let local_line = TEXT.resolved().caret.index.unwrap_or(CaretIndex::ZERO).line;
                if is_end {
                    let last_line = TEXT.laidout().shaped_text.lines_len() - 1;
                    if local_line == last_line {
                        // current line can end in a next sibling

                        let mut prev_id = c.info().id();
                        for c in c.info().rich_text_next() {
                            let line_info = c.info().rich_text_line_info();
                            if line_info.starts_new_line {
                                return (prev_id, prev_id != c.info().id());
                            } else if line_info.ends_in_new_line {
                                return (c.info().id(), true);
                            }
                            prev_id = c.info().id();
                        }

                        // text end
                        return (prev_id, prev_id != c.info().id());
                    }
                } else {
                    // !is_end

                    if local_line == 0 {
                        // current line can start in a prev sibling

                        let mut last_id = c.info().id();
                        for c in c.info().rich_text_prev() {
                            let line_info = c.info().rich_text_line_info();
                            if line_info.starts_new_line || line_info.ends_in_new_line {
                                return (c.info().id(), true);
                            }
                            last_id = c.info().id();
                        }

                        // text start
                        return (last_id, last_id != c.info().id());
                    }
                }
            }
            (WIDGET.id(), false)
        },
        // get local caret index in the rich line start/end widget
        move |is_from_sibling| {
            if is_from_sibling {
                // ensure the caret is at a start/end from the other sibling for `local_clear_line_start_end`
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
            local_line_start_end(clear_selection, is_end);

            (TEXT.resolved().caret.index.unwrap(), WIDGET.id())
        },
        // get the selection index widget, line selection always updates from the caret
        move |ctx, new_index| {
            if clear_selection {
                return None;
            }
            Some((ctx.caret.selection_index.unwrap_or(new_index), ()))
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
        i.index = if is_end { li.text_caret_range().end } else { li.text_range().start };
        ctx.set_index(i);
        ctx.used_retained_x = false;
    }
}

fn rich_clear_text_start_end(is_end: bool) {
    if let Some(ctx) = TEXT.try_rich() {
        let advance = if is_end { ctx.leaves_rev().next() } else { ctx.leaves().next() };

        if let Some(advance) = advance {
            let root_id = ctx.root_id;
            drop(ctx);
            let mut ctx = TEXT.resolve_rich_caret();
            ctx.selection_index = None;

            let advance_id = advance.info().id();
            SELECT_CMD.scoped(advance_id).notify_param(if is_end {
                TextSelectOp::local_text_end()
            } else {
                TextSelectOp::local_text_start()
            });
            ctx.index = Some(advance_id);
            if FOCUS.is_focus_within(root_id).get() {
                FOCUS.focus_widget(advance_id, false);
            }
        }
    } else {
        local_clear_text_start_end(is_end);
    }
}
fn local_clear_text_start_end(is_end: bool) {
    let idx = if is_end { TEXT.resolved().segmented_text.text().len() } else { 0 };

    let mut ctx = TEXT.resolve_caret();
    let mut i = ctx.index.unwrap_or(CaretIndex::ZERO);
    ctx.clear_selection();
    i.index = idx;
    ctx.set_index(i);
    ctx.used_retained_x = false;
}

fn rich_select_text_start_end(is_end: bool) {
    if let Some(ctx) = TEXT.try_rich() {
        // clear current selection
        let clear_op = if is_end {
            TextSelectOp::local_next()
        } else {
            TextSelectOp::local_prev()
        };
        for leaf in ctx.selection() {
            SELECT_CMD.scoped(leaf.info().id()).notify_param(clear_op.clone());
        }

        let current_index = ctx
            .caret
            .index
            .or_else(|| ctx.leaf_info(WIDGET.id()).map(|w| w.info().id()))
            .or_else(|| ctx.leaves().next().map(|w| w.info().id()));
        let current_index = match current_index {
            Some(id) => id,
            None => return,
        };
        let selection_index = ctx.caret.selection_index.unwrap_or(current_index);

        let mut new_index = None;
        if is_end {
            let op = TextSelectOp::local_select_text_end();
            for leaf in ctx.leaves_rev() {
                let leaf_id = leaf.info().id();
                if new_index.is_none() {
                    new_index = Some(leaf_id);
                }
                SELECT_CMD.scoped(leaf_id).notify_param(op.clone());
                if leaf_id == selection_index {
                    break;
                }
            }
        } else {
            let op = TextSelectOp::local_select_text_start();
            for leaf in ctx.leaves() {
                let leaf_id = leaf.info().id();
                if new_index.is_none() {
                    new_index = Some(leaf_id);
                }
                SELECT_CMD.scoped(leaf_id).notify_param(op.clone());
                if leaf_id == selection_index {
                    break;
                }
            }
        }

        if let Some(new_index) = new_index {
            let root_id = ctx.root_id;
            drop(ctx);
            let mut ctx = TEXT.resolve_rich_caret();
            ctx.selection_index = Some(selection_index);
            ctx.index = Some(new_index);

            if FOCUS.is_focus_within(root_id).get() {
                FOCUS.focus_widget(new_index, false);
            }
        }
    } else {
        local_select_text_start_end(is_end);
    }
}
fn local_select_text_start_end(is_end: bool) {
    let idx = if is_end { TEXT.resolved().segmented_text.text().len() } else { 0 };

    let mut ctx = TEXT.resolve_caret();
    let mut i = ctx.index.unwrap_or(CaretIndex::ZERO);
    if ctx.selection_index.is_none() {
        ctx.selection_index = Some(i);
    }
    i.index = idx;
    ctx.set_index(i);
    ctx.used_retained_x = false;
}

fn rich_nearest_to(clear_selection: bool, window_point: DipPoint) {
    if let Some(ctx) = TEXT.try_rich() {
        let root = match ctx.root_info() {
            Some(r) => r,
            None => return,
        };

        if let Some(nearest_leaf) = root.rich_text_nearest_leaf(window_point.to_px(root.tree().scale_factor())) {
            let id = nearest_leaf.info().id();
            if id != WIDGET.id() {
                SELECT_CMD
                    .scoped(id)
                    .notify_param(TextSelectOp::new(move || continue_rich_nearest_to(clear_selection, window_point)));
                return;
            }
        }

        drop(ctx);
        continue_rich_nearest_to(clear_selection, window_point);
    } else {
        local_nearest_to(clear_selection, window_point);
    }
}
fn continue_rich_nearest_to(clear_selection: bool, window_point: DipPoint) {
    if let Some(ctx) = TEXT.try_rich() {
        let id = WIDGET.id();

        if clear_selection {
            let op = TextSelectOp::local_clear_selection();
            let id = WIDGET.id();
            for leaf in ctx.selection() {
                let leaf_id = leaf.info().id();
                if leaf_id != id {
                    SELECT_CMD.scoped(leaf_id).notify_param(op.clone());
                }
            }
        } else if let Some(sid) = ctx.caret.selection_index {
            if sid != id {
                let mut local_ctx = TEXT.resolve_caret();
                if local_ctx.selection_index.is_none() {
                    let tree = WINDOW.info();
                    let r_info = tree.get(ctx.root_id);
                    let s_info = tree.get(sid);
                    let info = tree.get(id);

                    if let (Some(s_info), Some(info), Some(r_info)) = (s_info, info, r_info) {
                        if let Some(ordering) = info.cmp_sibling_in(&s_info, &r_info) {
                            // snap index to start/end in the direction of the rich selection index so that
                            // local_nearest_to can start the local selection from the right end.
                            match ordering {
                                cmp::Ordering::Less => {
                                    drop(local_ctx);
                                    let len = TEXT.resolved().segmented_text.text().len();
                                    let mut local_ctx = TEXT.resolve_caret();
                                    local_ctx.selection_index = Some(CaretIndex { index: len, line: 0 });
                                }
                                cmp::Ordering::Greater => local_ctx.selection_index = Some(CaretIndex::ZERO),
                                cmp::Ordering::Equal => {}
                            }

                            // fast cursor move can skip the start/end of local selection of previous widget, ensure they are finished here.
                            match ordering {
                                cmp::Ordering::Less => {
                                    let op = TextSelectOp::new(|| {
                                        TEXT.resolve_caret().set_char_index(0);
                                    });
                                    for next in info.rich_text_next() {
                                        let next_id = next.info().id();
                                        SELECT_CMD.scoped(next_id).notify_param(op.clone());
                                        if next_id == sid {
                                            break;
                                        }
                                    }
                                }
                                cmp::Ordering::Greater => {
                                    let op = TextSelectOp::new(|| {
                                        let len = TEXT.resolved().segmented_text.text().len();
                                        TEXT.resolve_caret().set_char_index(len);
                                    });
                                    for prev in info.rich_text_prev() {
                                        let prev_id = prev.info().id();
                                        SELECT_CMD.scoped(prev_id).notify_param(op.clone());
                                        if prev_id == sid {
                                            break;
                                        }
                                    }
                                }
                                cmp::Ordering::Equal => {}
                            }
                        }
                    }
                }
            }
        }
        local_nearest_to(clear_selection, window_point);

        let root_id = ctx.root_id;
        drop(ctx);
        let mut ctx = TEXT.resolve_rich_caret();
        if clear_selection {
            ctx.selection_index = None;
        } else if ctx.selection_index.is_none() {
            ctx.selection_index = Some(ctx.index.unwrap_or(id));
        }
        let prev_index = ctx.index;
        ctx.index = Some(id);
        if let (Some(prev_id), Some(sel_id)) = (prev_index, ctx.selection_index) {
            drop(ctx);
            if prev_id != id {
                // fast cursor move can leave some selection behind, ensure clear.
                let ctx = TEXT.rich();
                let tree = WINDOW.info();
                if let (Some(prev), Some(new), Some(selection), Some(root)) =
                    (tree.get(prev_id), tree.get(id), tree.get(sel_id), tree.get(ctx.root_id))
                {
                    if let (Some(ordering), Some(sel_ordering)) = (new.cmp_sibling_in(&prev, &root), prev.cmp_sibling_in(&selection, &root))
                    {
                        // all valid
                        if sel_ordering != ordering && sel_ordering != cmp::Ordering::Equal {
                            // reduced selection/changed towards selection
                            let op = TextSelectOp::local_clear_selection();
                            match ordering {
                                cmp::Ordering::Less => {
                                    for next in new.rich_text_next() {
                                        let next_id = next.info().id();
                                        SELECT_CMD.scoped(next_id).notify_param(op.clone());
                                        if next_id == prev_id {
                                            break;
                                        }
                                    }
                                }
                                cmp::Ordering::Greater => {
                                    for prev in new.rich_text_prev() {
                                        let prev_info_id = prev.info().id();
                                        SELECT_CMD.scoped(prev_info_id).notify_param(op.clone());
                                        if prev_info_id == prev_id {
                                            break;
                                        }
                                    }
                                }
                                cmp::Ordering::Equal => {}
                            }
                        }
                    }
                }
            }
        }
        if FOCUS.is_focus_within(root_id).get() {
            FOCUS.focus_widget(id, false);
        }
    }
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

fn rich_index_nearest_to(window_point: DipPoint, move_selection_index: bool) {
    if let Some(_ctx) = TEXT.try_rich() {
        // !!: TODO
    } else {
        local_index_nearest_to(window_point, move_selection_index);
    }
}
fn local_index_nearest_to(window_point: DipPoint, move_selection_index: bool) {
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

fn rich_select_line_word_nearest_to(replace_selection: bool, select_word: bool, window_point: DipPoint) {
    if let Some(_ctx) = TEXT.try_rich() {
        // !!: TODO
    } else {
        local_select_line_word_nearest_to(replace_selection, select_word, window_point);
    }
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
