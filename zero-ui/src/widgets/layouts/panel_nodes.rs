//! Helper nodes for implementing panels.

use zero_ui::core::{
    context::{StateId, WIDGET},
    widget_instance::{match_node, PanelListRange, UiNode, UiNodeOp},
};

/// Helper for a property that gets the *index* of the widget in the parent panel.
///
/// See [`with_index_len_node`] for more details.
pub fn with_index_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            // parent PanelList requests updates for this widget every time there is an update.
            let info = WIDGET.info();
            if let Some(parent) = info.parent() {
                if let Some(mut c) = PanelListRange::update(&parent, panel_list_id, &mut version) {
                    let id = info.id();
                    let p = c.position(|w| w.id() == id);
                    update(p);
                }
            }
        }
        _ => {}
    })
}

/// Helper for a property that gets the *index* of the widget in the parent panel.
///
/// See [`with_index_len_node`] for more details.
pub fn with_rev_index_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<usize>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            let info = WIDGET.info();
            if let Some(parent) = info.parent() {
                if let Some(c) = PanelListRange::update(&parent, panel_list_id, &mut version) {
                    let id = info.id();
                    let p = c.rev().position(|w| w.id() == id);
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
/// using the methods in this module. See the [`stack`] getter properties for examples.
///
/// [`stack`]: crate::widgets::layouts::stack
/// [`PanelList::track_info_range`]: crate::core::widget_instance::PanelList::track_info_range
pub fn with_index_len_node(
    child: impl UiNode,
    panel_list_id: impl Into<StateId<PanelListRange>>,
    mut update: impl FnMut(Option<(usize, usize)>) + Send + 'static,
) -> impl UiNode {
    let panel_list_id = panel_list_id.into();
    let mut version = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Deinit => {
            update(None);
            version = None;
        }
        UiNodeOp::Update { .. } => {
            let info = WIDGET.info();
            if let Some(parent) = info.parent() {
                if let Some(mut iter) = PanelListRange::update(&parent, panel_list_id, &mut version) {
                    let id = info.id();
                    let mut p = 0;
                    let mut count = 0;
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

    * How to pump update if the item is reused?
        -
*/
