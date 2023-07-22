//! Commands that control the editable text.
//!
//! Most of the normal text editing is controlled by keyboard events, the [`EDIT_CMD`]
//! command allows for arbitrary text editing without needing to simulate keyboard events.
//!
//! The [`nodes::resolve_text`] node implements [`EDIT_CMD`] when the text is editable.

use std::{any::Any, fmt, ops, sync::Arc};

use crate::core::{task::parking_lot::Mutex, undo::*};

use super::{
    nodes::{LayoutText, ResolvedText},
    *,
};

command! {
    /// Applies the [`TextEditOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static EDIT_CMD;

    /// Applies the [`TextSelectOp`] into the text if it is editable.
    ///
    /// The request must be set as the command parameter.
    pub static SELECT_CMD;
}

struct SharedTextEditOp {
    data: Box<dyn Any + Send>,
    op: Box<dyn FnMut(&BoxedVar<Txt>, &mut dyn Any, UndoFullOp) + Send>,
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
    /// the [`nodes::resolve_text`] context. You can position the caret using [`ResolvedText::caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    ///
    /// The `op` arguments are the text variable, a custom data `D` and what [`UndoFullOp`] query, all
    /// text edit operations must be undoable, first [`UndoOp::Redo`] is called to "do", then undo and redo again
    /// if the user requests undo & redo. The text variable is always read-write when `op` is called, more than
    /// one op can be called before the text variable updates, and [`ResolvedText::pending_edit`] is always false.
    pub fn new<D>(data: D, mut op: impl FnMut(&BoxedVar<Txt>, &mut D, UndoFullOp) + Send + 'static) -> Self
    where
        D: Send + Any + 'static,
    {
        Self(Arc::new(Mutex::new(SharedTextEditOp {
            data: Box::new(data),
            op: Box::new(move |var, data, o| op(var, data.downcast_mut().unwrap(), o)),
        })))
    }

    /// Insert operation.
    ///
    /// The `insert` text is inserted at the current caret index or at `0`, or replaces the current selection,
    /// after insert the caret is positioned after the inserted text.
    pub fn insert(insert: impl Into<Txt>) -> Self {
        let insert = insert.into();
        let mut insert_idx = CaretIndex {
            index: usize::MAX,
            line: 0,
        };
        Self::new((), move |txt, _, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
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
            UndoFullOp::Op(UndoOp::Undo) => {
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
            UndoFullOp::Info { info } => {
                let label = if insert.chars().any(|c| c.is_control() || c.is_whitespace()) {
                    formatx!("{insert:?}")
                } else {
                    insert.clone()
                };
                *info = Some(Arc::new(label));
            }
            UndoFullOp::Merge { .. } => {}
        })
    }

    /// Remove one *backspace range* ending at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::backspace_range`] for more details about what is removed.
    pub fn backspace() -> Self {
        let mut removed = Txt::from_static("");
        let mut undo_idx = CaretIndex {
            index: usize::MAX,
            line: 0,
        };

        Self::new((), move |txt, _, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
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
            UndoFullOp::Op(UndoOp::Undo) => {
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
            UndoFullOp::Info { info } => *info = Some(Arc::new("⌫")),
            UndoFullOp::Merge { .. } => {}
        })
    }

    /// Remove one *delete range* starting at the caret index, or removes the selection.
    ///
    /// See [`zero_ui::core::text::SegmentedText::delete_range`] for more details about what is removed.
    pub fn delete() -> Self {
        let mut removed = Txt::from_static("");

        Self::new((), move |txt, _, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
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
            UndoFullOp::Op(UndoOp::Undo) => {
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
            UndoFullOp::Info { info } => *info = Some(Arc::new("⌦")),
            UndoFullOp::Merge { .. } => {}
        })
    }

    /// Replace operation.
    ///
    /// The `select_before` is removed, and `insert` inserted at the `select_before.start`, after insertion
    /// the `select_after` is applied, you can use an empty insert to just remove.
    ///
    /// All indexes are snapped to the nearest grapheme, you can use empty ranges to just position the caret.
    pub fn replace(mut select_before: ops::Range<usize>, insert: impl Into<Txt>, mut select_after: ops::Range<usize>) -> Self {
        let insert = insert.into();
        let mut removed = Txt::from_static("");

        Self::new((), move |txt, _, op| match op {
            UndoFullOp::Op(UndoOp::Redo) => {
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
            UndoFullOp::Op(UndoOp::Undo) => {
                let ctx = ResolvedText::get();

                select_after.start = ctx.text.snap_grapheme_boundary(select_after.start);
                select_after.end = ctx.text.snap_grapheme_boundary(select_after.end);

                txt.modify(clmv!(select_after, removed, |args| {
                    args.to_mut().to_mut().replace_range(select_after, removed.as_str());
                }))
                .unwrap();

                ctx.caret.lock().set_char_index(select_before.start); // TODO, selection
            }
            UndoFullOp::Info { info } => *info = Some(Arc::new("↹")),
            UndoFullOp::Merge { .. } => {}
        })
    }

    pub(super) fn call(self, text: &BoxedVar<Txt>) {
        {
            let mut op = self.0.lock();
            let op = &mut *op;
            (op.op)(text, &mut *op.data, UndoFullOp::Op(UndoOp::Redo));
        }
        UNDO.register(UndoTextEditOp::new(self));
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
        let mut op = self.edit_op.0.lock();
        let op = &mut *op;
        (op.op)(text, &mut *op.data, UndoFullOp::Op(self.exec_op))
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
        let none_var = LocalVar(Txt::from_static("")).boxed();
        (op.op)(&none_var, &mut *op.data, UndoFullOp::Info { info: &mut info });

        info.unwrap_or_else(|| Arc::new("text edit"))
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
                let none_var = LocalVar(Txt::from_static("")).boxed();

                let mut next_op = next.edit_op.0.lock();

                (op.op)(
                    &none_var,
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
        let none_var = LocalVar(Txt::from_static("")).boxed();
        (op.op)(&none_var, &mut *op.data, UndoFullOp::Info { info: &mut info });

        info.unwrap_or_else(|| Arc::new("text edit"))
    }
}

/// Represents a text selection operation that can be send to an editable text using [`SELECT_CMD`].
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
    /// the [`nodes::layout_text`] context. You can position the caret using [`ResolvedText::caret`],
    /// the text widget will detect changes to it and react accordingly (updating caret position and animation),
    /// the caret index is also snapped to the nearest grapheme start.
    pub fn new(op: impl FnMut() + Send + 'static) -> Self {
        Self {
            op: Arc::new(Mutex::new(op)),
        }
    }

    /// Clear selection and move the caret on the next insert index.
    ///
    /// This is the `Right` key operation.
    pub fn grapheme_next() -> Self {
        Self::new(|| {
            let ctx = ResolvedText::get();
            let mut c = ctx.caret.lock();
            let mut caret = c.index.unwrap_or(CaretIndex::ZERO);
            caret.index = ctx.text.next_insert_index(caret.index);
            c.index = Some(caret);
            c.used_retained_x = false;
        })
    }

    /// Clear selection and move the caret to the previous insert index.
    ///
    /// This is the `Left` key operation.
    pub fn grapheme_prev() -> Self {
        Self::new(|| {
            let ctx = ResolvedText::get();
            let mut c = ctx.caret.lock();
            let mut caret = c.index.unwrap_or(CaretIndex::ZERO);
            caret.index = ctx.text.prev_insert_index(caret.index);
            c.index = Some(caret);
            c.used_retained_x = false;
        })
    }

    /// Clear selection and move the caret to the next word insert index.
    ///
    /// This is the `CTRL+Right` shortcut operation.
    pub fn word_next() -> Self {
        Self::new(|| {
            let ctx = ResolvedText::get();
            let mut c = ctx.caret.lock();
            let mut caret = c.index.unwrap_or(CaretIndex::ZERO);
            caret.index = ctx.text.next_word_index(caret.index);
            c.index = Some(caret);
            c.used_retained_x = false;
        })
    }

    /// Clear selection and move the caret to the previous word insert index.
    ///
    /// This is the `CTRL+Left` shortcut operation.
    pub fn word_prev() -> Self {
        Self::new(|| {
            let ctx = ResolvedText::get();
            let mut c = ctx.caret.lock();
            let mut caret = c.index.unwrap_or(CaretIndex::ZERO);
            caret.index = ctx.text.prev_word_index(caret.index);
            c.index = Some(caret);
            c.used_retained_x = false;
        })
    }

    /// Clear selection and move the caret in the nearest insert index on the previous line.
    ///
    /// This is the `Up` key operation.
    pub fn line_up() -> Self {
        Self::new(|| line_up_down(-1))
    }

    /// Clear selection and move the caret in the nearest insert index on the next line.
    ///
    /// This is the `Down` key operation.
    pub fn line_down() -> Self {
        Self::new(|| line_up_down(1))
    }

    /// Clear selection and move the caret one viewport up.
    ///
    /// This is the `PageUp` key operation.
    pub fn page_up() -> Self {
        Self::new(|| page_up_down(-1))
    }

    /// Clear selection and move the caret one viewport down.
    ///
    /// This is the `PageDown` key operation.
    pub fn page_down() -> Self {
        Self::new(|| page_up_down(1))
    }

    /// Clear selection and move the caret to the start of the line.
    ///
    /// This is the `Home` key operation.
    pub fn line_start() -> Self {
        Self::new(|| {
            let resolved = ResolvedText::get();
            let layout = LayoutText::get();

            let mut caret = resolved.caret.lock();
            if let Some(i) = &mut caret.index {
                if let Some(li) = layout.shaped_text.line(i.line) {
                    i.index = li.text_range().start;
                    caret.used_retained_x = false;
                }
            }
        })
    }

    /// Clear selection and move the caret to the end of the line (before the line-break if any).
    ///
    /// This is the `End` key operation.
    pub fn line_end() -> Self {
        Self::new(|| {
            let resolved = ResolvedText::get();
            let layout = LayoutText::get();

            let mut caret = resolved.caret.lock();
            if let Some(i) = &mut caret.index {
                if let Some(li) = layout.shaped_text.line(i.line) {
                    i.index = li.text_caret_range().end;
                    caret.used_retained_x = false;
                }
            }
        })
    }

    /// Clear selection and move the caret to the text start.
    ///
    /// This is the `CTRL+Home` shortcut operation.
    pub fn text_start() -> Self {
        Self::new(|| {
            let resolved = ResolvedText::get();
            let mut caret = resolved.caret.lock();
            caret.set_index(CaretIndex::ZERO);
            caret.used_retained_x = false;
        })
    }

    /// Clear selection and move the caret to the text end.
    ///
    /// This is the `CTRL+End` shortcut operation.
    pub fn text_end() -> Self {
        Self::new(|| {
            let resolved = ResolvedText::get();
            let mut c = CaretIndex::ZERO;
            c.index = resolved.text.len();
            let mut caret = resolved.caret.lock();
            caret.set_index(c);
            caret.used_retained_x = false;
        })
    }

    /// Clear selection and move the caret to the insert point nearest to the `window_point`.
    ///
    /// This is the mouse primary button down operation.
    pub fn nearest_to(window_point: DipPoint) -> Self {
        Self::new(move || {
            let resolved = ResolvedText::get();
            let layout = LayoutText::get();

            let mut caret = resolved.caret.lock();
            let caret = &mut *caret;

            caret.used_retained_x = false;

            //if there was at least one layout
            let info = layout.render_info.lock();
            if let Some(pos) = info
                .transform
                .inverse()
                .and_then(|t| t.transform_point(window_point.to_px(info.scale_factor.0)))
            {
                //if has rendered
                let mut i = match layout.shaped_text.nearest_line(pos.y) {
                    Some(l) => CaretIndex {
                        line: l.index(),
                        index: match l.nearest_seg(pos.x) {
                            Some(s) => s.nearest_char_index(pos.x, resolved.text.text()),
                            None => l.text_range().end,
                        },
                    },
                    None => CaretIndex::ZERO,
                };
                i.index = resolved.text.snap_grapheme_boundary(i.index);
                caret.set_index(i);
            }

            if caret.index.is_none() {
                caret.set_index(CaretIndex::ZERO);
            }
        })
    }

    pub(super) fn call(self) {
        (self.op.lock())();
    }
}

fn line_up_down(diff: i8) {
    let diff = diff as isize;
    let resolved = ResolvedText::get();
    let layout = LayoutText::get();

    let mut caret = resolved.caret.lock();
    let caret = &mut *caret;
    let caret_index = &mut caret.index;

    caret.used_retained_x = true;

    if layout.caret_origin.is_some() {
        let mut i = caret_index.unwrap_or(CaretIndex::ZERO);
        let last_line = layout.shaped_text.lines_len().saturating_sub(1);
        let li = i.line;
        let next_li = li.saturating_add_signed(diff).min(last_line);
        if li != next_li {
            match layout.shaped_text.line(next_li) {
                Some(l) => {
                    i.line = next_li;
                    i.index = match l.nearest_seg(layout.caret_retained_x) {
                        Some(s) => s.nearest_char_index(layout.caret_retained_x, resolved.text.text()),
                        None => l.text_range().end,
                    }
                }
                None => i = CaretIndex::ZERO,
            };
            i.index = resolved.text.snap_grapheme_boundary(i.index);
            *caret_index = Some(i);
        }
    }

    if caret_index.is_none() {
        *caret_index = Some(CaretIndex::ZERO);
    }
}

fn page_up_down(diff: i8) {
    let diff = diff as i32;
    let resolved = ResolvedText::get();
    let layout = LayoutText::get();

    let mut caret = resolved.caret.lock();
    let caret = &mut *caret;
    let caret_index = &mut caret.index;

    let page_y = layout.viewport.height * Px(diff);
    caret.used_retained_x = true;
    if layout.caret_origin.is_some() {
        let mut i = caret_index.unwrap_or(CaretIndex::ZERO);
        let li = i.line;
        if let Some(li) = layout.shaped_text.line(li) {
            let target_line_y = li.rect().origin.y + page_y;
            match layout.shaped_text.nearest_line(target_line_y) {
                Some(l) => {
                    i.line = l.index();
                    i.index = match l.nearest_seg(layout.caret_retained_x) {
                        Some(s) => s.nearest_char_index(layout.caret_retained_x, resolved.text.text()),
                        None => l.text_range().end,
                    }
                }
                None => i = CaretIndex::ZERO,
            };
            i.index = resolved.text.snap_grapheme_boundary(i.index);
            *caret_index = Some(i);
        }
    }

    if caret_index.is_none() {
        *caret_index = Some(CaretIndex::ZERO);
    }
}
