use std::sync::Arc;

use parking_lot::RwLock;
use zng_app::widget::node::Z_INDEX;
use zng_ext_font::CaretIndex;
use zng_ext_input::focus::{FOCUS, FOCUS_CHANGED_EVENT};
use zng_ext_window::WINDOWS;
use zng_wgt::prelude::*;

use crate::{
    RICH_TEXT_FOCUSED_Z_VAR,
    cmd::{SELECT_CMD, TextSelectOp},
};

use super::{RICH_TEXT, RICH_TEXT_NOTIFY, RichCaretInfo, RichText, TEXT};

pub(crate) fn rich_text_node(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    let child = rich_text_component(child, "rich_text");

    let mut ctx = None;
    let mut dispatch = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&enabled);
            if enabled.get() && TEXT.try_rich().is_none() {
                ctx = Some(Arc::new(RwLock::new(RichText {
                    root_id: WIDGET.id(),
                    caret: RichCaretInfo {
                        index: None,
                        selection_index: None,
                    },
                })));
                dispatch = Some(Arc::new(RwLock::new(vec![])));

                RICH_TEXT.with_context(&mut ctx, || child.init());
            }
        }
        UiNodeOp::Event { update } => {
            if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || {
                    RICH_TEXT_NOTIFY.with_context(&mut dispatch, || {
                        child.event(update);

                        let mut requests = std::mem::take(&mut *RICH_TEXT_NOTIFY.write());
                        let mut tree = None;
                        while !requests.is_empty() {
                            for mut update in requests.drain(..) {
                                if update.delivery_list_mut().has_pending_search() {
                                    if tree.is_none() {
                                        tree = Some(WINDOW.info());
                                    }
                                    update.delivery_list_mut().fulfill_search(tree.iter());
                                }
                                child.event(&update);
                            }
                            requests.extend(RICH_TEXT_NOTIFY.write().drain(..));
                        }
                    });
                });
            }
        }
        UiNodeOp::Update { updates } => {
            if enabled.is_new() {
                WIDGET.reinit();
            } else if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || child.update(updates));
            }
        }
        UiNodeOp::Deinit => {
            if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || child.deinit());
                ctx = None;
                dispatch = None;
            }
        }
        op => {
            if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || child.op(op));
            }
        }
    })
}

/// An UI node that implements some behavior for rich text composition.
///
/// This node is intrinsic to the `Text!` widget and is part of the `rich_text` property. Note that the
/// actual rich text editing is implemented by the `resolve_text` and `layout_text` nodes that are intrinsic to `Text!`.
///
/// The `kind` identifies what kind of component, the value `"rich_text"` is used by the `rich_text` property, the value `"text"`
/// is used by the `Text!` widget, any other value defines a [`RichTextComponent::Leaf`] that is expected to be focusable, inlined
/// and able to handle rich text composition requests.
pub fn rich_text_component(child: impl UiNode, kind: &'static str) -> impl UiNode {
    let mut focus_within = false;
    let mut prev_index = ZIndex::DEFAULT;
    let mut index_update = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            c.init();

            if TEXT.try_rich().is_some() {
                WIDGET.sub_event(&FOCUS_CHANGED_EVENT).sub_var(&RICH_TEXT_FOCUSED_Z_VAR);
                prev_index = Z_INDEX.get();
            }
        }
        UiNodeOp::Deinit => {
            focus_within = false;
        }
        UiNodeOp::Info { info } => {
            if let Some(r) = TEXT.try_rich() {
                let c = match kind {
                    "rich_text" => {
                        if r.root_id == WIDGET.id() {
                            RichTextComponent::Root
                        } else {
                            RichTextComponent::Branch
                        }
                    }
                    kind => RichTextComponent::Leaf { kind },
                };
                info.set_meta(*RICH_TEXT_COMPONENT_ID, c);
            }
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                let new_is_focus_within = args.is_focus_within(WIDGET.id());
                if focus_within != new_is_focus_within {
                    focus_within = new_is_focus_within;

                    if TEXT.try_rich().is_some() {
                        index_update = Some(focus_within);
                        WIDGET.update(); // Z_INDEX.set only works on update
                    }
                }
            }
        }
        UiNodeOp::Update { updates } => {
            c.update(updates);

            if let Some(apply) = index_update.take() {
                if apply {
                    prev_index = Z_INDEX.get();
                    if let Some(i) = RICH_TEXT_FOCUSED_Z_VAR.get() {
                        Z_INDEX.set(i);
                    }
                } else if RICH_TEXT_FOCUSED_Z_VAR.get().is_some() {
                    Z_INDEX.set(prev_index);
                }
            }
            if let Some(idx) = RICH_TEXT_FOCUSED_Z_VAR.get_new() {
                if focus_within {
                    Z_INDEX.set(idx.unwrap_or(prev_index));
                }
            }
        }
        _ => {}
    })
}

impl RichText {
    /// Get root widget info.
    ///
    /// See also [`RichTextWidgetInfoExt`] to query the
    pub fn root_info(&self) -> Option<WidgetInfo> {
        WINDOWS.widget_info(self.root_id)
    }

    /// Iterate over the text/leaf component descendants that can be interacted with.
    pub fn leaves(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        self.root_info().into_iter().flat_map(|w| rich_text_leaves_static(&w))
    }

    /// Iterate over the text/leaf component descendants that can be interacted with in reverse.
    pub fn leaves_rev(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        self.root_info().into_iter().flat_map(|w| rich_text_leaves_rev_static(&w))
    }

    /// Iterate over all text/leaf components that are part of the selection.
    ///
    /// The return iterator is empty if there is no selection.
    pub fn selection(&self) -> impl ExactSizeIterator<Item = WidgetInfo> + 'static {
        let (root, a, b) = match (self.caret.index, self.caret.selection_index) {
            (Some(a), Some(b)) => (self.root_info(), a, b),
            _ => (None, self.root_id, self.root_id),
        };
        OptKnownLenIter {
            known_len_iter: root.into_iter().flat_map(move |w| rich_text_selection_static(&w, a, b)),
        }
    }

    /// Iterate over all text/leaf components that are part of the selection in reverse.
    ///
    /// The return iterator is empty if there is no selection.
    pub fn selection_rev(&self) -> impl ExactSizeIterator<Item = WidgetInfo> + 'static {
        let (root, a, b) = match (self.caret.index, self.caret.selection_index) {
            (Some(a), Some(b)) => (self.root_info(), a, b),
            _ => (None, self.root_id, self.root_id),
        };
        OptKnownLenIter {
            known_len_iter: root.into_iter().flat_map(move |w| rich_text_selection_rev_static(&w, a, b)),
        }
    }

    /// Gets the `caret.index` widget info if it is set and is a valid leaf.
    pub fn caret_index_info(&self) -> Option<WidgetInfo> {
        self.leaf_info(self.caret.index?)
    }

    /// Gets the `caret.selection_index` widget info if it is set and is a valid leaf.
    pub fn caret_selection_index_info(&self) -> Option<WidgetInfo> {
        self.leaf_info(self.caret.selection_index?)
    }

    /// Gets the `id` widget info if it is a valid leaf in the rich text context.
    pub fn leaf_info(&self, id: WidgetId) -> Option<WidgetInfo> {
        let root = self.root_info()?;
        let wgt = root.tree().get(id)?;
        if !matches!(wgt.rich_text_component(), Some(RichTextComponent::Leaf { .. })) {
            return None;
        }
        if !wgt.is_descendant(&root) {
            return None;
        }
        Some(wgt)
    }
}
impl RichCaretInfo {
    /// Update the rich selection and local selection for each rich component.
    ///
    /// Before calling this you must update the [`CaretInfo::index`] in `new_index` and the [`CaretInfo::selection_index`] in
    /// `new_selection_index`. Alternatively enable `skip_end_points` to handle the local selection at the end point widgets.
    ///
    /// If you don't want focus to be moved to the `new_index` set `skip_focus` to `true`.
    ///
    /// # Panics
    ///
    /// Panics if `new_index` or `new_selection_index` is not inside the same rich text context.
    ///
    /// [`CaretInfo::index`]: crate::node::CaretInfo::index
    /// [`CaretInfo::selection_index`]: crate::node::CaretInfo::selection_index
    pub fn update_selection(
        &mut self,
        new_index: &WidgetInfo,
        new_selection_index: Option<&WidgetInfo>,
        skip_end_points: bool,
        skip_focus: bool,
    ) {
        let root = new_index.rich_text_root().unwrap();
        let old_index = self
            .index
            .and_then(|id| new_index.tree().get(id))
            .unwrap_or_else(|| new_index.clone());
        let old_selection_index = self.selection_index.and_then(|id| new_index.tree().get(id));

        self.index = Some(new_index.id());
        self.selection_index = new_selection_index.map(|w| w.id());

        match (&old_selection_index, new_selection_index) {
            (None, None) => self.continue_focus(skip_focus, new_index, &root),
            (None, Some(new_sel)) => {
                // add selection
                let (a, b) = match new_index.cmp_sibling_in(new_sel, &root).unwrap() {
                    std::cmp::Ordering::Less => (new_index, new_sel),
                    std::cmp::Ordering::Greater => (new_sel, new_index),
                    std::cmp::Ordering::Equal => {
                        // single widget selection, already defined
                        return self.continue_focus(skip_focus, new_index, &root);
                    }
                };
                if !skip_end_points {
                    self.continue_select_lesser(a, a == new_index);
                }
                let middle_op = TextSelectOp::local_select_all();
                for middle in a.rich_text_next().take_while(|n| n != b) {
                    notify_leaf_select_op(middle.id(), middle_op.clone());
                }
                if !skip_end_points {
                    self.continue_select_greater(b, b == new_index);
                }

                self.continue_focus(skip_focus, new_index, &root);
            }
            (Some(old_sel), None) => {
                // remove selection
                let (a, b) = match old_index.cmp_sibling_in(old_sel, &root).unwrap() {
                    std::cmp::Ordering::Less => (&old_index, old_sel),
                    std::cmp::Ordering::Greater => (old_sel, &old_index),
                    std::cmp::Ordering::Equal => {
                        // was single widget selection
                        if !skip_end_points {
                            notify_leaf_select_op(old_sel.id(), TextSelectOp::local_clear_selection());
                        }
                        return self.continue_focus(skip_focus, new_index, &root);
                    }
                };
                let op = TextSelectOp::local_clear_selection();
                if !skip_end_points {
                    notify_leaf_select_op(a.id(), op.clone());
                }
                for middle in a.rich_text_next().take_while(|n| n != b) {
                    notify_leaf_select_op(middle.id(), op.clone());
                }
                if !skip_end_points {
                    notify_leaf_select_op(b.id(), op);
                }

                self.continue_focus(skip_focus, new_index, &root);
            }
            (Some(old_sel), Some(new_sel)) => {
                // update selection

                let (old_a, old_b) = match old_index.cmp_sibling_in(old_sel, &root).unwrap() {
                    std::cmp::Ordering::Less | std::cmp::Ordering::Equal => (&old_index, old_sel),
                    std::cmp::Ordering::Greater => (old_sel, &old_index),
                };
                let (new_a, new_b) = match new_index.cmp_sibling_in(new_sel, &root).unwrap() {
                    std::cmp::Ordering::Less | std::cmp::Ordering::Equal => (new_index, new_sel),
                    std::cmp::Ordering::Greater => (new_sel, new_index),
                };

                let min_a = match old_a.cmp_sibling_in(new_a, &root).unwrap() {
                    std::cmp::Ordering::Less | std::cmp::Ordering::Equal => old_a,
                    std::cmp::Ordering::Greater => new_a,
                };
                let max_b = match old_b.cmp_sibling_in(new_b, &root).unwrap() {
                    std::cmp::Ordering::Less => new_b,
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => old_b,
                };

                fn inclusive_range_contains(a: &WidgetInfo, b: &WidgetInfo, q: &WidgetInfo, root: &WidgetInfo) -> bool {
                    match a.cmp_sibling_in(q, root).unwrap() {
                        // a < q
                        std::cmp::Ordering::Less => match b.cmp_sibling_in(q, root).unwrap() {
                            // b < q
                            std::cmp::Ordering::Less => false,
                            // b >= q
                            std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => true,
                        },
                        // a == q
                        std::cmp::Ordering::Equal => true,
                        // a > q
                        std::cmp::Ordering::Greater => false,
                    }
                }

                // println!("(old_a, old_b) = ({}, {})", old_a.id(), old_b.id());
                // println!("(new_a, new_b) = ({}, {})", new_a.id(), new_b.id());
                // println!("(min_a, max_b) = ({}, {})", min_a.id(), max_b.id());

                for wgt in min_a.rich_text_self_and_next() {
                    if &wgt == new_a {
                        if !skip_end_points && new_a != new_b {
                            self.continue_select_lesser(new_a, new_a == new_index);
                        }
                    } else if &wgt == new_b {
                        if !skip_end_points && new_a != new_b {
                            self.continue_select_greater(new_b, new_b == new_index);
                        }
                    } else {
                        let is_old = inclusive_range_contains(old_a, old_b, &wgt, &root);
                        let is_new = inclusive_range_contains(new_a, new_b, &wgt, &root);

                        match (is_old, is_new) {
                            (true, true) => {
                                if &wgt == old_a || &wgt == old_b {
                                    // was endpoint now is full selection
                                    notify_leaf_select_op(wgt.id(), TextSelectOp::local_select_all())
                                }
                            }
                            (true, false) => {
                                notify_leaf_select_op(wgt.id(), TextSelectOp::local_clear_selection());
                            }
                            (false, true) => notify_leaf_select_op(wgt.id(), TextSelectOp::local_select_all()),
                            (false, false) => {}
                        }
                    }

                    if &wgt == max_b {
                        break;
                    }
                }

                self.continue_focus(skip_focus, new_index, &root);
            }
        }
    }
    fn continue_select_lesser(&self, a: &WidgetInfo, a_is_caret: bool) {
        notify_leaf_select_op(
            a.id(),
            TextSelectOp::new(move || {
                let len = TEXT.resolved().segmented_text.text().len();
                let len = CaretIndex { index: len, line: 0 }; // line is updated next layout
                let mut ctx = TEXT.resolve_caret();
                if a_is_caret {
                    ctx.selection_index = Some(len);
                } else {
                    ctx.index = Some(len);
                }
                ctx.index_version += 1;
            }),
        );
    }
    fn continue_select_greater(&self, b: &WidgetInfo, b_is_caret: bool) {
        notify_leaf_select_op(
            b.id(),
            TextSelectOp::new(move || {
                let mut ctx = TEXT.resolve_caret();
                if b_is_caret {
                    ctx.selection_index = Some(CaretIndex::ZERO);
                } else {
                    ctx.index = Some(CaretIndex::ZERO);
                }
                ctx.index_version += 1;
            }),
        );
    }
    fn continue_focus(&self, skip_focus: bool, new_index: &WidgetInfo, root: &WidgetInfo) {
        if !skip_focus && FOCUS.is_focus_within(root.id()).get() {
            FOCUS.focus_widget(new_index.id(), false);
        }
    }
}

pub(crate) fn notify_leaf_select_op(leaf_id: WidgetId, op: TextSelectOp) {
    RICH_TEXT_NOTIFY.write().push(SELECT_CMD.scoped(leaf_id).new_update_param(op));
}

/// Extends [`WidgetInfo`] state to provide information about rich text.
pub trait RichTextWidgetInfoExt {
    /// Gets the outer most ancestor that defines the rich text root.
    fn rich_text_root(&self) -> Option<WidgetInfo>;

    /// Gets what kind of component of the rich text tree this widget is.
    fn rich_text_component(&self) -> Option<RichTextComponent>;

    /// Iterate over the text/leaf component descendants that can be interacted with.
    fn rich_text_leaves(&self) -> impl Iterator<Item = WidgetInfo> + 'static;
    /// Iterate over the text/leaf component descendants that can be interacted with, in reverse.
    fn rich_text_leaves_rev(&self) -> impl Iterator<Item = WidgetInfo> + 'static;

    /// Iterate over the selection text/leaf component descendants that can be interacted with.
    ///
    /// The iterator is over `a..=b` or if `a` is after `b` the iterator is over `b..=a`.
    fn rich_text_selection(&self, a: WidgetId, b: WidgetId) -> impl ExactSizeIterator<Item = WidgetInfo> + 'static;
    /// Iterate over the selection text/leaf component descendants that can be interacted with, in reverse.
    ///
    /// The iterator is over `b..=a` or if `a` is after `b` the iterator is over `a..=b`.
    fn rich_text_selection_rev(&self, a: WidgetId, b: WidgetId) -> impl ExactSizeIterator<Item = WidgetInfo> + 'static;

    /// Iterate over the prev text/leaf components before the current one.
    fn rich_text_prev(&self) -> impl Iterator<Item = WidgetInfo> + 'static;
    /// Iterate over the text/leaf component and previous.
    fn rich_text_self_and_prev(&self) -> impl Iterator<Item = WidgetInfo> + 'static;
    /// Iterate over the next text/leaf components after the current one.
    fn rich_text_next(&self) -> impl Iterator<Item = WidgetInfo> + 'static;
    /// Iterate over the text/leaf component and next.
    fn rich_text_self_and_next(&self) -> impl Iterator<Item = WidgetInfo> + 'static;

    /// Gets info about how this rich leaf affects the text lines.
    fn rich_text_line_info(&self) -> RichLineInfo;

    /// Gets the leaf descendant that is nearest to the `window_point`.
    fn rich_text_nearest_leaf(&self, window_point: PxPoint) -> Option<WidgetInfo>;
    /// Gets the leaf descendant that is nearest to the `window_point` and is approved by the filter.
    ///
    /// The filter parameters are the widget, the rect, the rect row index and the widget inline rows length. If the widget is not inlined
    /// both index and len are zero.
    fn rich_text_nearest_leaf_filtered(
        &self,
        window_point: PxPoint,
        filter: impl FnMut(&WidgetInfo, PxRect, usize, usize) -> bool,
    ) -> Option<WidgetInfo>;
}
impl RichTextWidgetInfoExt for WidgetInfo {
    fn rich_text_root(&self) -> Option<WidgetInfo> {
        self.self_and_ancestors()
            .find(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Root)))
    }

    fn rich_text_component(&self) -> Option<RichTextComponent> {
        self.meta().copy(*RICH_TEXT_COMPONENT_ID)
    }

    fn rich_text_leaves(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        rich_text_leaves_static(self)
    }
    fn rich_text_leaves_rev(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        rich_text_leaves_rev_static(self)
    }

    fn rich_text_selection(&self, a: WidgetId, b: WidgetId) -> impl ExactSizeIterator<Item = WidgetInfo> + 'static {
        rich_text_selection_static(self, a, b)
    }
    fn rich_text_selection_rev(&self, a: WidgetId, b: WidgetId) -> impl ExactSizeIterator<Item = WidgetInfo> + 'static {
        rich_text_selection_rev_static(self, a, b)
    }

    fn rich_text_prev(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        let me = self.clone();
        self.rich_text_root()
            .into_iter()
            .flat_map(move |w| me.prev_siblings_in(&w))
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
    }

    fn rich_text_next(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        let me = self.clone();
        self.rich_text_root()
            .into_iter()
            .flat_map(move |w| me.next_siblings_in(&w))
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
    }

    fn rich_text_self_and_prev(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        let me = self.clone();
        self.rich_text_root()
            .into_iter()
            .flat_map(move |w| me.self_and_prev_siblings_in(&w))
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
    }

    fn rich_text_self_and_next(&self) -> impl Iterator<Item = WidgetInfo> + 'static {
        let me = self.clone();
        self.rich_text_root()
            .into_iter()
            .flat_map(move |w| me.self_and_next_siblings_in(&w))
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
    }

    fn rich_text_line_info(&self) -> RichLineInfo {
        let (prev_min, prev_max) = match self.rich_text_prev().next() {
            Some(p) => {
                let bounds = p.bounds_info();
                let inner_bounds = bounds.inner_bounds();
                if let Some(inline) = bounds.inline() {
                    let mut last = inline.rows[inline.rows.len() - 1];
                    last.origin += inner_bounds.origin.to_vector();
                    (last.min_y(), last.max_y())
                } else {
                    (inner_bounds.min_y(), inner_bounds.max_y())
                }
            }
            None => (Px::MIN, Px::MIN),
        };

        let bounds = self.bounds_info();
        let inner_bounds = bounds.inner_bounds();
        let (min, max, wraps) = if let Some(inline) = bounds.inline() {
            let mut first = inline.rows[0];
            first.origin += inner_bounds.origin.to_vector();
            (first.min_y(), first.max_y(), inline.rows.len() > 1)
        } else {
            (inner_bounds.min_y(), inner_bounds.max_y(), false)
        };

        let starts = !lines_overlap_strict(prev_min, prev_max, min, max);

        RichLineInfo {
            starts_new_line: starts,
            ends_in_new_line: wraps,
        }
    }

    fn rich_text_nearest_leaf(&self, window_point: PxPoint) -> Option<WidgetInfo> {
        self.rich_text_nearest_leaf_filtered(window_point, |_, _, _, _| true)
    }
    fn rich_text_nearest_leaf_filtered(
        &self,
        window_point: PxPoint,
        mut filter: impl FnMut(&WidgetInfo, PxRect, usize, usize) -> bool,
    ) -> Option<WidgetInfo> {
        let root_size = self.inner_border_size();
        let search_radius = root_size.width.max(root_size.height);

        self.nearest_rect_filtered(window_point, search_radius, |w, rect, i, len| {
            matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })) && filter(w, rect, i, len)
        })
    }
}
fn lines_overlap_strict(y_min1: Px, y_max1: Px, y_min2: Px, y_max2: Px) -> bool {
    let (a_min, a_max) = if y_min1 <= y_max1 { (y_min1, y_max1) } else { (y_max1, y_min1) };
    let (b_min, b_max) = if y_min2 <= y_max2 { (y_min2, y_max2) } else { (y_max2, y_min2) };

    a_min < b_max && b_min < a_max
}

/// Info about how a rich text leaf defines new lines in a rich text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RichLineInfo {
    /// Leaf widget first line height span does not intersect the previous sibling last line height span vertically.
    ///
    /// This heuristic allow multiple *baselines* in the same row (sub/superscript), it also allows bidi mixed segments that
    /// maybe have negative horizontal offsets, but very custom layouts such as a diagonal stack panel may want to provide
    /// their own definition of a *line* as an alternative to this API.
    pub starts_new_line: bool,
    /// Leaf widget inline layout declared multiple lines so the end is in a new line.
    ///
    /// Note that the widget may define multiple other lines inside itself, those don't count as "rich text lines".
    pub ends_in_new_line: bool,
}

// implemented here because there is a borrow checker limitation with `+'static`
// that will only be fixed when `+use<>` is allowed in trait methods.
fn rich_text_leaves_static(wgt: &WidgetInfo) -> impl Iterator<Item = WidgetInfo> + use<> {
    wgt.descendants()
        .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
}
fn rich_text_leaves_rev_static(wgt: &WidgetInfo) -> impl Iterator<Item = WidgetInfo> + use<> {
    wgt.descendants()
        .tree_rev()
        .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
}
fn rich_text_selection_static(wgt: &WidgetInfo, a: WidgetId, b: WidgetId) -> impl ExactSizeIterator<Item = WidgetInfo> + use<> {
    let mut ai = usize::MAX;
    let mut bi = usize::MAX;

    for (i, leaf) in wgt.rich_text_leaves().enumerate() {
        let id = leaf.id();
        if id == a {
            ai = i;
        }
        if id == b {
            bi = i;
        }
        if ai != usize::MAX && bi != usize::MAX {
            break;
        }
    }

    if ai > bi {
        std::mem::swap(&mut ai, &mut bi);
    }

    let (skip, take) = if ai != usize::MAX && bi != usize::MAX {
        (ai, bi - ai + 1)
    } else {
        (0, 0)
    };

    KnownLenIter {
        take: rich_text_leaves_static(wgt).skip(skip).take(take),
        len: take,
    }
}
fn rich_text_selection_rev_static(wgt: &WidgetInfo, a: WidgetId, b: WidgetId) -> impl ExactSizeIterator<Item = WidgetInfo> + use<> {
    let mut ai = usize::MAX;
    let mut bi = usize::MAX;

    for (i, leaf) in wgt.rich_text_leaves_rev().enumerate() {
        let id = leaf.id();
        if id == a {
            ai = i;
        } else if id == b {
            bi = i;
        }
        if ai != usize::MAX && bi != usize::MAX {
            break;
        }
    }

    if ai > bi {
        std::mem::swap(&mut ai, &mut bi);
    }

    let (skip, take) = if ai != usize::MAX && bi != usize::MAX {
        (ai, bi - ai + 1)
    } else {
        (0, 0)
    };

    KnownLenIter {
        take: rich_text_leaves_rev_static(wgt).skip(skip).take(take),
        len: take,
    }
}

/// Represents what kind of component the widget is in a rich text tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RichTextComponent {
    /// Outermost widget that is `rich_text` enabled.
    Root,
    /// Widget is `rich_text` enabled, but is inside another rich text tree.
    Branch,
    /// Widget is a text or custom component of the rich text.
    Leaf {
        /// Arbitrary identifier.
        ///
        /// Is `"text"` for `Text!` widgets.
        kind: &'static str,
    },
}

static_id! {
    static ref RICH_TEXT_COMPONENT_ID: StateId<RichTextComponent>;
}

struct KnownLenIter<I> {
    take: I,
    len: usize,
}
impl<I: Iterator<Item = WidgetInfo>> Iterator for KnownLenIter<I> {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        match self.take.next() {
            Some(r) => {
                self.len -= 1;
                Some(r)
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}
impl<I: Iterator<Item = WidgetInfo>> ExactSizeIterator for KnownLenIter<I> {}

struct OptKnownLenIter<I> {
    known_len_iter: I,
}
impl<I: Iterator<Item = WidgetInfo>> Iterator for OptKnownLenIter<I> {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.known_len_iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // is either 0 from `None` or known len from `Some(KnownLenIter)`
        self.known_len_iter.size_hint()
    }
}
impl<I: Iterator<Item = WidgetInfo>> ExactSizeIterator for OptKnownLenIter<I> {}
