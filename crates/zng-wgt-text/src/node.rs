//! UI nodes used for building a text widget.

use std::{fmt, num::Wrapping, ops, sync::Arc};

use super::text_properties::*;
use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use zng_app::render::FontSynthesis;
use zng_app_context::{MappedRwLockWriteGuardOwned, RwLockReadGuardOwned, RwLockWriteGuardOwned};
use zng_ext_font::{CaretIndex, FontFaceList, FontList, SegmentedText, ShapedLine, ShapedText, TextOverflowInfo};
use zng_ext_input::{
    focus::FOCUS_CHANGED_EVENT,
    keyboard::{KEY_INPUT_EVENT, Key, KeyState},
    mouse::MOUSE_INPUT_EVENT,
    touch::{TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT},
};
use zng_ext_window::WINDOW_Ext as _;
use zng_view_api::{mouse::ButtonState, touch::TouchPhase};
use zng_wgt::prelude::*;
use zng_wgt_data::{DATA, DataNoteHandle};
use zng_wgt_layer::{
    AnchorMode, AnchorTransform,
    popup::{ContextCapture, POPUP, PopupState},
};

mod rich;
pub use rich::*;

mod resolve;
pub use resolve::*;

mod layout;
pub use layout::*;

mod render;
pub use render::*;

mod caret;
pub use caret::*;

/// Represents the caret position in a [`RichText`] context.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RichCaretInfo {
    /// Widget that defines the caret insert position.
    ///
    /// Inside the widget the [`CaretInfo::index`] defines the actual index.
    pub index: Option<WidgetId>,
    /// Widget that defines the selection second index.
    ///
    /// Inside the widget the [`CaretInfo::selection_index`] defines the actual index.
    pub selection_index: Option<WidgetId>,
}

/// Represents the caret position at the [`ResolvedText`] context.
#[derive(Clone)]
#[non_exhaustive]
pub struct CaretInfo {
    /// Caret opacity.
    ///
    /// This variable is replaced often, the text resolver subscribes to it for
    /// [`UpdateOp::RenderUpdate`] automatically.
    ///
    /// [`UpdateOp::RenderUpdate`]: zng_wgt::prelude::UpdateOp::RenderUpdate
    pub opacity: Var<Factor>,

    /// Caret byte offset in the text string.
    ///
    /// This is the insertion offset on the text, it can be the text length.
    pub index: Option<CaretIndex>,

    /// Second index that defines the start or end of a selection range.
    pub selection_index: Option<CaretIndex>,

    /// Selection by word or line sets this value, selection extend by word or line
    /// grows from this central selection. The value is `(selection, is_word)`.
    pub initial_selection: Option<(ops::Range<CaretIndex>, bool)>,

    /// Value incremented by one every time the `index` is set.
    ///
    /// This is used to signal interaction with the `index` value by [`TextEditOp`]
    /// even if the interaction only sets-it to the index same value.
    ///
    /// [`TextEditOp`]: crate::cmd::TextEditOp
    pub index_version: Wrapping<u8>,

    /// If the index was set by using the [`caret_retained_x`].
    ///
    /// [`caret_retained_x`]: LaidoutText::caret_retained_x
    pub used_retained_x: bool,

    /// Don't scroll to new caret position on the next update.
    ///
    /// If this is set to `true` the next time `index` or `index_version` changes auto-scroll is skipped once.
    pub skip_next_scroll: bool,
}
impl fmt::Debug for CaretInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CaretInfo")
            .field("opacity", &self.opacity.get_debug(false))
            .field("index", &self.index)
            .field("index_version", &self.index_version)
            .field("used_retained_x", &self.used_retained_x)
            .finish()
    }
}
impl CaretInfo {
    /// Set the index and update the index version.
    pub fn set_index(&mut self, index: CaretIndex) {
        self.index = Some(index);
        self.index_version += 1;
    }

    /// Sets the selection start, end and update the index version.
    ///
    /// The `end` is the caret position.
    pub fn set_selection(&mut self, start: CaretIndex, end: CaretIndex) {
        self.selection_index = Some(start);
        self.set_index(end);
    }

    /// Clears selection.
    pub fn clear_selection(&mut self) {
        self.selection_index = None;
        self.initial_selection = None;
        self.index_version += 1;
    }

    /// Set the char byte index and update the index version.
    ///
    /// The caret line is always snapped when the caret changes, so the line value will be updated.
    pub fn set_char_index(&mut self, index: usize) {
        if let Some(i) = &mut self.index {
            i.index = index;
        } else {
            self.index = Some(CaretIndex { index, line: 0 });
        }
        self.index_version += 1;
    }

    /// Set the char byte index of the selection start, end and update the index version.
    ///
    /// The `end` is the caret position.
    ///
    /// The caret and selection lines are always snapped when the caret changes, so the line values will be updated.
    pub fn set_char_selection(&mut self, start: usize, end: usize) {
        if let Some(s) = &mut self.selection_index {
            s.index = start;
        } else {
            self.selection_index = Some(CaretIndex { index: start, line: 0 });
        }
        self.set_char_index(end);
    }

    /// Gets the selection range if both [`index`] and [`selection_index`] are set.
    ///
    /// [`index`]: Self::index
    /// [`selection_index`]: Self::selection_index
    pub fn selection_range(&self) -> Option<ops::Range<CaretIndex>> {
        let a = self.index?;
        let b = self.selection_index?;

        use std::cmp::Ordering;
        match a.index.cmp(&b.index) {
            Ordering::Less => Some(a..b),
            Ordering::Equal => None,
            Ordering::Greater => Some(b..a),
        }
    }

    /// Gets the character range of the selection if both [`index`] and [`selection_index`] are set.
    ///
    /// [`index`]: Self::index
    /// [`selection_index`]: Self::selection_index
    pub fn selection_char_range(&self) -> Option<ops::Range<usize>> {
        self.selection_range().map(|r| r.start.index..r.end.index)
    }
}

/// IME text edit that is not committed yet.
#[derive(Clone)]
#[non_exhaustive]
pub struct ImePreview {
    /// The inserted text.
    pub txt: Txt,

    /// Caret index when IME started.
    pub prev_caret: CaretIndex,
    /// Selection index when IME started.
    ///
    /// If set defines a selection of the text variable that is replaced with the `txt`.
    pub prev_selection: Option<CaretIndex>,
}

/// Text internals used by text implementer nodes and properties.
///
/// The text implementation is split between two contexts, [`resolve_text`] and [`layout_text`], this service
/// provides access to data produced by these two contexts.
pub struct TEXT;

impl TEXT {
    /// Read lock the current rich text context if any parent widget defines it.
    pub fn try_rich(&self) -> Option<RwLockReadGuardOwned<RichText>> {
        if RICH_TEXT.is_default() {
            None
        } else {
            Some(RICH_TEXT.read_recursive())
        }
    }

    /// Read lock the current contextual resolved text if called in a node inside [`resolve_text`].
    ///
    /// Note that this will block until a read lock can be acquired.
    pub fn try_resolved(&self) -> Option<RwLockReadGuardOwned<ResolvedText>> {
        if RESOLVED_TEXT.is_default() {
            None
        } else {
            Some(RESOLVED_TEXT.read_recursive())
        }
    }

    /// Read lock the current rich text context.
    ///
    /// # Panics
    ///
    /// Panics if requested in a node outside [`rich_text`].
    ///
    /// [`rich_text`]: fn@crate::rich_text
    pub fn rich(&self) -> RwLockReadGuardOwned<RichText> {
        RICH_TEXT.read_recursive()
    }

    /// Read lock the current contextual resolved text.
    ///
    /// # Panics
    ///
    /// Panics if requested in a node outside [`resolve_text`].
    pub fn resolved(&self) -> RwLockReadGuardOwned<ResolvedText> {
        RESOLVED_TEXT.read_recursive()
    }

    /// Read lock the current contextual laidout text if called in a node inside [`layout_text`].
    ///
    /// Note that this will block until a read lock can be acquired.
    pub fn try_laidout(&self) -> Option<RwLockReadGuardOwned<LaidoutText>> {
        if LAIDOUT_TEXT.is_default() {
            None
        } else {
            Some(LAIDOUT_TEXT.read_recursive())
        }
    }

    /// Read lock the current contextual laidout text.
    ///
    /// # Panics
    ///
    /// Panics if not available in context. Is only available inside [`layout_text`] after the first layout.
    pub fn laidout(&self) -> RwLockReadGuardOwned<LaidoutText> {
        LAIDOUT_TEXT.read_recursive()
    }

    /// Write lock the current contextual resolved text to edit the caret.
    ///
    /// Note that the entire `ResolvedText` is exclusive locked, you cannot access the resolved text while holding this lock.
    ///     
    /// # Panics
    ///
    /// Panics if requested in a node outside [`resolve_text`].
    pub fn resolve_caret(&self) -> MappedRwLockWriteGuardOwned<ResolvedText, CaretInfo> {
        RwLockWriteGuardOwned::map(self.resolve(), |ctx| &mut ctx.caret)
    }

    /// Write lock the current contextual rich text to edit the caret.
    ///
    /// Note that the entire `RichText` is exclusive locked, you cannot access the rich text while holding this lock.
    ///
    /// # Panics
    ///
    /// Panics if requested in a node outside [`rich_text`].
    ///
    /// [`rich_text`]: fn@crate::rich_text
    pub fn resolve_rich_caret(&self) -> MappedRwLockWriteGuardOwned<RichText, RichCaretInfo> {
        RwLockWriteGuardOwned::map(RICH_TEXT.write(), |ctx| &mut ctx.caret)
    }

    /// Set the `caret_retained_x` value.
    ///
    /// Note that the value is already updated automatically on caret layout, this method is for rich text operations
    /// to propagate the line position between widgets.
    pub fn set_caret_retained_x(&self, x: Px) {
        self.layout().caret_retained_x = x;
    }

    pub(crate) fn resolve(&self) -> RwLockWriteGuardOwned<ResolvedText> {
        RESOLVED_TEXT.write()
    }

    fn layout(&self) -> RwLockWriteGuardOwned<LaidoutText> {
        LAIDOUT_TEXT.write()
    }

    pub(crate) fn take_rich_selection_started_by_alt(&self) -> bool {
        std::mem::take(&mut *RICH_TEXT_SELECTION_STARTED_BY_ALT.write())
    }

    pub(crate) fn flag_rich_selection_started_by_alt(&self) {
        *RICH_TEXT_SELECTION_STARTED_BY_ALT.write() = true;
    }
}

/// Defines the source of the current selection.
///
/// See [`ResolvedText::selection_by`] for more details.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, bytemuck::NoUninit)]
#[repr(u8)]
pub enum SelectionBy {
    /// Command or other programmatic selection.
    Command = 0,
    /// Key press.
    Keyboard = 1,
    /// Mouse drag.
    Mouse = 2,
    /// Touch drag.
    Touch = 3,
}
impl SelectionBy {
    /// Returns `true` if the interactive carets must be used for the current selection given the interactive caret mode.
    pub fn matches_interactive_mode(self, mode: InteractiveCaretMode) -> bool {
        match mode {
            InteractiveCaretMode::TouchOnly => matches!(self, SelectionBy::Touch),
            InteractiveCaretMode::Enabled => true,
            InteractiveCaretMode::Disabled => false,
        }
    }
}

/// Represents the resolved fonts and the transformed, white space corrected and segmented text.
///
/// Use [`TEXT`] to get.
#[non_exhaustive]
pub struct ResolvedText {
    /// The text source variable.
    pub txt: Var<Txt>,
    /// IME text edit that is not committed yet. Only the text in the segmented and shaped text is edited,
    /// the text variable is not updated yet and undo is not tracking these changes.
    pub ime_preview: Option<ImePreview>,

    /// Text transformed, white space corrected and segmented.
    pub segmented_text: SegmentedText,
    /// Queried font faces.
    pub faces: FontFaceList,
    /// Font synthesis allowed by the text context and required to render the best font match.
    pub synthesis: FontSynthesis,

    /// Layout that needs to be recomputed as identified by the text resolver node.
    ///
    /// This is added to the layout invalidation by the layout node itself. When set a layout must
    /// be requested for the widget.
    pub pending_layout: PendingLayout,

    /// Text modification is scheduled, caret info will only be valid after update.
    pub pending_edit: bool,

    /// Caret index and animation.
    pub caret: CaretInfo,

    /// Source of the current selection.
    pub selection_by: SelectionBy,

    /// If the selection toolbar is open.
    pub selection_toolbar_is_open: bool,
}
impl fmt::Debug for ResolvedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedText")
            .field("segmented_text", &self.segmented_text)
            .field("faces", &self.faces)
            .field("synthesis", &self.synthesis)
            .field("pending_layout", &self.pending_layout)
            .field("pending_edit", &self.pending_edit)
            .field("caret", &self.caret)
            .field("selection_by", &self.selection_by)
            .field("selection_toolbar_is_open", &self.selection_toolbar_is_open)
            .finish_non_exhaustive()
    }
}
impl ResolvedText {
    fn no_context() -> Self {
        panic!("no `ResolvedText` in context, only available inside `resolve_text`")
    }
}

/// Info about the last text render or render update.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RenderInfo {
    /// Render transform of the text, in the window space.
    pub transform: PxTransform,
    /// Render scale factor of the text.
    pub scale_factor: Factor,
}
impl Default for RenderInfo {
    /// Identify, 1.fct()
    fn default() -> Self {
        Self {
            transform: PxTransform::identity(),
            scale_factor: 1.fct(),
        }
    }
}

/// Represents the laidout text.
///
/// Use [`TEXT`] to get.
#[derive(Debug)]
#[non_exhaustive]
pub struct LaidoutText {
    /// Sized [`faces`].
    ///
    /// [`faces`]: ResolvedText::faces
    pub fonts: FontList,

    /// Layout text.
    pub shaped_text: ShapedText,

    /// Shaped text overflow info.
    pub overflow: Option<TextOverflowInfo>,

    /// Shaped text used as suffix when `shaped_text` overflows.
    pub overflow_suffix: Option<ShapedText>,

    /// Version updated every time the `shaped_text` is reshaped.
    pub shaped_text_version: u32,

    /// List of overline segments, defining origin and width of each line.
    ///
    /// Note that overlines are only computed if the `overline_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_overlines`].
    pub overlines: Vec<(PxPoint, Px)>,

    /// Computed [`OVERLINE_THICKNESS_VAR`].
    pub overline_thickness: Px,

    /// List of strikethrough segments, defining origin and width of each line.
    ///
    /// Note that strikethroughs are only computed if the `strikethrough_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_strikethroughs`].
    pub strikethroughs: Vec<(PxPoint, Px)>,
    /// Computed [`STRIKETHROUGH_THICKNESS_VAR`].
    pub strikethrough_thickness: Px,

    /// List of underline segments, defining origin and width of each line.
    ///
    /// Note that underlines are only computed if the `underline_thickness` is more than `0`. These
    /// underlines never cover the IME review text range.
    ///
    /// Default underlines are rendered by [`render_underlines`].
    pub underlines: Vec<(PxPoint, Px)>,
    /// Computed [`UNDERLINE_THICKNESS_VAR`].
    pub underline_thickness: Px,

    /// List of underline segments for IME preview text, defining origin and width of each line.
    ///
    /// Note that underlines are only computed if the `ime_underline_thickness` is more than `0`.
    ///
    /// Default underlines are rendered by [`render_underlines`].
    pub ime_underlines: Vec<(PxPoint, Px)>,
    /// Computed [`IME_UNDERLINE_THICKNESS_VAR`].
    pub ime_underline_thickness: Px,

    /// Top-middle offset of the caret index in the shaped text.
    pub caret_origin: Option<PxPoint>,

    /// Top-middle offset of the caret selection_index in the shaped text.
    pub caret_selection_origin: Option<PxPoint>,

    /// The x offset used when pressing up or down.
    pub caret_retained_x: Px,

    /// Info about the last text render or render update.
    pub render_info: RenderInfo,

    /// Latest layout viewport.
    pub viewport: PxSize,
}
impl LaidoutText {
    fn no_context() -> Self {
        panic!("no `LaidoutText` in context, only available inside `layout_text`")
    }
}

/// Represents the rich text context.
///
/// Use [`TEXT`] to get.
#[non_exhaustive]
pub struct RichText {
    /// Widget that defines the rich text context.
    pub root_id: WidgetId,

    /// Widgets that define the caret and selection indexes.
    pub caret: RichCaretInfo,
}
impl RichText {
    fn no_context() -> Self {
        panic!("no `RichText` in context, only available inside `rich_text`")
    }
    fn no_dispatch_context() -> Vec<EventUpdate> {
        panic!("`RichText::notify_leaf` must be called inside `UiNode::event` only")
    }
}

context_local! {
    /// Represents the contextual [`RichText`] setup by the [`rich_text`] property.
    static RICH_TEXT: RwLock<RichText> = RwLock::new(RichText::no_context());
    /// Represents the contextual [`ResolvedText`] setup by the [`resolve_text`] node.
    static RESOLVED_TEXT: RwLock<ResolvedText> = RwLock::new(ResolvedText::no_context());
    /// Represents the contextual [`LaidoutText`] setup by the [`layout_text`] node.
    static LAIDOUT_TEXT: RwLock<LaidoutText> = RwLock::new(LaidoutText::no_context());
    /// Represents a list of events send from rich text leaves to other leaves.
    static RICH_TEXT_NOTIFY: RwLock<Vec<EventUpdate>> = RwLock::new(RichText::no_dispatch_context());
    /// TODO(breaking) refactor into RichCaretInfo private field.
    static RICH_TEXT_SELECTION_STARTED_BY_ALT: RwLock<bool> = RwLock::new(false);
}

impl RichText {
    /// Send an event *immediately* to a leaf widget inside the rich context.
    ///
    /// After the current event returns to the rich text root widget the `update` is sent. Rich text leaves can send
    /// multiple commands to sibling leaves to implement rich text operations, using this method instead of the global dispatch
    /// can gain significant performance.
    ///
    /// Note that all requests during a single app event run after that event, and all recursive requests during these notification events only
    /// run after they all notify, that is, not actually recursive.
    ///
    /// # Panics
    ///
    /// Panics is not called during a `UiNode::event`.
    pub fn notify_leaf(&self, update: EventUpdate) {
        RICH_TEXT_NOTIFY.write().push(update);
    }
}

bitflags! {
    /// Text layout parts that need rebuild.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PendingLayout: u8 {
        /// Underline size and position.
        const UNDERLINE     = 0b0000_0001;
        /// Strikethrough size and position.
        const STRIKETHROUGH = 0b0000_0010;
        /// Overline size and position.
        const OVERLINE      = 0b0000_0100;
        /// Caret origin.
        const CARET         = 0b0000_1000;
        /// Overflow.
        const OVERFLOW      = 0b0001_0000;
        /// Text lines position, retains line glyphs but reposition for new align and outer box.
        const RESHAPE_LINES = 0b0111_1111;
        /// Full reshape, re-compute all glyphs.
        const RESHAPE       = 0b1111_1111;
    }
}

/// Create a node that is sized one text line height by `width`.
///
/// This node can be used to reserve space for a full text in lazy initing contexts.
///
/// The contextual variables affect the layout size.
pub fn line_placeholder(width: impl IntoVar<Length>) -> impl UiNode {
    let child = layout_text(FillUiNode);
    let child = resolve_text(child, " ");
    zng_wgt_size_offset::width(child, width)
}

pub(super) fn get_caret_index(child: impl UiNode, index: impl IntoVar<Option<CaretIndex>>) -> impl UiNode {
    let index = index.into_var();
    match_node(child, move |c, op| {
        let mut u = false;
        match op {
            UiNodeOp::Init => {
                c.init();
                index.set(TEXT.resolved().caret.index);
            }
            UiNodeOp::Deinit => {
                index.set(None);
            }
            UiNodeOp::Event { update } => {
                c.event(update);
                u = true;
            }
            UiNodeOp::Update { updates } => {
                c.update(updates);
                u = true;
            }
            _ => {}
        }
        if u {
            let t = TEXT.resolved();
            let idx = t.caret.index;
            if !t.pending_edit && index.get() != idx {
                index.set(idx);
            }
        }
    })
}

pub(super) fn get_caret_status(child: impl UiNode, status: impl IntoVar<CaretStatus>) -> impl UiNode {
    let status = status.into_var();
    match_node(child, move |c, op| {
        let mut u = false;
        match op {
            UiNodeOp::Init => {
                c.init();
                let t = TEXT.resolved();
                status.set(match t.caret.index {
                    None => CaretStatus::none(),
                    Some(i) => CaretStatus::new(i.index, &t.segmented_text),
                });
            }
            UiNodeOp::Deinit => {
                status.set(CaretStatus::none());
            }
            UiNodeOp::Event { update } => {
                c.event(update);
                u = true;
            }
            UiNodeOp::Update { updates } => {
                c.update(updates);
                u = true;
            }
            _ => {}
        }
        if u {
            let t = TEXT.resolved();
            let idx = t.caret.index;
            if !t.pending_edit && status.get().index() != idx.map(|ci| ci.index) {
                status.set(match idx {
                    None => CaretStatus::none(),
                    Some(i) => CaretStatus::new(i.index, &t.segmented_text),
                });
            }
        }
    })
}

pub(super) fn get_lines_len(child: impl UiNode, len: impl IntoVar<usize>) -> impl UiNode {
    let len = len.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Deinit => {
            len.set(0usize);
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            let t = TEXT.laidout();
            let l = t.shaped_text.lines_len();
            if l != len.get() {
                len.set(t.shaped_text.lines_len());
            }
        }
        _ => {}
    })
}

pub(super) fn get_lines_wrap_count(child: impl UiNode, lines: impl IntoVar<super::LinesWrapCount>) -> impl UiNode {
    let lines = lines.into_var();
    let mut version = 0;
    match_node(child, move |c, op| match op {
        UiNodeOp::Deinit => {
            lines.set(super::LinesWrapCount::NoWrap(0));
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            let t = TEXT.laidout();
            if t.shaped_text_version != version {
                version = t.shaped_text_version;
                if let Some(update) = lines.with(|l| lines_wrap_count(l, &t.shaped_text)) {
                    lines.set(update);
                }
            }
        }
        _ => {}
    })
}
// Returns `Some(_)` if the current wrap count changed from `prev`. Only allocates if new count has wrapped lines.
fn lines_wrap_count(prev: &super::LinesWrapCount, txt: &ShapedText) -> Option<super::LinesWrapCount> {
    match prev {
        super::LinesWrapCount::NoWrap(len) => {
            let mut counter = lines_wrap_counter(txt);
            let mut l = 0;
            for c in &mut counter {
                if c != 1 {
                    // at least one line wraps now
                    let mut wrap = vec![1; l];
                    wrap.push(c);
                    wrap.extend(&mut counter);
                    return Some(super::LinesWrapCount::Wrap(wrap));
                }
                l += 1;
            }
            if l != *len {
                // no line wraps, but changed line count.
                Some(super::LinesWrapCount::NoWrap(l))
            } else {
                None
            }
        }
        super::LinesWrapCount::Wrap(counts) => {
            // find `counts[i]` that diverges from counts, OR
            // find if all new counts is now NoWrap
            let mut prev_counts = counts.iter();
            let mut new_counts = lines_wrap_counter(txt);
            let mut eq_l = 0;
            let mut eq_wrap = false;
            for c in &mut new_counts {
                if prev_counts.next() == Some(&c) {
                    eq_l += 1;
                    eq_wrap |= c != 1;
                } else if eq_wrap || c != 1 {
                    // not eq, and already found a wrap line
                    let mut wrap = counts[..eq_l].to_vec();
                    wrap.push(c);
                    wrap.extend(&mut new_counts);
                    return Some(super::LinesWrapCount::Wrap(wrap));
                } else {
                    // not eq, but maybe no wrap
                    let mut l = eq_l + 1; // +1 is +c
                    for c in &mut new_counts {
                        if c != 1 {
                            // nope, found a line wrap
                            let mut wrap = vec![1; l];
                            wrap.push(c);
                            wrap.extend(&mut new_counts);
                            return Some(super::LinesWrapCount::Wrap(wrap));
                        }
                        l += 1;
                    }
                    // changed to no wrap
                    return Some(super::LinesWrapCount::NoWrap(l));
                }
            }
            if prev_counts.next().is_some() {
                Some(super::LinesWrapCount::Wrap(counts[..eq_l].to_vec()))
            } else {
                None
            }
        }
    }
}
fn lines_wrap_counter(txt: &ShapedText) -> impl Iterator<Item = u32> + '_ {
    struct Counter<I> {
        lines: I,
        count: u32,
    }
    impl<'a, I: Iterator<Item = ShapedLine<'a>>> Iterator for Counter<I> {
        type Item = u32;

        fn next(&mut self) -> Option<u32> {
            loop {
                let line = self.lines.next()?;
                if line.ended_by_wrap() {
                    self.count += 1;
                    continue;
                }

                let c = self.count;
                self.count = 1;

                return Some(c);
            }
        }
    }
    Counter {
        lines: txt.lines(),
        count: 1,
    }
}

pub(super) fn parse_text<T>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode
where
    T: super::TxtParseValue,
{
    let value = value.into_var();

    let error = var(Txt::from_static(""));
    let mut _error_note = DataNoteHandle::dummy();

    #[derive(Clone, Copy, bytemuck::NoUninit)]
    #[repr(u8)]
    enum State {
        Sync,
        Requested,
        Pending,
    }
    let state = Arc::new(Atomic::new(State::Sync));

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            let ctx = TEXT.resolved();

            // initial T -> Txt sync
            ctx.txt.set_from_map(&value, |val| val.to_txt());

            // bind `TXT_PARSE_LIVE_VAR` <-> `value` using `bind_filter_map_bidi`:
            // - in case of parse error, it is set in `error` variable, that is held by the binding.
            // - on error update the DATA note is updated.
            // - in case parse is not live, ignores updates (Txt -> None), sets `state` to `Pending`.
            // - in case of Pending and `PARSE_CMD` state is set to `Requested` and `TXT_PARSE_LIVE_VAR.update()`.
            // - the pending state is also tracked in `TXT_PARSE_PENDING_VAR` and the `PARSE_CMD` handle.

            let live = TXT_PARSE_LIVE_VAR.current_context();
            let is_pending = TXT_PARSE_PENDING_VAR.current_context();
            let cmd_handle = Arc::new(super::cmd::PARSE_CMD.scoped(WIDGET.id()).subscribe(false));

            let binding = ctx.txt.bind_filter_map_bidi(
                &value,
                clmv!(state, error, is_pending, cmd_handle, |txt| {
                    if live.get() || matches!(state.load(Ordering::Relaxed), State::Requested) {
                        // can try parse

                        if !matches!(state.swap(State::Sync, Ordering::Relaxed), State::Sync) {
                            // exit pending state, even if it parse fails
                            is_pending.set(false);
                            cmd_handle.set_enabled(false);
                        }

                        // try parse
                        match T::from_txt(txt) {
                            Ok(val) => {
                                error.set(Txt::from_static(""));
                                Some(val)
                            }
                            Err(e) => {
                                error.set(e);
                                None
                            }
                        }
                    } else {
                        // cannot try parse

                        if !matches!(state.swap(State::Pending, Ordering::Relaxed), State::Pending) {
                            // enter pending state
                            is_pending.set(true);
                            cmd_handle.set_enabled(true);
                        }

                        // does not update the value
                        None
                    }
                }),
                clmv!(state, error, |val| {
                    // value updated externally, exit error, exit pending.

                    error.set(Txt::from_static(""));

                    if !matches!(state.swap(State::Sync, Ordering::Relaxed), State::Sync) {
                        is_pending.set(false);
                        cmd_handle.set_enabled(false);
                    }

                    Some(val.to_txt())
                }),
            );

            // cmd_handle is held by the binding

            WIDGET.sub_var(&TXT_PARSE_LIVE_VAR).sub_var(&error).push_var_handles(binding);
        }
        UiNodeOp::Deinit => {
            _error_note = DataNoteHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = super::cmd::PARSE_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                if matches!(state.load(Ordering::Relaxed), State::Pending) {
                    // requested parse and parse is pending

                    state.store(State::Requested, Ordering::Relaxed);
                    TEXT.resolved().txt.update();
                    args.propagation().stop();
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(true) = TXT_PARSE_LIVE_VAR.get_new() {
                if matches!(state.load(Ordering::Relaxed), State::Pending) {
                    // enabled live parse and parse is pending

                    TEXT.resolved().txt.update();
                }
            }

            if let Some(error) = error.get_new() {
                // remove or replace the error

                _error_note = if error.is_empty() {
                    DataNoteHandle::dummy()
                } else {
                    DATA.invalidate(error)
                };
            }
        }
        _ => {}
    })
}

pub(super) fn on_change_stop(child: impl UiNode, mut handler: impl WidgetHandler<ChangeStopArgs>) -> impl UiNode {
    let mut pending = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Event { update } => {
            if pending.is_none() {
                return;
            }

            if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                if let KeyState::Pressed = args.state
                    && let Key::Enter = &args.key
                    && !ACCEPTS_ENTER_VAR.get()
                {
                    pending = None;
                    handler.event(&ChangeStopArgs {
                        cause: ChangeStopCause::Enter,
                    });
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                let target = WIDGET.id();
                if args.is_blur(target) {
                    pending = None;
                    handler.event(&ChangeStopArgs {
                        cause: ChangeStopCause::Blur,
                    });
                }
            }
        }
        UiNodeOp::Update { updates } => {
            if TEXT.resolved().txt.is_new() {
                let deadline = TIMERS.deadline(CHANGE_STOP_DELAY_VAR.get());
                deadline.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                pending = Some(deadline);
            } else if let Some(p) = &pending {
                if p.get().has_elapsed() {
                    pending = None;

                    handler.event(&ChangeStopArgs {
                        cause: ChangeStopCause::DelayElapsed,
                    });
                }
            }

            c.update(updates);
            handler.update();
        }
        _ => {}
    })
}

/// Implements the selection toolbar.
pub fn selection_toolbar_node(child: impl UiNode) -> impl UiNode {
    use super::node::*;

    let mut selection_range = None;
    let mut popup_state = None::<Var<PopupState>>;
    match_node(child, move |c, op| {
        let mut open = false;
        let mut open_long_press = false;
        let mut close = false;
        match op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&SELECTION_TOOLBAR_FN_VAR);
            }
            UiNodeOp::Deinit => {
                close = true;
            }
            UiNodeOp::Event { update } => {
                c.event(update);

                let open_id = || {
                    if let Some(popup_state) = &popup_state {
                        if let PopupState::Open(id) = popup_state.get() {
                            return Some(id);
                        }
                    }
                    None
                };

                if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    if open_id().map(|id| !args.target.contains(id)).unwrap_or(false) {
                        close = true;
                    }
                    if args.state == ButtonState::Released {
                        open = true;
                    }
                } else if TOUCH_LONG_PRESS_EVENT.has(update) {
                    open = true;
                    open_long_press = true;
                } else if KEY_INPUT_EVENT.has(update) {
                    close = true;
                } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                    if args.is_blur(WIDGET.id())
                        && open_id()
                            .map(|id| args.new_focus.as_ref().map(|p| !p.contains(id)).unwrap_or(true))
                            .unwrap_or(false)
                    {
                        close = true;
                    }
                } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
                    if matches!(args.phase, TouchPhase::Start | TouchPhase::Move)
                        && open_id().map(|id| !args.target.contains(id)).unwrap_or(false)
                    {
                        close = true;
                    }
                }

                if popup_state.is_some() {
                    let r_txt = TEXT.resolved();
                    if selection_range != r_txt.caret.selection_range() {
                        close = true;
                    }
                }
            }
            UiNodeOp::Update { .. } => {
                if SELECTION_TOOLBAR_FN_VAR.is_new() {
                    close = true;
                }
                if let Some(state) = popup_state.as_ref().and_then(|s| s.get_new()) {
                    let is_open = !matches!(state, PopupState::Closed);
                    let mut r_txt = TEXT.resolve();
                    if r_txt.selection_toolbar_is_open != is_open {
                        r_txt.selection_toolbar_is_open = is_open;
                        WIDGET.layout().render();

                        if !is_open {
                            popup_state = None;
                        }
                    }
                }
            }
            _ => {}
        }
        if close {
            if let Some(state) = &popup_state.take() {
                selection_range = None;
                POPUP.close(state);
                TEXT.resolve().selection_toolbar_is_open = false;
                WIDGET.layout().render();
            }
        }
        if open {
            let r_txt = TEXT.resolved();

            let range = r_txt.caret.selection_range();
            if open_long_press || range.is_some() {
                selection_range = range;

                let toolbar_fn = SELECTION_TOOLBAR_FN_VAR.get();
                if let Some(node) = toolbar_fn.call_checked(SelectionToolbarArgs {
                    anchor_id: WIDGET.id(),
                    is_touch: matches!(r_txt.selection_by, SelectionBy::Touch),
                }) {
                    let (node, _) = node.init_widget();

                    let mut translate = PxVector::zero();
                    let transform_key = FrameValueKey::new_unique();
                    let node = match_widget(node, move |c, op| match op {
                        UiNodeOp::Init => {
                            c.init();
                            c.with_context(WidgetUpdateMode::Bubble, || WIDGET.sub_var_layout(&SELECTION_TOOLBAR_ANCHOR_VAR));
                        }
                        UiNodeOp::Layout { wl, final_size } => {
                            let r_txt = TEXT.resolved();

                            let range = if open_long_press {
                                Some(r_txt.caret.selection_range().unwrap_or_else(|| {
                                    let i = r_txt.caret.index.unwrap_or(CaretIndex::ZERO);
                                    i..i
                                }))
                            } else {
                                r_txt.caret.selection_range()
                            };

                            if let Some(range) = range {
                                let l_txt = TEXT.laidout();
                                let r_txt = r_txt.segmented_text.text();

                                let mut bounds = PxBox::new(PxPoint::splat(Px::MAX), PxPoint::splat(Px::MIN));
                                for line_rect in l_txt.shaped_text.highlight_rects(range, r_txt) {
                                    let line_box = line_rect.to_box2d();
                                    bounds.min = bounds.min.min(line_box.min);
                                    bounds.max = bounds.max.max(line_box.max);
                                }
                                let selection_bounds = bounds.to_rect();

                                *final_size = c.layout(wl);

                                let offset = SELECTION_TOOLBAR_ANCHOR_VAR.get();

                                fn layout_offset(size: PxSize, point: Point) -> PxVector {
                                    LAYOUT
                                        .with_constraints(PxConstraints2d::new_exact_size(size), || point.layout())
                                        .to_vector()
                                }
                                let place = layout_offset(selection_bounds.size, offset.place);
                                let origin = layout_offset(*final_size, offset.origin);

                                translate = selection_bounds.origin.to_vector() + place - origin;
                            } else {
                                // no selection, must be closing
                                wl.collapse();
                                *final_size = PxSize::zero();
                            };
                        }
                        UiNodeOp::Render { frame } => {
                            let l_txt = TEXT.laidout();
                            let transform = l_txt.render_info.transform.then_translate(translate.cast());
                            let transform = adjust_viewport_bound(transform, c);

                            frame.push_reference_frame(transform_key.into(), FrameValue::Value(transform), true, false, |frame| {
                                c.render(frame)
                            });
                        }
                        UiNodeOp::RenderUpdate { update } => {
                            let l_txt = TEXT.laidout();
                            let transform = l_txt.render_info.transform.then_translate(translate.cast());
                            let transform = adjust_viewport_bound(transform, c);

                            update.with_transform(transform_key.update(transform, true), false, |update| c.render_update(update));
                        }
                        _ => {}
                    });

                    // capture all context including LaidoutText, exclude text style properties.
                    let capture = ContextCapture::CaptureBlend {
                        filter: CaptureFilter::Exclude({
                            let mut exclude = ContextValueSet::new();
                            super::Text::context_vars_set(&mut exclude);

                            let mut allow = ContextValueSet::new();
                            super::LangMix::<()>::context_vars_set(&mut allow);
                            exclude.remove_all(&allow);
                            exclude.remove(&SELECTION_TOOLBAR_ANCHOR_VAR);

                            exclude
                        }),
                        over: false,
                    };

                    let mut anchor_mode = AnchorMode::tooltip();
                    anchor_mode.transform = AnchorTransform::None;
                    let state = POPUP.open_config(node, anchor_mode, capture);
                    state.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                    popup_state = Some(state);
                    drop(r_txt);
                    TEXT.resolve().selection_toolbar_is_open = true;
                    WIDGET.layout().render();
                }
            };
        }
    })
}
fn adjust_viewport_bound(transform: PxTransform, widget: &mut impl UiNode) -> PxTransform {
    let window_bounds = WINDOW.vars().actual_size_px().get();
    let wgt_bounds = PxBox::from(
        widget
            .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().outer_size())
            .unwrap_or_else(PxSize::zero),
    );
    let wgt_bounds = transform.outer_transformed(wgt_bounds).unwrap_or_default();

    let x_underflow = -wgt_bounds.min.x.min(Px(0));
    let x_overflow = (wgt_bounds.max.x - window_bounds.width).max(Px(0));
    let y_underflow = -wgt_bounds.min.y.min(Px(0));
    let y_overflow = (wgt_bounds.max.y - window_bounds.height).max(Px(0));

    let x = x_underflow - x_overflow;
    let y = y_underflow - y_overflow;

    let correction = PxVector::new(x, y);

    transform.then_translate(correction.cast())
}
