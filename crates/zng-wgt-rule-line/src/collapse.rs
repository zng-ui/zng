use std::sync::Arc;

use zng_app::{
    update::LayoutUpdates,
    widget::info::{TreeFilter, iter::TreeIterator},
};
use zng_wgt::prelude::*;

/// Collapse adjacent descendant rule lines.
///
/// Set this in a panel widget to automatically collapse rule lines that would appear repeated or dangling on screen.
#[property(LAYOUT - 100)]
pub fn collapse_scope(child: impl IntoUiNode, mode: impl IntoVar<CollapseMode>) -> UiNode {
    let mode = mode.into_var();
    let mut scope: Option<Arc<CollapseScope>> = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&mode);
            scope = Some(Arc::new(CollapseScope::new(WIDGET.id())));
        }
        UiNodeOp::Deinit => {
            scope = None;
        }
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = SCOPE.with_context(&mut scope, || c.measure(wm));
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = SCOPE.with_context(&mut scope, || c.layout(wl));

            // update collapsed list
            let mode = mode.get();

            // try to reuse the current scope
            let maybe_exclusive = Arc::get_mut(scope.as_mut().unwrap());
            // if not possible alloc new (this can happen if a child captured context and is keeping it)
            let is_new = maybe_exclusive.is_none();
            let mut new = CollapseScope::new(WIDGET.id());
            let s = maybe_exclusive.unwrap_or(&mut new);

            // tracks changes in reused set, ignore this if is_new
            let mut changes = UpdateDeliveryList::new_any();
            if mode.is_empty() {
                if !s.collapse.is_empty() {
                    let info = WIDGET.info();
                    let info = info.tree();
                    for id in s.collapse.drain() {
                        if let Some(wgt) = info.get(id) {
                            changes.insert_wgt(&wgt);
                        }
                    }
                }
            } else {
                // does one pass of the info tree descendants collecting collapsable lines,
                // it is a bit complex to avoid allocating the `IdMap` for most widgets,
                // flags `changed` if a second layout pass is needed

                let info = WIDGET.info();
                macro_rules! filter {
                    ($iter:expr) => {
                        $iter.tree_filter(|w| {
                            if w.meta().flagged(*COLLAPSE_SKIP_ID) {
                                TreeFilter::SkipAll
                            } else {
                                TreeFilter::Include
                            }
                        })
                    };
                }

                let mut trim_start = mode.contains(CollapseMode::TRIM_START);
                let mut trim_end_id = None;
                if mode.contains(CollapseMode::TRIM_END) {
                    // find trim_end start *i* first, so that we can update `s.collapse` in a single pass
                    for wgt in filter!(info.descendants().tree_rev()) {
                        if wgt.meta().flagged(*COLLAPSABLE_LINE_ID) {
                            trim_end_id = Some(wgt.id());
                        } else if wgt.descendants_len() == 0 && !wgt.bounds_info().inner_size().is_empty() {
                            // only consider leafs that are not collapsed
                            break;
                        }
                    }
                }
                let mut trim_end = false;
                let mut merge = false;
                for wgt in filter!(info.descendants()) {
                    if wgt.meta().flagged(*COLLAPSABLE_LINE_ID) {
                        if let Some(id) = trim_end_id
                            && id == wgt.id()
                        {
                            trim_end_id = None;
                            trim_end = true;
                        }
                        let changed = if trim_start || merge || trim_end {
                            s.collapse.insert(wgt.id())
                        } else {
                            merge = mode.contains(CollapseMode::MERGE);
                            s.collapse.remove(&wgt.id())
                        };
                        if changed && !is_new {
                            changes.insert_wgt(&wgt);
                        }
                    } else if wgt.descendants_len() == 0 && !wgt.bounds_info().inner_size().is_empty() {
                        // only consider leafs that are not collapsed
                        trim_start = false;
                        merge = false;
                    }
                }
            }
            if is_new {
                let s = scope.as_mut().unwrap();
                // previous changed state set assuming it was reusing set, override it
                let info = WIDGET.info();
                let info = info.tree();
                for id in s.collapse.symmetric_difference(&new.collapse) {
                    if let Some(wgt) = info.get(*id) {
                        changes.insert_wgt(&wgt);
                    }
                }
                if !changes.widgets().is_empty() {
                    scope = Some(Arc::new(new));
                }
            }

            if !changes.widgets().is_empty() {
                *final_size = wl.with_layout_updates(Arc::new(LayoutUpdates::new(changes)), |wl| {
                    SCOPE.with_context(&mut scope, || c.layout(wl))
                });
            }
        }
        _ => {}
    })
}

/// Defines if this widget and descendants are ignored by [`collapse_scope`].
///
/// If `true` the widget subtree is skipped, as if not present.
///
/// [`collapse_scope`]: fn@collapse_scope
#[property(CONTEXT, default(false))]
pub fn collapse_skip(child: impl IntoUiNode, skip: impl IntoVar<bool>) -> UiNode {
    let skip = skip.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&skip);
        }
        UiNodeOp::Info { info } => {
            if skip.get() {
                info.flag_meta(*COLLAPSE_SKIP_ID);
            }
        }
        _ => {}
    })
}

bitflags::bitflags! {
    /// Represents what rule lines are collapsed in a [`collapse_scope`].
    ///
    /// [`collapse_scope`]: fn@collapse_scope
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub struct CollapseMode: u8 {
        /// Collapse first line(s) if it has no previous visible sibling in the scope.
        const TRIM_START = 0b0000_0001;
        /// Collapse the last line(s) if it has no next visible sibling in the scope.
        const TRIM_END = 0b0000_0010;
        /// Collapse start and end.
        const TRIM = Self::TRIM_START.bits() | Self::TRIM_END.bits();

        /// Adjacent lines without intermediary visible siblings are collapsed except the first in sequence.
        const MERGE = 0b0001_0000;
    }
}
impl_from_and_into_var! {
    fn from(all: bool) -> CollapseMode {
        if all { CollapseMode::all() } else { CollapseMode::empty() }
    }
}

/// Contextual service managed by [`collapse_scope`].
///
/// Custom line widgets not derived from [`RuleLine!`] can participate in [`collapse_scope`] by setting [`COLLAPSABLE_LINE_ID`]
/// during info build and using this service during measure and layout.
///
/// [`collapse_scope`]: fn@collapse_scope
/// [`RuleLine!`]: struct@crate::RuleLine
#[allow(non_camel_case_types)]
pub struct COLLAPSE_SCOPE;
impl COLLAPSE_SCOPE {
    /// Get the parent scope ID.
    pub fn scope_id(&self) -> Option<WidgetId> {
        SCOPE.get().scope_id
    }

    ///Gets if the line widget needs to collapse
    pub fn collapse(&self, line_id: WidgetId) -> bool {
        let scope = SCOPE.get();
        scope.collapse.contains(&line_id)
    }
}

static_id! {
    /// Identifies a line widget that can be collapsed by [`collapse_scope`].
    ///
    /// [`collapse_scope`]: fn@collapse_scope
    pub static ref COLLAPSABLE_LINE_ID: StateId<()>;

    /// Identifies a widget (and descendants) to be ignored by the [`collapse_scope`].
    ///
    /// [`collapse_scope`]: fn@collapse_scope
    pub static ref COLLAPSE_SKIP_ID: StateId<()>;
}

context_local! {
    static SCOPE: CollapseScope = CollapseScope {
        collapse: IdSet::new(),
        scope_id: None,
    };
}

struct CollapseScope {
    collapse: IdSet<WidgetId>,
    scope_id: Option<WidgetId>,
}

impl CollapseScope {
    fn new(scope_id: WidgetId) -> Self {
        Self {
            collapse: IdSet::new(),
            scope_id: Some(scope_id),
        }
    }
}
