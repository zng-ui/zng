//! Helper nodes for implementing panels.

use std::mem;

use zero_ui::core::{
    context::{StateId, WIDGET},
    widget_instance::{match_node, PanelListRange, UiNode, UiNodeOp},
};

/// Helper for a property that gets the *index* of the widget in the parent panel.
///
/// See [`get_child_position_and_count_node`] for more details.
pub fn with_index_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut u = true;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            u = false;
        }
        UiNodeOp::Info { .. } => {
            u = true;
            WIDGET.update();
        }
        UiNodeOp::Update { .. } => {
            if mem::take(&mut u) {
                let info = WIDGET.info();
                if let Some(parent) = info.parent() {
                    let id = info.id();
                    let p = PanelListRange::get(parent, panel_list_id).position(|w| w.id() == id);
                    update(p);
                }
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the *index* of the widget in the parent panel.
///
/// See [`get_child_position_and_count_node`] for more details.
pub fn with_rev_index_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut u = true;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            u = false;
        }
        UiNodeOp::Info { .. } => {
            u = true;
            WIDGET.update();
        }
        UiNodeOp::Update { .. } => {
            if mem::take(&mut u) {
                let info = WIDGET.info();
                if let Some(parent) = info.parent() {
                    let id = info.id();
                    let p = PanelListRange::get(parent, panel_list_id).rev().position(|w| w.id() == id);
                    update(p);
                }
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the *index* of the widget in the parent panel and the number of children.
///  
/// Panels must use [`PanelList::track_info_range`] to collect the `panel_list_id`, then implement getter properties
/// using the methods in this module. See the [`crate::widgets::layouts::stack`] getter properties for examples.
pub fn with_index_len_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<(usize, usize)>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut u = true;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            u = true;
        }
        UiNodeOp::Info { .. } => {
            u = true;
            WIDGET.update();
        }
        UiNodeOp::Update { .. } => {
            if mem::take(&mut u) {
                let info = WIDGET.info();
                if let Some(parent) = info.parent() {
                    let id = info.id();
                    let mut p = 0;
                    let mut count = 0;
                    let mut iter = PanelListRange::get(parent, panel_list_id);
                    for c in &mut iter {
                        if c.id() == id {
                            p = count;
                            count += 1 + iter.count();
                            break;
                        } else {
                            count += 1;
                        }
                    }
                    update(Some((p, count)));
                }
            }
        }
        _ => {}
    })
}

/*
 # !!: ISSUES

    * If the parent panel rebuilds info and reuses the widget it will not update.
    * If we issue an update for each child just in case this is the previous perf issue.
    * Maybe we can set a flag on the parent info saying that there is interest, so pump it.

    * Lets rethink this, why not have "pos info actions".
        - Registered/reused like interactivity filters?
        - How is this different from subscribing to the full info tree update?
*/
