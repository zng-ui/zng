use std::sync::Arc;

use zng_wgt::prelude::*;

/// Collapse adjacent descendant rule lines.
///
/// Set this in a panel widget to automatically collapse rule lines that would appear repeated on screen.
#[property(CONTEXT)]
pub fn collapse_scope(child: impl IntoUiNode, mode: impl IntoVar<CollapseMode>) -> UiNode {
    let mode = mode.into_var();
    let mut scope: Option<Arc<CollapseScope>> = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&mode);
            scope = Some(Arc::new(CollapseScope {
                collapse: IdSet::new(),
                is_default: false,
            }));
        }
        UiNodeOp::Deinit => {
            scope = None;
        }
        UiNodeOp::Info { .. } => {
            match Arc::into_inner(scope.take().unwrap()) {
                Some(mut s) => {
                    s.collapse.clear();
                    scope = Some(Arc::new(s));
                }
                None => {
                    scope = Some(Arc::new(CollapseScope {
                        collapse: IdSet::new(),
                        is_default: false,
                    }))
                }
            }
            WIDGET.layout();
        }
        UiNodeOp::Update { .. } => {
            if mode.is_new() {
                match Arc::into_inner(scope.take().unwrap()) {
                    Some(mut s) => {
                        s.collapse.clear();
                        scope = Some(Arc::new(s));
                    }
                    None => {
                        scope = Some(Arc::new(CollapseScope {
                            collapse: IdSet::new(),
                            is_default: false,
                        }))
                    }
                }
                WIDGET.layout();
            }
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
            let mut new = CollapseScope {
                collapse: IdSet::new(),
                is_default: false,
            };
            let s = maybe_exclusive.unwrap_or(&mut new);

            // tracks changes in reused set, ignore this if is_new
            let mut changed = false;
            if mode.is_empty() {
                changed = !s.collapse.is_empty();
                s.collapse.clear();
            } else {
                // does one pass of the info tree descendants collecting collapsable lines,
                // it is a bit complex to avoid allocating the `IdMap` for most widgets,
                // flags `changed` if a second layout pass is needed

                let info = WIDGET.info();

                let mut trim_start = mode.contains(CollapseMode::TRIM_START);
                let mut trim_end_i = usize::MAX;
                if mode.contains(CollapseMode::TRIM_END) {
                    // find trim_end start *i* first, so that we can update `s.collapse` in a single pass
                    for (i, wgt) in info.descendants().tree_rev().enumerate() {
                        if !wgt.meta().flagged(*COLLAPSABLE_LINE_ID) && !wgt.bounds_info().inner_size().is_empty() {
                            trim_end_i = info.descendants_len() - i - 1;
                            break;
                        }
                    }
                    if trim_end_i == usize::MAX {
                        trim_end_i = 0;
                    }
                }
                let mut merge = false;

                for (i, wgt) in info.descendants().enumerate() {
                    if wgt.meta().flagged(*COLLAPSABLE_LINE_ID) {
                        // collapsable line child
                        if trim_start || merge || i >= trim_end_i {
                            changed |= s.collapse.insert(wgt.id());
                        } else {
                            changed |= s.collapse.remove(&wgt.id());
                        }
                        merge = mode.contains(CollapseMode::MERGE);
                    } else if !wgt.bounds_info().inner_size().is_empty() {
                        // other non-collapsed child
                        trim_start = false;
                        merge = false;
                    }
                }
            }
            if is_new {
                let s = scope.as_mut().unwrap();
                // previous changed state set assuming it was reusing set, override it
                changed = s.collapse != new.collapse;
                if changed {
                    scope = Some(Arc::new(new));
                }
            }

            if changed {
                *final_size = SCOPE.with_context(&mut scope, || c.layout(wl));
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
/// # Panics
///
/// This service is only available during measure and layout, panics if used in other UI methods.
///
/// [`collapse_scope`]: fn@collapse_scope
/// [`RuleLine!`]: struct@crate::RuleLine
#[allow(non_camel_case_types)]
pub struct COLLAPSE_SCOPE;
impl COLLAPSE_SCOPE {
    ///Gets if the line widget needs to collapse
    pub fn collapse(&self, line_id: WidgetId) -> bool {
        let scope = SCOPE.get();
        assert!(!scope.is_default, "COLLAPSE_SCOPE only available in measure and layout");
        scope.collapse.contains(&line_id)
    }
}

static_id! {
    /// Identifies a line widget that can be collapsed by [`collapse_scope`].
    ///
    /// [`collapse_scope`]: fn@collapse_scope
    pub static ref COLLAPSABLE_LINE_ID: StateId<()>;
}

context_local! {
    static SCOPE: CollapseScope = CollapseScope {
        collapse: IdSet::new(),
        is_default: true,
    };
}

struct CollapseScope {
    collapse: IdSet<WidgetId>,
    is_default: bool,
}
