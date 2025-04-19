//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`node::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{any::Any, borrow::Cow, fmt, ops, sync::Arc};

use parking_lot::Mutex;
use zng_ext_font::*;
use zng_ext_input::focus::{FOCUS, WidgetFocusInfo};
use zng_ext_l10n::l10n;
use zng_ext_undo::*;
use zng_wgt::prelude::*;

use crate::node::{RichTextComponent, RichTextWidgetInfoExt};

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
    /// New text select operation.
    ///
    /// The editable text widget that handles [`SELECT_CMD`] will call `op` during event handling in
    /// the [`node::layout_text`] context. You can position the caret using [`TEXT.resolve_caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    ///
    /// [`TEXT.resolve_caret`]: super::node::TEXT::resolve_caret
    pub fn new(op: impl FnMut() + Send + 'static) -> Self {
        Self {
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// Clear selection and move the caret to the next insert index.
    ///
    /// This is the `Right` key operation.
    pub fn next() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::next_insert_index,
                |_, s| s.end.index,
                Some(|w| w.rich_text_next().next()),
                || {
                    Self::new(|| {
                        Self::local_text_start().call();
                        Self::local_next().call();
                    })
                },
            )
        })
    }
    /// Like [`next`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`next`]: Self::next
    pub fn local_next() -> Self {
        Self::new(|| next_prev(true, SegmentedText::next_insert_index, |_, s| s.end.index, None, || unreachable!()))
    }

    /// Extend or shrink selection by moving the caret to the next insert index.
    ///
    /// This is the `SHIFT+Right` key operation.
    pub fn select_next() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::next_insert_index,
                |_, _| unreachable!(),
                Some(|w| w.rich_text_next().next()),
                Self::local_text_start, // !!: TODO
            )
        })
    }
    /// Like [`select_next`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_next`]: Self::select_next
    pub fn local_select_next() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::next_insert_index,
                |_, _| unreachable!(),
                None,
                || unreachable!(),
            )
        })
    }

    /// Clear selection and move the caret to the previous insert index.
    ///
    /// This is the `Left` key operation.
    pub fn prev() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::prev_insert_index,
                |_, s| s.start.index,
                Some(|w| w.rich_text_prev().next()),
                Self::local_text_end,
            )
        })
    }
    /// Like [`prev`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`prev`]: Self::prev
    pub fn local_prev() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::prev_insert_index,
                |_, s| s.start.index,
                None,
                || unreachable!(),
            )
        })
    }

    /// Extend or shrink selection by moving the caret to the previous insert index.
    ///
    /// This is the `SHIFT+Left` key operation.
    pub fn select_prev() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::prev_insert_index,
                |_, _| unreachable!(),
                Some(|w| w.rich_text_prev().next()),
                Self::local_text_end, // !!: TODO
            )
        })
    }
    /// Like [`select_prev`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_prev`]: Self::select_prev
    pub fn local_select_prev() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::prev_insert_index,
                |_, _| unreachable!(),
                None,
                || unreachable!(),
            )
        })
    }

    /// Clear selection and move the caret to the next word insert index.
    ///
    /// This is the `CTRL+Right` shortcut operation.
    pub fn next_word() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::next_word_index,
                |t, s| t.next_word_index(s.end.index),
                Some(|w| w.rich_text_next().next()),
                || {
                    Self::new(|| {
                        Self::local_text_start().call();
                        Self::local_next_word().call();
                    })
                },
            )
        })
    }
    /// Like [`next_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`next_word`]: Self::next_word
    pub fn local_next_word() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::next_word_index,
                |t, s| t.next_word_index(s.end.index),
                None,
                || unreachable!(),
            )
        })
    }

    /// Extend or shrink selection by moving the caret to the next word insert index.
    ///
    /// This is the `CTRL+SHIFT+Right` shortcut operation.
    pub fn select_next_word() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::next_word_index,
                |_, _| unreachable!(),
                Some(|w| w.rich_text_next().next()),
                Self::local_text_start, // !!: TODO
            )
        })
    }
    /// Like [`select_next_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_next_word`]: Self::select_next_word
    pub fn local_select_next_word() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::next_word_index,
                |_, _| unreachable!(),
                None,
                || unreachable!(),
            )
        })
    }

    /// Clear selection and move the caret to the previous word insert index.
    ///
    /// This is the `CTRL+Left` shortcut operation.
    pub fn prev_word() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::prev_word_index,
                |t, s| t.prev_word_index(s.start.index),
                Some(|w| w.rich_text_prev().next()),
                Self::local_text_end,
            )
        })
    }
    /// Like [`prev_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`prev_word`]: Self::prev_word
    pub fn local_prev_word() -> Self {
        Self::new(|| {
            next_prev(
                true,
                SegmentedText::prev_word_index,
                |t, s| t.prev_word_index(s.start.index),
                None,
                || unreachable!(),
            )
        })
    }

    /// Extend or shrink selection by moving the caret to the previous word insert index.
    ///
    /// This is the `CTRL+SHIFT+Left` shortcut operation.
    pub fn select_prev_word() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::prev_word_index,
                |_, _| unreachable!(),
                Some(|w| w.rich_text_prev().next()),
                Self::local_text_end, // !!: TODO
            )
        })
    }
    /// Like [`select_prev_word`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_prev_word`]: Self::select_prev_word
    pub fn local_select_prev_word() -> Self {
        Self::new(|| {
            next_prev(
                false,
                SegmentedText::prev_word_index,
                |_, _| unreachable!(),
                None,
                || unreachable!(),
            )
        })
    }

    /// Clear selection and move the caret to the nearest insert index on the previous line.
    ///
    /// This is the `Up` key operation.
    pub fn line_up() -> Self {
        Self::new(|| line_up_down(true, -1))
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the previous line.
    ///
    /// This is the `SHIFT+Up` key operation.
    pub fn select_line_up() -> Self {
        Self::new(|| line_up_down(false, -1))
    }

    /// Clear selection and move the caret to the nearest insert index on the next line.
    ///
    /// This is the `Down` key operation.
    pub fn line_down() -> Self {
        Self::new(|| line_up_down(true, 1))
    }

    /// Extend or shrink selection by moving the caret to the nearest insert index on the next line.
    ///
    /// This is the `SHIFT+Down` key operation.
    pub fn select_line_down() -> Self {
        Self::new(|| line_up_down(false, 1))
    }

    /// Clear selection and move the caret one viewport up.
    ///
    /// This is the `PageUp` key operation.
    pub fn page_up() -> Self {
        Self::new(|| page_up_down(true, -1))
    }

    /// Extend or shrink selection by moving the caret one viewport up.
    ///
    /// This is the `SHIFT+PageUp` key operation.
    pub fn select_page_up() -> Self {
        Self::new(|| page_up_down(false, -1))
    }

    /// Clear selection and move the caret one viewport down.
    ///
    /// This is the `PageDown` key operation.
    pub fn page_down() -> Self {
        Self::new(|| page_up_down(true, 1))
    }

    /// Extend or shrink selection by moving the caret one viewport down.
    ///
    /// This is the `SHIFT+PageDown` key operation.
    pub fn select_page_down() -> Self {
        Self::new(|| page_up_down(false, 1))
    }

    /// Clear selection and move the caret to the start of the line.
    ///
    /// This is the `Home` key operation.
    pub fn line_start() -> Self {
        Self::new(|| line_start_end(true, |li| li.text_range().start))
    }

    /// Extend or shrink selection by moving the caret to the start of the line.
    ///
    /// This is the `SHIFT+Home` key operation.
    pub fn select_line_start() -> Self {
        Self::new(|| line_start_end(false, |li| li.text_range().start))
    }

    /// Clear selection and move the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `End` key operation.
    pub fn line_end() -> Self {
        Self::new(|| line_start_end(true, |li| li.text_caret_range().end))
    }

    /// Extend or shrink selection by moving the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `SHIFT+End` key operation.
    pub fn select_line_end() -> Self {
        Self::new(|| line_start_end(false, |li| li.text_caret_range().end))
    }

    /// Clear selection and move the caret to the text start.
    ///
    /// This is the `CTRL+Home` shortcut operation.
    pub fn text_start() -> Self {
        Self::new(|| text_start_end(true, |_| 0, Some(|w| w.rich_text_leafs().next()), Self::local_text_start))
    }
    /// Like [`text_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`text_start`]: Self::text_start
    pub fn local_text_start() -> Self {
        Self::new(|| text_start_end(true, |_| 0, None, || unreachable!()))
    }

    /// Extend or shrink selection by moving the caret to the text start.
    ///
    /// This is the `CTRL+SHIFT+Home` shortcut operation.
    pub fn select_text_start() -> Self {
        // !!: TODO
        Self::new(|| text_start_end(false, |_| 0, Some(|w| w.rich_text_leafs().next()), Self::local_select_text_start))
    }
    /// Like [`select_text_start`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_text_start`]: Self::select_text_start
    pub fn local_select_text_start() -> Self {
        Self::new(|| text_start_end(false, |_| 0, None, || unreachable!()))
    }

    /// Clear selection and move the caret to the text end.
    ///
    /// This is the `CTRL+End` shortcut operation.
    pub fn text_end() -> Self {
        Self::new(|| text_start_end(true, |s| s.len(), Some(|w| w.rich_text_leafs_rev().next()), Self::local_text_end))
    }
    /// Like [`text_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`text_end`]: Self::text_end
    pub fn local_text_end() -> Self {
        Self::new(|| text_start_end(true, |s| s.len(), None, || unreachable!()))
    }

    /// Extend or shrink selection by moving the caret to the text end.
    ///
    /// This is the `CTRL+SHIFT+End` shortcut operation.
    pub fn select_text_end() -> Self {
        // !!: TODO
        Self::new(|| {
            text_start_end(
                false,
                |s| s.len(),
                Some(|w| w.rich_text_leafs_rev().next()),
                Self::local_select_text_end,
            )
        })
    }
    /// Like [`select_text_end`] but stays within the same text widget, ignores rich text context.
    ///
    /// [`select_text_end`]: Self::select_text_end
    pub fn local_select_text_end() -> Self {
        Self::new(|| text_start_end(false, |s| s.len(), None, || unreachable!()))
    }

    /// Clear selection and move the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down operation.
    pub fn nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            nearest_to(true, window_point);
        })
    }

    /// Extend or shrink selection by moving the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down when holding SHIFT operation.
    pub fn select_nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            nearest_to(false, window_point);
        })
    }

    /// Extend or shrink selection by moving the caret index or caret selection index to the insert point nearest to `window_point`.
    ///
    /// This is the touch selection caret drag operation.
    pub fn select_index_nearest_to(window_point: DipPoint, move_selection_index: bool) -> Self {
        Self::new(move || {
            index_nearest_to(window_point, move_selection_index);
        })
    }

    /// Replace or extend selection with the word nearest to the `window_point`
    ///
    /// This is the mouse primary button double click.
    pub fn select_word_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || select_line_word_nearest_to(replace_selection, true, window_point))
    }

    /// Replace or extend selection with the line nearest to the `window_point`
    ///
    /// This is the mouse primary button triple click.
    pub fn select_line_nearest_to(replace_selection: bool, window_point: DipPoint) -> Self {
        Self::new(move || select_line_word_nearest_to(replace_selection, false, window_point))
    }

    /// Select the full text.
    pub fn select_all() -> Self {
        Self::new(|| {
            if let Some(ctx) = TEXT.try_rich() {
                if let Some(info) = ctx.root_info() {
                    let mut last_id = None;
                    for leaf in info.rich_text_leafs() {
                        let leaf_id = leaf.info().id();
                        SELECT_CMD.scoped(leaf_id).notify_param(Self::local_select_all());
                        last_id = Some(leaf_id);
                    }
                    if let Some(last_id) = last_id {
                        let current_id = WIDGET.id();
                        if last_id != current_id && FOCUS.is_focused(current_id).get() {
                            FOCUS.focus_widget(last_id, false);
                        }
                        return;
                    }
                }
            }
            Self::local_select_all().call();
        })
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

    pub(super) fn call(self) {
        (self.op.lock())();
    }
}

fn next_prev(
    clear_selection: bool,
    index_from_caret: fn(&SegmentedText, usize) -> usize,
    index_from_selection: fn(&SegmentedText, ops::Range<CaretIndex>) -> usize,
    widget_from_current: Option<fn(WidgetInfo) -> Option<WidgetFocusInfo>>,
    rich_text_op: fn() -> TextSelectOp,
) {
    // compute next caret position
    let resolved = TEXT.resolved();
    let mut next_index = resolved.caret.index.unwrap_or(CaretIndex::ZERO);
    let current_index = next_index;
    if clear_selection {
        next_index.index = if let Some(s) = resolved.caret.selection_range() {
            // get if selection collapses to the start or end position
            index_from_selection(&resolved.segmented_text, s)
        } else {
            // no selection to clear, just advance the caret
            index_from_caret(&resolved.segmented_text, next_index.index)
        };
    } else {
        // no selection to clear, just advance the caret
        next_index.index = index_from_caret(&resolved.segmented_text, next_index.index);
    }
    drop(resolved);

    // update caret and selection
    let mut c = TEXT.resolve_caret();
    if clear_selection {
        c.clear_selection();
    } else if c.selection_index.is_none() {
        // starting selection (shift+arrow)
        c.selection_index = Some(current_index);
    }
    c.set_index(next_index);
    c.used_retained_x = false;

    if current_index == next_index || next_index == CaretIndex::ZERO {
        // !!: TODO prev_word to next_index=0 causes jump to start
        if let Some(widget_from_current) = widget_from_current {
            if let Some(_ctx) = TEXT.try_rich() {
                let info = WIDGET.info();
                if matches!(info.rich_text_component(), Some(RichTextComponent::Leaf { .. })) {
                    if let Some(next) = widget_from_current(info) {
                        let next_id = next.info().id();
                        let cmd = SELECT_CMD.scoped(next_id);
                        if cmd.is_enabled().get() {
                            if FOCUS.is_focused(WIDGET.id()).get() {
                                FOCUS.focus_widget(next_id, false);
                            }
                            cmd.notify_param(rich_text_op());
                        }
                    }
                }
            }
        }
    }
}

fn line_up_down(clear_selection: bool, diff: i8) {
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

fn page_up_down(clear_selection: bool, diff: i8) {
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

fn line_start_end(clear_selection: bool, index: impl FnOnce(ShapedLine) -> usize) {
    let mut caret = TEXT.resolve_caret();
    let mut i = caret.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        caret.clear_selection();
    } else if caret.selection_index.is_none() {
        caret.selection_index = Some(i);
    }

    if let Some(li) = TEXT.laidout().shaped_text.line(i.line) {
        i.index = index(li);
        caret.set_index(i);
        caret.used_retained_x = false;
    }
}

fn text_start_end(
    clear_selection: bool,
    index: impl FnOnce(&str) -> usize,
    widget_from_root: Option<fn(WidgetInfo) -> Option<WidgetFocusInfo>>,
    rich_text_op: fn() -> TextSelectOp,
) {
    if let Some(widget_from_root) = widget_from_root {
        if let Some(w) = TEXT.try_rich() {
            if let Some(root) = w.root_info() {
                if let Some(start) = widget_from_root(root) {
                    let id = start.info().id();
                    let current_id = WIDGET.id();
                    if id != current_id {
                        let cmd = SELECT_CMD.scoped(id);
                        if cmd.is_enabled().get() {
                            if FOCUS.is_focused(current_id).get() {
                                FOCUS.focus_widget(id, false);
                            }
                            SELECT_CMD.scoped(id).notify_param(rich_text_op());
                        }
                        return;
                    }
                }
            }
        }
    }

    let idx = index(TEXT.resolved().segmented_text.text());

    let mut caret = TEXT.resolve_caret();
    let mut i = caret.index.unwrap_or(CaretIndex::ZERO);
    if clear_selection {
        caret.clear_selection();
    } else if caret.selection_index.is_none() {
        caret.selection_index = Some(i);
    }

    i.index = idx;

    caret.set_index(i);
    caret.used_retained_x = false;
}

fn nearest_to(clear_selection: bool, window_point: DipPoint) {
    let mut caret = TEXT.resolve_caret();
    let mut i = caret.index.unwrap_or(CaretIndex::ZERO);

    if clear_selection {
        caret.clear_selection();
    } else if caret.selection_index.is_none() {
        caret.selection_index = Some(i);
    } else if let Some((_, is_word)) = caret.initial_selection.clone() {
        drop(caret);
        return select_line_word_nearest_to(false, is_word, window_point);
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

fn index_nearest_to(window_point: DipPoint, move_selection_index: bool) {
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

fn select_line_word_nearest_to(replace_selection: bool, select_word: bool, window_point: DipPoint) {
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
