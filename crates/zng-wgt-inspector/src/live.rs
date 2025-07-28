#![cfg(feature = "live")]

use zng_app::access::ACCESS_CLICK_EVENT;
use zng_ext_config::CONFIG;
use zng_ext_input::{
    gesture::CLICK_EVENT,
    mouse::{MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT},
    touch::{TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT, TOUCHED_EVENT},
};
use zng_ext_window::{WINDOW_Ext as _, WINDOWS};
use zng_view_api::window::CursorIcon;
use zng_wgt::prelude::*;
use zng_wgt_input::CursorSource;

use crate::INSPECT_CMD;

pub mod data_model;
mod inspector_window;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Config {
    adorn_selected: bool,
    select_focused: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            adorn_selected: true,
            select_focused: false,
        }
    }
}

/// Node set on the window to inspect.
pub fn inspect_node(can_inspect: impl IntoVar<bool>) -> impl UiNode {
    let mut inspected_tree = None::<data_model::InspectedTree>;
    let inspector = WindowId::new_unique();

    let selected_wgt = var(None);
    let hit_select = var(HitSelect::Disabled);

    // persist config, at least across instances of the Inspector.
    let config = CONFIG.get::<Config>(
        if WINDOW.id().name().is_empty() {
            formatx!("window.sequential({}).inspector", WINDOW.id().sequential())
        } else {
            formatx!("window.{}.inspector", WINDOW.id().name())
        },
        Config::default(),
    );
    let adorn_selected = config.map_ref_bidi(|c| &c.adorn_selected, |c| &mut c.adorn_selected);
    let select_focused = config.map_ref_bidi(|c| &c.select_focused, |c| &mut c.select_focused);

    let can_inspect = can_inspect.into_var();
    let mut cmd_handle = CommandHandle::dummy();

    /// Message send to ourselves as an `INSPECT_CMD` param.
    enum InspectorUpdateOnly {
        /// Pump `inspected_tree.update`
        Info,
        /// Pump `inspected_tree.update_render`
        Render,
    }

    let child = match_node_leaf(clmv!(selected_wgt, hit_select, adorn_selected, select_focused, |op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&can_inspect);
            cmd_handle = INSPECT_CMD.scoped(WINDOW.id()).subscribe_wgt(can_inspect.get(), WIDGET.id());
        }
        UiNodeOp::Update { .. } => {
            if let Some(e) = can_inspect.get_new() {
                cmd_handle.set_enabled(e);
            }
        }
        UiNodeOp::Info { .. } => {
            if inspected_tree.is_some() {
                if WINDOWS.is_open(inspector) {
                    INSPECT_CMD.scoped(WINDOW.id()).notify_param(InspectorUpdateOnly::Info);
                } else if !WINDOWS.is_opening(inspector) {
                    inspected_tree = None;
                }
            }
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = INSPECT_CMD.scoped(WINDOW.id()).on_unhandled(update) {
                args.propagation().stop();

                if let Some(u) = args.param::<InspectorUpdateOnly>() {
                    // pump state
                    if let Some(i) = &inspected_tree {
                        match u {
                            InspectorUpdateOnly::Info => i.update(WINDOW.info()),
                            InspectorUpdateOnly::Render => i.update_render(),
                        }
                    }
                } else if let Some(inspected) = inspector_window::inspected() {
                    // can't inspect inspector window, redirect command to inspected
                    INSPECT_CMD.scoped(inspected).notify();
                } else {
                    // focus or open the inspector window
                    let inspected_tree = match &inspected_tree {
                        Some(i) => {
                            i.update(WINDOW.info());
                            i.clone()
                        }
                        None => {
                            let i = data_model::InspectedTree::new(WINDOW.info());
                            inspected_tree = Some(i.clone());
                            i
                        }
                    };

                    let inspected = WINDOW.id();
                    WINDOWS.focus_or_open(
                        inspector,
                        async_clmv!(inspected_tree, selected_wgt, hit_select, adorn_selected, select_focused, {
                            inspector_window::new(inspected, inspected_tree, selected_wgt, hit_select, adorn_selected, select_focused)
                        }),
                    );
                }
            }
        }
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            INSPECT_CMD.scoped(WINDOW.id()).notify_param(InspectorUpdateOnly::Render);
        }
        _ => {}
    }));

    let child = self::adorn_selected(child, selected_wgt, adorn_selected);
    select_on_click(child, hit_select)
}

/// Node in the inspected window, draws adorners around widgets selected on the inspector window.
fn adorn_selected(child: impl UiNode, selected_wgt: Var<Option<data_model::InspectedWidget>>, enabled: Var<bool>) -> impl UiNode {
    use inspector_window::SELECTED_BORDER_VAR;

    let selected_info = selected_wgt.flat_map(|s| {
        if let Some(s) = s {
            s.info().map(|i| Some(i.clone()))
        } else {
            var_local(None)
        }
    });
    let transform_id = SpatialFrameId::new_unique();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&selected_info)
                .sub_var_render(&enabled)
                .sub_var_render(&SELECTED_BORDER_VAR);
        }
        UiNodeOp::Render { frame } => {
            c.render(frame);

            if !enabled.get() {
                return;
            }
            selected_info.with(|w| {
                if let Some(w) = w {
                    let bounds = w.bounds_info();
                    let transform = bounds.inner_transform();
                    let size = bounds.inner_size();

                    frame.push_reference_frame(transform_id.into(), transform.into(), false, false, |frame| {
                        let widths = Dip::new(3).to_px(frame.scale_factor());
                        frame.push_border(
                            PxRect::from_size(size).inflate(widths, widths),
                            PxSideOffsets::new_all_same(widths),
                            SELECTED_BORDER_VAR.get().into(),
                            PxCornerRadius::default(),
                        );
                    });
                }
            });
        }
        _ => {}
    })
}

// node in the inspected window, handles selection on click.
fn select_on_click(child: impl UiNode, hit_select: Var<HitSelect>) -> impl UiNode {
    // when `pending` we need to block interaction with window content, as if a modal
    // overlay was opened, but we can't rebuild info, and we actually want the click target,
    // so we only manually block common pointer events.

    let mut click_handle = EventHandles::dummy();
    let mut _cursor_handle = VarHandle::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&hit_select);
        }
        UiNodeOp::Deinit => {
            _cursor_handle = VarHandle::dummy();
            click_handle.clear();
        }
        UiNodeOp::Update { .. } => {
            if let Some(h) = hit_select.get_new() {
                if matches!(h, HitSelect::Enabled) {
                    let cursor = WINDOW.vars().cursor();

                    // set cursor to Crosshair and lock it in by resetting on a hook.
                    let locked_cur = CursorSource::Icon(CursorIcon::Crosshair);
                    cursor.set(locked_cur.clone());
                    let weak_cursor = cursor.downgrade();
                    _cursor_handle = cursor.hook(move |a| {
                        let icon = a.value();
                        if icon != &locked_cur {
                            let cursor = weak_cursor.upgrade().unwrap();
                            cursor.set(locked_cur.clone());
                        }
                        true
                    });

                    click_handle.push(MOUSE_INPUT_EVENT.subscribe(WIDGET.id()));
                    click_handle.push(TOUCH_INPUT_EVENT.subscribe(WIDGET.id()));
                } else {
                    WINDOW.vars().cursor().set(CursorIcon::Default);
                    _cursor_handle = VarHandle::dummy();

                    click_handle.clear();
                }
            }
        }
        UiNodeOp::Event { update } => {
            if matches!(hit_select.get(), HitSelect::Enabled) {
                let mut select = None;

                if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                    select = Some(args.target.widget_id());
                } else if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = MOUSE_WHEEL_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = CLICK_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = ACCESS_CLICK_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                    select = Some(args.target.widget_id());
                } else if let Some(args) = TOUCHED_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_MOVE_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_TAP_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_TRANSFORM_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                } else if let Some(args) = TOUCH_LONG_PRESS_EVENT.on(update) {
                    args.propagation().stop();
                    c.delegated();
                }

                if let Some(id) = select {
                    hit_select.set(HitSelect::Select(id));
                }
            }
        }
        _ => {}
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HitSelect {
    Disabled,
    Enabled,
    Select(WidgetId),
}
