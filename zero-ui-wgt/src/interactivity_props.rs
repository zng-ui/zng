use std::sync::Arc;

use task::parking_lot::Mutex;
use zero_ui_app::widget::info;

use crate::prelude::*;

context_var! {
    static IS_ENABLED_VAR: bool = true;
}

/// If default interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`ENABLED`] or [`DISABLED`], to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe the a widget's state use [`interactivity`] value.
///
/// # Interactivity
///
/// Every widget has an [`interactivity`] value, it defines two *tiers* of disabled, the normal disabled blocks the default actions
/// of the widget, but still allows some interactions, such as a different cursor on hover or event an error tool-tip on click, the
/// second tier blocks all interaction with the widget. This property controls the *normal* disabled, to fully block interaction use
/// the [`interactive`] property.
///
/// # Disabled Visual
///
/// Widgets that are interactive should visually indicate when the normal interactions are disabled, you can use the [`is_disabled`]
/// state property in a when block to implement the *visually disabled* appearance of a widget.
///
/// The visual cue for the disabled state is usually a reduced contrast from content and background by *graying-out* the text and applying a
/// grayscale filter for image content. You should also consider adding *disabled interactions* that inform the user when the widget will be
/// enabled.
///
/// # Implicit
///
/// This property is included in all widgets by default, you don't need to import it to use it.
///
/// [`ENABLED`]: zero_ui_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zero_ui_app::widget::info::Interactivity::DISABLED
/// [`interactivity`]: zero_ui_app::widget::info::WidgetInfo::interactivity
/// [`interactive`]: fn@interactive
/// [`is_enabled`]: fn@is_enabled
/// [`is_disabled`]: fn@is_disabled
#[property(CONTEXT, default(true))]
pub fn enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();

    let child = match_node(
        child,
        clmv!(enabled, |_, op| match op {
            UiNodeOp::Init => {
                WIDGET.sub_var_info(&enabled);
            }
            UiNodeOp::Info { info } => {
                if !enabled.get() {
                    info.push_interactivity(Interactivity::DISABLED);
                }
            }
            _ => {}
        }),
    );

    with_context_var(child, IS_ENABLED_VAR, merge_var!(IS_ENABLED_VAR, enabled, |&a, &b| a && b))
}

/// Defines if any interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`BLOCKED`] when `false`, widgets with blocked interactivity do not
/// receive any interaction event and behave like a background visual. To probe the widget state use [`interactivity`] value.
///
/// This property *enables* and *disables* interaction with the widget and its descendants without causing
/// a visual change like [`enabled`], it also blocks "disabled" interactions such as a different cursor or tool-tip for disabled buttons,
/// its use cases are more advanced then [`enabled`], it is mostly used when large parts of the screen are "not ready".
///
/// Note that this affects the widget where it is set and descendants, to disable interaction only in the widgets
/// inside `child` use the [`node::interactive_node`].
///
/// [`enabled`]: fn@enabled
/// [`BLOCKED`]: Interactivity::BLOCKED
/// [`interactivity`]: zero_ui_app::widget::info::WidgetInfo::interactivity
/// [`node::interactive_node`]: crate::node::interactive_node
#[property(CONTEXT, default(true))]
pub fn interactive(child: impl UiNode, interactive: impl IntoVar<bool>) -> impl UiNode {
    let interactive = interactive.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&interactive);
        }
        UiNodeOp::Info { info } => {
            if !interactive.get() {
                info.push_interactivity(Interactivity::BLOCKED);
            }
        }
        _ => {}
    })
}

/// If the widget is enabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
/// [`WidgetInfo::allow_interaction`]: crate::widget_info::WidgetInfo::allow_interaction
#[property(EVENT)]
pub fn is_enabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    vis_enabled_eq_state(child, state, true)
}
/// If the widget is disabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// This is the same as `!self.is_enabled`.
///
/// [`enabled`]: fn@enabled
#[property(EVENT)]
pub fn is_disabled(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    vis_enabled_eq_state(child, state, false)
}

fn vis_enabled_eq_state(child: impl UiNode, state: impl IntoVar<bool>, expected: bool) -> impl UiNode {
    event_is_state(child, state, true, info::INTERACTIVITY_CHANGED_EVENT, move |args| {
        if let Some((_, new)) = args.vis_enabled_change(WIDGET.id()) {
            Some(new.is_vis_enabled() == expected)
        } else {
            None
        }
    })
}

event_property! {
    /// Widget interactivity changed.
    ///
    /// Note that there are multiple specific events for interactivity changes, [`on_enable`], [`on_disable`], [`on_block`] and [`on_unblock`]
    /// are some of then.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// [`on_enable`]: fn@on_enable
    /// [`on_disable`]: fn@on_disable
    /// [`on_block`]: fn@on_block
    /// [`on_unblock`]: fn@on_unblock
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn interactivity_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
    }

    /// Widget was enabled or disabled.
    ///
    /// Note that this event tracks the *actual* enabled status of the widget, not the *visually enabled* status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn enabled_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.enabled_change(WIDGET.id()).is_some(),
    }

    /// Widget changed to enabled or disabled visuals.
    ///
    /// Note that this event tracks the *visual* enabled status of the widget, not the *actual* status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn vis_enabled_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.vis_enabled_change(WIDGET.id()).is_some(),
    }

    /// Widget interactions where blocked or unblocked.
    ///
    /// Note  that blocked widgets may still be visually enabled, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn blocked_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.blocked_change(WIDGET.id()).is_some(),
    }

    /// Widget normal interactions now enabled.
    ///
    /// Note that this event tracks the *actual* enabled status of the widget, not the *visually enabled* status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts enabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_enabled_changed`] for a more general event.
    ///
    /// [`on_enabled_changed`]: fn@on_enabled_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn enable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_enable(WIDGET.id()),
    }

    /// Widget normal interactions now disabled.
    ///
    /// Note that this event tracks the *actual* enabled status of the widget, not the *visually enabled* status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts disabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_enabled_changed`] for a more general event.
    ///
    /// [`on_enabled_changed`]: fn@on_enabled_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn disable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_disable(WIDGET.id()),
    }

    /// Widget now using the enabled visuals.
    ///
    /// Note that this event tracks the *visual* enabled status of the widget, not the *actual* status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts visually enabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_vis_enabled_changed`] for a more general event.
    ///
    /// [`on_vis_enabled_changed`]: fn@on_vis_enabled_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn vis_enable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_vis_enable(WIDGET.id()),
    }

    /// Widget now using the disabled visuals.
    ///
    /// Note that this event tracks the *visual* enabled status of the widget, not the *actual* status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts visually disabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_vis_enabled_changed`] for a more general event.
    ///
    /// [`on_vis_enabled_changed`]: fn@on_vis_enabled_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn vis_disable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_vis_disable(WIDGET.id()),
    }

    /// Widget interactions now blocked.
    ///
    /// Note  that blocked widgets may still be visually enabled, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts blocked,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_blocked_changed`] for a more general event.
    ///
    /// [`on_blocked_changed`]: fn@on_blocked_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn block {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_block(WIDGET.id()),
    }

    /// Widget interactions now unblocked.
    ///
    /// Note that the widget may still be disabled.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts unblocked,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_blocked_changed`] for a more general event.
    ///
    /// [`on_blocked_changed`]: fn@on_blocked_changed
    /// [`Interactivity`]: zero_ui_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn unblock {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_unblock(WIDGET.id()),
    }
}

/// Only allow interaction inside the widget, descendants and ancestors.
///
/// When modal mode is enabled in a widget only it and descendants [allows interaction], all other widgets behave as if disabled, but
/// without the visual indication of disabled. This property is a building block for modal overlay widgets.
///
/// Only one widget can be the modal at a time, if multiple widgets set `modal = true` only the last one by traversal order is modal.
///
/// This property also sets the accessibility modal flag.
///
/// [allows interaction]: zero_ui_app::widget::info::WidgetInfo::interactivity
#[property(CONTEXT, default(false))]
pub fn modal(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    static MODAL_WIDGETS: StaticStateId<Arc<Mutex<ModalWidgetsData>>> = StaticStateId::new_unique();
    #[derive(Default)]
    struct ModalWidgetsData {
        widgets: IdSet<WidgetId>,
        registrar: Option<WidgetId>,

        last_in_tree: Option<WidgetId>,
    }
    let enabled = enabled.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&enabled);
            WINDOW.init_state_default(&MODAL_WIDGETS); // insert window state
        }
        UiNodeOp::Deinit => {
            let mws = WINDOW.req_state(&MODAL_WIDGETS);

            // maybe unregister.
            let mut mws = mws.lock();
            let widget_id = WIDGET.id();
            if mws.widgets.remove(&widget_id) {
                if mws.registrar == Some(widget_id) {
                    // change the existing modal that will re-register on info rebuild.
                    mws.registrar = mws.widgets.iter().next().copied();
                    if let Some(id) = mws.registrar {
                        // ensure that the next registrar is not reused.
                        UPDATES.update_info(id);
                    }
                }

                if mws.last_in_tree == Some(widget_id) {
                    // will re-compute next time the filter is used.
                    mws.last_in_tree = None;
                }
            }
        }
        UiNodeOp::Info { info } => {
            let mws = WINDOW.req_state(&MODAL_WIDGETS);

            if enabled.get() {
                if let Some(mut a) = info.access() {
                    a.flag_modal();
                }

                let insert_filter = {
                    let mut mws = mws.lock();
                    let widget_id = WIDGET.id();
                    if mws.widgets.insert(widget_id) {
                        mws.last_in_tree = None;
                        let r = mws.registrar.is_none();
                        if r {
                            mws.registrar = Some(widget_id);
                        }
                        r
                    } else {
                        mws.registrar == Some(widget_id)
                    }
                };
                if insert_filter {
                    // just registered and we are the first, insert the filter:

                    info.push_interactivity_filter(clmv!(mws, |a| {
                        let mut mws = mws.lock();

                        // caches the top-most modal.
                        if mws.last_in_tree.is_none() {
                            match mws.widgets.len() {
                                0 => unreachable!(),
                                1 => {
                                    // only one modal
                                    mws.last_in_tree = mws.widgets.iter().next().copied();
                                }
                                _ => {
                                    // multiple modals, find the *top* one.
                                    let mut found = 0;
                                    for info in a.info.root().self_and_descendants() {
                                        if mws.widgets.contains(&info.id()) {
                                            mws.last_in_tree = Some(info.id());
                                            found += 1;
                                            if found == mws.widgets.len() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            };
                        }

                        // filter, only allows inside self inclusive, and ancestors.
                        let modal = mws.last_in_tree.unwrap();
                        if a.info.self_and_ancestors().any(|w| w.id() == modal) || a.info.self_and_descendants().any(|w| w.id() == modal) {
                            Interactivity::ENABLED
                        } else {
                            Interactivity::BLOCKED
                        }
                    }));
                }
            } else {
                // maybe unregister.
                let mut mws = mws.lock();
                let widget_id = WIDGET.id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }
        }
        _ => {}
    })
}
