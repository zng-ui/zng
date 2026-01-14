use std::sync::Arc;

use task::parking_lot::Mutex;
use zng_app::{
    static_id,
    widget::info::{WIDGET_TREE_CHANGED_EVENT, WidgetTreeChangedArgs},
};

use crate::{
    event_property,
    node::{EventNodeBuilder, VarEventNodeBuilder, bind_state_init},
    prelude::*,
};

context_var! {
    static IS_ENABLED_VAR: bool = true;
}

/// Defines if default interaction is allowed in the widget and its descendants.
///
/// This property sets the interactivity of the widget to [`ENABLED`] or [`DISABLED`], to probe the enabled state in `when` clauses
/// use [`is_enabled`] or [`is_disabled`]. To probe the a widget's info state use [`WidgetInfo::interactivity`] value.
///
/// # Interactivity
///
/// Every widget has an interactivity state, it defines two tiers of disabled, the normal disabled blocks the default actions
/// of the widget, but still allows some interactions, such as a different cursor on hover or event an error tooltip on click, the
/// second tier blocks all interaction with the widget. This property controls the normal disabled, to fully block interaction use
/// the [`interactive`] property.
///
/// # Disabled Visual
///
/// Widgets that are interactive should visually indicate when the normal interactions are disabled, you can use the [`is_disabled`]
/// state property in a when block to implement the visually disabled appearance of a widget.
///
/// The visual cue for the disabled state is usually a reduced contrast from content and background by graying-out the text and applying a
/// grayscale filter for images. Also consider adding disabled interactions, such as a different cursor or a tooltip that explains why the button
/// is disabled.
///
/// [`ENABLED`]: zng_app::widget::info::Interactivity::ENABLED
/// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
/// [`WidgetInfo::interactivity`]: zng_app::widget::info::WidgetInfo::interactivity
/// [`interactive`]: fn@interactive
/// [`is_enabled`]: fn@is_enabled
/// [`is_disabled`]: fn@is_disabled
#[property(CONTEXT, default(true))]
pub fn enabled(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
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
/// receive any interaction event and behave like a background visual. To probe the widget's info state use [`WidgetInfo::interactivity`] value.
///
/// This property *enables* and *disables* interaction with the widget and its descendants without causing
/// a visual change like [`enabled`], it also blocks "disabled" interactions such as a different cursor or tooltip for disabled buttons.
///
/// Note that this affects the widget where it is set and descendants, to disable interaction only in the widgets
/// inside `child` use the [`node::interactive_node`].
///
/// [`enabled`]: fn@enabled
/// [`BLOCKED`]: Interactivity::BLOCKED
/// [`WidgetInfo::interactivity`]: zng_app::widget::info::WidgetInfo::interactivity
/// [`node::interactive_node`]: crate::node::interactive_node
#[property(CONTEXT, default(true))]
pub fn interactive(child: impl IntoUiNode, interactive: impl IntoVar<bool>) -> UiNode {
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

fn is_interactivity_state(child: UiNode, state: Var<bool>, check: fn(Interactivity) -> bool) -> UiNode {
    bind_state_init(child, state, move |state| {
        let win_id = WINDOW.id();
        let wgt_id = WIDGET.id();
        WIDGET_TREE_CHANGED_EVENT.var_latest_bind(state, move |args| {
            if args.tree.window_id() == win_id
                && let Some(wgt) = args.tree.get(wgt_id)
            {
                Some(check(wgt.interactivity()))
            } else {
                None
            }
        })
    })
}

/// If the widget is enabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
#[property(EVENT)]
pub fn is_enabled(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    is_interactivity_state(child.into_node(), state.into_var(), Interactivity::is_vis_enabled)
}
/// If the widget is disabled for interaction.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] property.
///
/// [`enabled`]: fn@enabled
#[property(EVENT)]
pub fn is_disabled(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    fn check(i: Interactivity) -> bool {
        !i.is_vis_enabled()
    }
    is_interactivity_state(child.into_node(), state.into_var(), check)
}

/// Get the widget interactivity value.
///
/// This property is used only for probing the state. You can set the state using
/// the [`enabled`] and [`interactive`] properties.
///
/// [`enabled`]: fn@enabled
/// [`interactive`]: fn@interactive
#[property(EVENT)]
pub fn get_interactivity(child: impl IntoUiNode, state: impl IntoVar<Interactivity>) -> UiNode {
    bind_state_init(child, state, move |state| {
        let win_id = WINDOW.id();
        let wgt_id = WIDGET.id();
        WIDGET_TREE_CHANGED_EVENT.var_latest_bind(state, move |args| {
            if args.tree.window_id() == win_id
                && let Some(wgt) = args.tree.get(wgt_id)
            {
                Some(wgt.interactivity())
            } else {
                None
            }
        })
    })
}

/// Only allow interaction inside the widget, descendants and ancestors.
///
/// When a widget is in modal mode, only it, descendants and ancestors are interactive. If [`modal_includes`]
/// is set on the widget the ancestors and descendants of each include are also allowed.
///
/// Only one widget can be the modal at a time, if multiple widgets set `modal = true` only the last one by traversal order is actually modal.
///
/// This property also sets the accessibility modal flag.
///
/// [`modal_includes`]: fn@modal_includes
#[property(CONTEXT, default(false))]
pub fn modal(child: impl IntoUiNode, enabled: impl IntoVar<bool>) -> UiNode {
    static_id! {
        static ref MODAL_WIDGETS: StateId<Arc<Mutex<ModalWidgetsData>>>;
    }
    #[derive(Default)]
    struct ModalWidgetsData {
        widgets: IdSet<WidgetId>,
        registrar: Option<WidgetId>,

        last_in_tree: Option<WidgetInfo>,
    }
    let enabled = enabled.into_var();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&enabled);
            WINDOW.init_state_default(*MODAL_WIDGETS); // insert window state
        }
        UiNodeOp::Deinit => {
            let mws = WINDOW.req_state(*MODAL_WIDGETS);

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

                if mws.last_in_tree.as_ref().map(WidgetInfo::id) == Some(widget_id) {
                    // will re-compute next time the filter is used.
                    mws.last_in_tree = None;
                }
            }
        }
        UiNodeOp::Info { info } => {
            let mws = WINDOW.req_state(*MODAL_WIDGETS);

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
                                    mws.last_in_tree = a.info.tree().get(*mws.widgets.iter().next().unwrap());
                                    assert!(mws.last_in_tree.is_some());
                                }
                                _ => {
                                    // multiple modals, find the *top* one.
                                    let mut found = 0;
                                    for info in a.info.root().self_and_descendants() {
                                        if mws.widgets.contains(&info.id()) {
                                            mws.last_in_tree = Some(info);
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
                        // modal_includes checks if the id is modal or one of the includes.

                        let modal = mws.last_in_tree.as_ref().unwrap();

                        if a.info
                            .self_and_ancestors()
                            .any(|w| modal.modal_includes(w.id()) || w.modal_included(modal.id()))
                        {
                            // widget ancestor is modal, modal include or includes itself in modal
                            return Interactivity::ENABLED;
                        }
                        if a.info
                            .self_and_descendants()
                            .any(|w| modal.modal_includes(w.id()) || w.modal_included(modal.id()))
                        {
                            // widget or descendant is modal, modal include or includes itself in modal
                            return Interactivity::ENABLED;
                        }
                        Interactivity::BLOCKED
                    }));
                }
            } else {
                // maybe unregister.
                let mut mws = mws.lock();
                let widget_id = WIDGET.id();
                if mws.widgets.remove(&widget_id) && mws.last_in_tree.as_ref().map(|w| w.id()) == Some(widget_id) {
                    mws.last_in_tree = None;
                }
            }
        }
        _ => {}
    })
}

/// Extra widgets that are allowed interaction by this widget when it is [`modal`].
///
/// Note that this is only needed for widgets that are not descendants nor ancestors of this widget, but
/// still need to be interactive when the modal is active.
///
/// See also [`modal_included`] if you prefer setting the modal widget id on the included widget.
///
/// This property calls [`insert_modal_include`] on the widget.
///
/// [`modal`]: fn@modal
/// [`insert_modal_include`]: WidgetInfoBuilderModalExt::insert_modal_include
/// [`modal_included`]: fn@modal_included
#[property(CONTEXT, default(IdSet::new()))]
pub fn modal_includes(child: impl IntoUiNode, includes: impl IntoVar<IdSet<WidgetId>>) -> UiNode {
    let includes = includes.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&includes);
        }
        UiNodeOp::Info { info } => includes.with(|w| {
            for id in w {
                info.insert_modal_include(*id);
            }
        }),
        _ => (),
    })
}

/// Include itself in the allow list of another widget that is [`modal`] or descendant of modal.
///
/// Note that this is only needed for widgets that are not descendants nor ancestors of the modal widget, but
/// still need to be interactive when the modal is active.
///
/// See also [`modal_includes`] if you prefer setting the included widget id on the modal widget.
///
/// This property calls [`set_modal_included`] on the widget.
///
/// [`modal`]: fn@modal
/// [`set_modal_included`]: WidgetInfoBuilderModalExt::set_modal_included
/// [`modal_includes`]: fn@modal_includes
#[property(CONTEXT)]
pub fn modal_included(child: impl IntoUiNode, modal_or_descendant: impl IntoVar<WidgetId>) -> UiNode {
    let modal = modal_or_descendant.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_info(&modal);
        }
        UiNodeOp::Info { info } => {
            info.set_modal_included(modal.get());
        }
        _ => {}
    })
}

/// Widget info builder extensions for [`modal`] control.
///
/// [`modal`]: fn@modal
pub trait WidgetInfoBuilderModalExt {
    /// Include an extra widget in the modal filter of this widget.
    fn insert_modal_include(&mut self, include: WidgetId);
    /// Register a modal widget that must include this widget in its modal filter.
    fn set_modal_included(&mut self, modal: WidgetId);
}
impl WidgetInfoBuilderModalExt for WidgetInfoBuilder {
    fn insert_modal_include(&mut self, include: WidgetId) {
        self.with_meta(|mut m| m.entry(*MODAL_INCLUDES).or_default().insert(include));
    }

    fn set_modal_included(&mut self, modal: WidgetId) {
        self.set_meta(*MODAL_INCLUDED, modal);
    }
}

trait WidgetInfoModalExt {
    fn modal_includes(&self, id: WidgetId) -> bool;
    fn modal_included(&self, modal: WidgetId) -> bool;
}
impl WidgetInfoModalExt for WidgetInfo {
    fn modal_includes(&self, id: WidgetId) -> bool {
        self.id() == id || self.meta().get(*MODAL_INCLUDES).map(|i| i.contains(&id)).unwrap_or(false)
    }

    fn modal_included(&self, modal: WidgetId) -> bool {
        if let Some(id) = self.meta().get_clone(*MODAL_INCLUDED) {
            if id == modal {
                return true;
            }
            if let Some(id) = self.tree().get(id) {
                return id.ancestors().any(|w| w.id() == modal);
            }
        }
        false
    }
}

static_id! {
    static ref MODAL_INCLUDES: StateId<IdSet<WidgetId>>;
    static ref MODAL_INCLUDED: StateId<WidgetId>;
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
    #[property(EVENT)]
    pub fn on_interactivity_changed(child: impl IntoUiNode, handler: Handler<Interactivity>) -> UiNode {
        VarEventNodeBuilder::new(|| {
            let win_id = WINDOW.id();
            let wgt_id = WIDGET.id();
            WIDGET_TREE_CHANGED_EVENT.var_map(
                move |a| {
                    if a.tree.window_id() == win_id
                        && let Some(w) = a.tree.get(wgt_id)
                    {
                        Some(w.interactivity())
                    } else {
                        None
                    }
                },
                Interactivity::empty,
            )
        })
        .build::<false>(child, handler)
    }
}
old_stuff! {
    /// Widget was enabled or disabled.
    ///
    /// Note that this event tracks the actual enabled status of the widget, not the visually enabled status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn enabled_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.enabled_change(WIDGET.id()).is_some(),
    }

    /// Widget changed to enabled or disabled visuals.
    ///
    /// Note that this event tracks the visual enabled status of the widget, not the actual status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn vis_enabled_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.vis_enabled_change(WIDGET.id()).is_some(),
    }

    /// Widget interactions where blocked or unblocked.
    ///
    /// Note that blocked widgets may still be visually enabled, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn blocked_changed {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.blocked_change(WIDGET.id()).is_some(),
    }

    /// Widget normal interactions now enabled.
    ///
    /// Note that this event tracks the actual enabled status of the widget, not the visually enabled status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts enabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_enabled_changed`] for a more general event.
    ///
    /// [`on_enabled_changed`]: fn@on_enabled_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn enable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_enable(WIDGET.id()),
    }

    /// Widget normal interactions now disabled.
    ///
    /// Note that this event tracks the actual enabled status of the widget, not the visually enabled status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts disabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_enabled_changed`] for a more general event.
    ///
    /// [`on_enabled_changed`]: fn@on_enabled_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn disable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_disable(WIDGET.id()),
    }

    /// Widget now looks enabled.
    ///
    /// Note that this event tracks the visual enabled status of the widget, not the actual status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts visually enabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_vis_enabled_changed`] for a more general event.
    ///
    /// [`on_vis_enabled_changed`]: fn@on_vis_enabled_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn vis_enable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_vis_enable(WIDGET.id()),
    }

    /// Widget now looks disabled.
    ///
    /// Note that this event tracks the visual enabled status of the widget, not the actual status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts visually disabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_vis_enabled_changed`] for a more general event.
    ///
    /// [`on_vis_enabled_changed`]: fn@on_vis_enabled_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn vis_disable {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_vis_disable(WIDGET.id()),
    }

    /// Widget interactions now blocked.
    ///
    /// Note that blocked widgets may still be visually enabled, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts blocked,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_blocked_changed`] for a more general event.
    ///
    /// [`on_blocked_changed`]: fn@on_blocked_changed
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
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
    /// [`Interactivity`]: zng_app::widget::info::Interactivity
    /// [`is_new`]: info::InteractivityChangedArgs::is_new
    pub fn unblock {
        event: info::INTERACTIVITY_CHANGED_EVENT,
        args: info::InteractivityChangedArgs,
        filter: |a| a.is_unblock(WIDGET.id()),
    }
}
