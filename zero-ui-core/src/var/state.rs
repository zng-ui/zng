use super::*;

use crate::{context::*, event::*, widget_instance::*};

/// Variable for state properties (`is_*`, `has_*`).
///
/// State variables are `bool` probes that are set by the property, they are created automatically
/// by the property default when used in `when` expressions, but can be created manually.
///
/// # Examples
///
/// Example of manual usage to show a state as text:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, var::*, text::*};
/// # #[property(CONTEXT)]
/// # pub fn is_pressed(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
/// #   let _ = state;
/// #   child
/// # }
/// # #[widget($crate::Text)]
/// # pub struct Text(widget_base::WidgetBase);
/// # #[property(CHILD, widget_impl(Text))]
/// # pub fn txt(child: impl UiNode, txt: impl IntoVar<Txt>) -> impl UiNode { child }
/// # fn main() {
/// # let _scope = zero_ui_core::app::App::minimal();
/// let probe = state_var();
/// # let _ =
/// Text! {
///     txt = probe.map(|p| formatx!("is_pressed = {p:?}"));
///     is_pressed = probe;
/// }
/// # ; }
/// ```
pub fn state_var() -> ArcVar<bool> {
    var(false)
}

/// Variable for getter properties (`get_*`, `actual_*`).
///
/// Getter variables are inited with a default value that is overridden by the property on node init and updated
/// by the property when the internal state they track changes. They are created automatically by the property
/// default when used in `when` expressions, but can be created manually.
///
/// # Examples
///
/// Example of manual usage to map the state to a color:
///
/// ```
/// # use zero_ui_core::{*, widget_instance::*, var::*, text::*, color::*};
/// # #[property(CONTEXT)]
/// # pub fn get_index(child: impl UiNode, state: impl IntoVar<usize>) -> impl UiNode {
/// #   let _ = state;
/// #   child
/// # }
/// # #[widget($crate::Row)] pub struct Row(widget_base::WidgetBase);
/// # #[property(FILL)] pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode { child }
/// # fn main() {
/// # let _scope = zero_ui_core::app::App::minimal();
/// let probe = getter_var::<usize>();
/// # let _ =
/// Row! {
///     background_color = probe.map(|&i| {
///         let g = (i % 255) as u8;
///         rgb(g, g, g)
///     });
///     get_index = probe;
/// }
/// # ; }
/// ```
pub fn getter_var<T: VarValue + Default>() -> ArcVar<T> {
    var(T::default())
}

fn validate_getter_var<T: VarValue>(_var: &impl Var<T>) {
    #[cfg(debug_assertions)]
    if _var.capabilities().is_always_read_only() {
        tracing::error!("`is_`, `has_` or `get_` property inited with read-only var in {:?}", WIDGET.id());
    }
}

/// Helper for declaring state properties that depend on a single event.
pub fn event_is_state<A: EventArgs>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event: Event<A>,
    mut on_event: impl FnMut(&A) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event);
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = event.on(update) {
                if let Some(s) = on_event(args) {
                    let _ = state.set(s);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on two other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state2<A0, A1>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    mut on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    mut on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    mut merge: impl FnMut(bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
{
    let state = state.into_var();
    let partial_default = (default0, default1);
    let mut partial = (default0, default1);

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1);

            partial = partial_default;
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            let mut updated = false;
            if let Some(args) = event0.on(update) {
                if let Some(state) = on_event0(args) {
                    if partial.0 != state {
                        partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event1.on(update) {
                if let Some(state) = on_event1(args) {
                    if partial.1 != state {
                        partial.1 = state;
                        updated = true;
                    }
                }
            }
            child.event(update);

            if updated {
                if let Some(value) = merge(partial.0, partial.1) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on three other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state3<A0, A1, A2>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    mut on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    mut on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    mut on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
    mut merge: impl FnMut(bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
{
    let state = state.into_var();
    let partial_default = (default0, default1, default2);
    let mut partial = (default0, default1, default2);

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1).sub_event(&event2);

            partial = partial_default;
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            let mut updated = false;
            if let Some(args) = event0.on(update) {
                if let Some(state) = on_event0(args) {
                    if partial.0 != state {
                        partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event1.on(update) {
                if let Some(state) = on_event1(args) {
                    if partial.1 != state {
                        partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event2.on(update) {
                if let Some(state) = on_event2(args) {
                    if partial.2 != state {
                        partial.2 = state;
                        updated = true;
                    }
                }
            }
            child.event(update);

            if updated {
                if let Some(value) = merge(partial.0, partial.1, partial.2) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that depend on four other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state4<A0, A1, A2, A3>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    mut on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    mut on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    mut on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
    event3: Event<A3>,
    default3: bool,
    mut on_event3: impl FnMut(&A3) -> Option<bool> + Send + 'static,
    mut merge: impl FnMut(bool, bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
    A3: EventArgs,
{
    let state = state.into_var();
    let partial_default = (default0, default1, default2, default3);
    let mut partial = (default0, default1, default2, default3);

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            WIDGET.sub_event(&event0).sub_event(&event1).sub_event(&event2).sub_event(&event3);

            partial = partial_default;
            let _ = state.set(default);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(default);
        }
        UiNodeOp::Event { update } => {
            let mut updated = false;
            if let Some(args) = event0.on(update) {
                if let Some(state) = on_event0(args) {
                    if partial.0 != state {
                        partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event1.on(update) {
                if let Some(state) = on_event1(args) {
                    if partial.1 != state {
                        partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event2.on(update) {
                if let Some(state) = on_event2(args) {
                    if partial.2 != state {
                        partial.2 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = event3.on(update) {
                if let Some(state) = on_event3(args) {
                    if partial.3 != state {
                        partial.3 = state;
                        updated = true;
                    }
                }
            }
            child.event(update);

            if updated {
                if let Some(value) = merge(partial.0, partial.1, partial.2, partial.3) {
                    let _ = state.set(value);
                }
            }
        }
        _ => {}
    })
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create composite properties
/// that merge other state properties.
pub fn bind_is_state(child: impl UiNode, source: impl IntoVar<bool>, state: impl IntoVar<bool>) -> impl UiNode {
    let source = source.into_var();
    let state = state.into_var();
    let mut _binding = VarHandle::dummy();

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            let _ = state.set_from(&source);
            _binding = source.bind(&state);
        }
        UiNodeOp::Deinit => {
            _binding = VarHandle::dummy();
        }
        _ => {}
    })
}

/// Helper for declaring state properties that are controlled by values in the widget state map.
///
/// The `predicate` closure is called with the widget state on init and every update, if the returned value changes the `state`
/// updates. The `deinit` closure is called on deinit to get the *reset* value.
pub fn widget_state_is_state(
    child: impl UiNode,
    predicate: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    deinit: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    state: impl IntoVar<bool>,
) -> impl UiNode {
    let state = state.into_var();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            child.init();
            let s = WIDGET.with_state(&predicate);
            if s != state.get() {
                let _ = state.set(s);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();
            let s = WIDGET.with_state(&deinit);
            if s != state.get() {
                let _ = state.set(s);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            let s = WIDGET.with_state(&predicate);
            if s != state.get() {
                let _ = state.set(s);
            }
        }
        _ => {}
    })
}

/// Helper for declaring state getter properties that are controlled by values in the widget state map.
///
/// The `get_new` closure is called with the widget state and current `state` every init and update, if it returns some value
/// the `state` updates. The `get_deinit` closure is called on deinit to get the *reset* value.
pub fn widget_state_get_state<T: VarValue>(
    child: impl UiNode,
    get_new: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    get_deinit: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    state: impl IntoVar<T>,
) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            validate_getter_var(&state);
            child.init();
            let new = state.with(|s| WIDGET.with_state(|w| get_new(w, s)));
            if let Some(new) = new {
                let _ = state.set(new);
            }
        }
        UiNodeOp::Deinit => {
            child.deinit();

            let new = state.with(|s| WIDGET.with_state(|w| get_deinit(w, s)));
            if let Some(new) = new {
                let _ = state.set(new);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);
            let new = state.with(|s| WIDGET.with_state(|w| get_new(w, s)));
            if let Some(new) = new {
                let _ = state.set(new);
            }
        }
        _ => {}
    })
}
