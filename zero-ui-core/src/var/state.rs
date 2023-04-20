use super::*;

use crate::{context::*, event::*, widget_instance::*, *};

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
    on_event: impl FnMut(&A) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct EventIsStateNode<A: EventArgs> {
        child: impl UiNode,
        #[event] event: Event<A>,
        default: bool,
        state: impl Var<bool>,
        on_event: impl FnMut(&A) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventIsStateNode {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            self.auto_subs();
            let _ = self.state.set_ne(self.default);
            self.child.init();
        }
        fn deinit(&mut self) {
            let _ = self.state.set_ne(self.default);
            self.child.deinit();
        }
        fn event(&mut self, update: &EventUpdate) {
            if let Some(args) = self.event.on(update) {
                if let Some(state) = (self.on_event)(args) {
                    let _ = self.state.set_ne(state);
                }
            }
            self.child.event(update);
        }
    }
    EventIsStateNode {
        child: child.cfg_boxed(),
        event,
        default,
        state: state.into_var(),
        on_event,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on two other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state2<A0, A1>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    merge: impl FnMut(bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
{
    #[ui_node(struct EventIsState2Node<A0: EventArgs, A1: EventArgs,> {
        child: impl UiNode,
        #[event] event0: Event<A0>,
        #[event] event1: Event<A1>,
        default: bool,
        state: impl Var<bool>,
        on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
        merge: impl FnMut(bool, bool) -> Option<bool> + Send + 'static,
        partial: (bool, bool),
        partial_default: (bool, bool),
    })]
    impl UiNode for EventIsState2Node {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            self.auto_subs();

            self.partial = self.partial_default;
            let _ = self.state.set_ne(self.default);
            self.child.init();
        }
        fn deinit(&mut self) {
            let _ = self.state.set_ne(self.default);
            self.child.deinit();
        }
        fn event(&mut self, update: &EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.event0.on(update) {
                if let Some(state) = (self.on_event0)(args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event1.on(update) {
                if let Some(state) = (self.on_event1)(args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(update);

            if updated {
                if let Some(value) = (self.merge)(self.partial.0, self.partial.1) {
                    let _ = self.state.set_ne(value);
                }
            }
        }
    }
    EventIsState2Node {
        child: child.cfg_boxed(),
        event0,
        event1,
        default,
        state: state.into_var(),
        on_event0,
        on_event1,
        partial_default: (default0, default1),
        partial: (default0, default1),
        merge,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on three other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state3<A0, A1, A2>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
    merge: impl FnMut(bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
{
    #[ui_node(struct EventIsState3Node<A0: EventArgs, A1: EventArgs, A2: EventArgs> {
        child: impl UiNode,
        #[event] event0: Event<A0>,
        #[event] event1: Event<A1>,
        #[event] event2: Event<A2>,
        default: bool,
        state: impl Var<bool>,
        on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
        on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
        partial_default: (bool, bool, bool),
        partial: (bool, bool, bool),
        merge: impl FnMut(bool, bool, bool) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventIsState3Node {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            self.auto_subs();

            self.partial = self.partial_default;
            let _ = self.state.set_ne(self.default);
            self.child.init();
        }
        fn deinit(&mut self) {
            let _ = self.state.set_ne(self.default);
            self.child.deinit();
        }
        fn event(&mut self, update: &EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.event0.on(update) {
                if let Some(state) = (self.on_event0)(args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event1.on(update) {
                if let Some(state) = (self.on_event1)(args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event2.on(update) {
                if let Some(state) = (self.on_event2)(args) {
                    if self.partial.2 != state {
                        self.partial.2 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(update);

            if updated {
                if let Some(value) = (self.merge)(self.partial.0, self.partial.1, self.partial.2) {
                    let _ = self.state.set_ne(value);
                }
            }
        }
    }
    EventIsState3Node {
        child: child.cfg_boxed(),
        event0,
        event1,
        event2,
        default,
        state: state.into_var(),
        on_event0,
        on_event1,
        on_event2,
        partial_default: (default0, default1, default2),
        partial: (default0, default1, default2),
        merge,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on four other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state4<A0, A1, A2, A3>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
    event3: Event<A3>,
    default3: bool,
    on_event3: impl FnMut(&A3) -> Option<bool> + Send + 'static,
    merge: impl FnMut(bool, bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode
where
    A0: EventArgs,
    A1: EventArgs,
    A2: EventArgs,
    A3: EventArgs,
{
    #[ui_node(struct EventIsState4Node<A0: EventArgs, A1: EventArgs, A2: EventArgs, A3: EventArgs> {
        child: impl UiNode,
        #[event] event0: Event<A0>,
        #[event] event1: Event<A1>,
        #[event] event2: Event<A2>,
        #[event] event3: Event<A3>,
        default: bool,
        state: impl Var<bool>,
        on_event0: impl FnMut(&A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&A1) -> Option<bool> + Send + 'static,
        on_event2: impl FnMut(&A2) -> Option<bool> + Send + 'static,
        on_event3: impl FnMut(&A3) -> Option<bool> + Send + 'static,
        partial_default: (bool, bool, bool, bool),
        partial: (bool, bool, bool, bool),
        merge: impl FnMut(bool, bool, bool, bool) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventIsState4Node {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            self.auto_subs();

            self.partial = self.partial_default;
            let _ = self.state.set_ne(self.default);
            self.child.init();
        }
        fn deinit(&mut self) {
            let _ = self.state.set_ne(self.default);
            self.child.deinit();
        }
        fn event(&mut self, update: &EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.event0.on(update) {
                if let Some(state) = (self.on_event0)(args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event1.on(update) {
                if let Some(state) = (self.on_event1)(args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event2.on(update) {
                if let Some(state) = (self.on_event2)(args) {
                    if self.partial.2 != state {
                        self.partial.2 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event3.on(update) {
                if let Some(state) = (self.on_event3)(args) {
                    if self.partial.3 != state {
                        self.partial.3 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(update);

            if updated {
                if let Some(value) = (self.merge)(self.partial.0, self.partial.1, self.partial.2, self.partial.3) {
                    let _ = self.state.set_ne(value);
                }
            }
        }
    }
    EventIsState4Node {
        child: child.cfg_boxed(),
        event0,
        event1,
        event2,
        event3,
        default,
        state: state.into_var(),
        on_event0,
        on_event1,
        on_event2,
        on_event3,
        partial_default: (default0, default1, default2, default3),
        partial: (default0, default1, default2, default3),
        merge,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create composite properties
/// that merge other state properties.
pub fn bind_is_state(child: impl UiNode, source: impl IntoVar<bool>, state: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct BindIsStateNode {
        child: impl UiNode,
        source: impl Var<bool>,
        state: impl Var<bool>,
        binding: VarHandle,
    })]
    impl UiNode for BindIsStateNode {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            let _ = self.state.set_ne(self.source.get());
            self.binding = self.source.bind(&self.state);
            self.child.init();
        }

        fn deinit(&mut self) {
            self.binding = VarHandle::dummy();
            self.child.deinit();
        }
    }
    BindIsStateNode {
        child: child.cfg_boxed(),
        source: source.into_var(),
        state: state.into_var(),
        binding: VarHandle::dummy(),
    }
    .cfg_boxed()
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
    #[ui_node(struct WidgetStateIsStateNode {
        child: impl UiNode,
        state: impl Var<bool>,
        predicate: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
        deinit: impl Fn(StateMapRef<WIDGET>) -> bool + Send + 'static,
    })]
    impl UiNode for WidgetStateIsStateNode {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            self.child.init();
            let state = WIDGET.with_state(&mut self.predicate);
            if state != self.state.get() {
                let _ = self.state.set(state);
            }
        }
        fn deinit(&mut self) {
            self.child.deinit();
            let state = WIDGET.with_state(&mut self.deinit);
            if state != self.state.get() {
                let _ = self.state.set(state);
            }
        }
        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);
            let state = WIDGET.with_state(&mut self.predicate);
            if state != self.state.get() {
                let _ = self.state.set(state);
            }
        }
    }
    WidgetStateIsStateNode {
        child: child.cfg_boxed(),
        state: state.into_var(),
        predicate,
        deinit,
    }
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
    #[ui_node(struct WidgetStateGetStateNode<T: VarValue> {
        _t: PhantomData<T>,
        child: impl UiNode,
        state: impl Var<T>,
        get_new: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
        get_deinit: impl Fn(StateMapRef<WIDGET>, &T) -> Option<T> + Send + 'static,
    })]
    impl UiNode for WidgetStateGetStateNode {
        fn init(&mut self) {
            validate_getter_var(&self.state);
            self.child.init();
            let new = self.state.with(|s| WIDGET.with_state(|w| (self.get_new)(w, s)));
            if let Some(new) = new {
                let _ = self.state.set(new);
            }
        }

        fn deinit(&mut self) {
            self.child.deinit();

            let new = self.state.with(|s| WIDGET.with_state(|w| (self.get_deinit)(w, s)));
            if let Some(new) = new {
                let _ = self.state.set(new);
            }
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            self.child.update(updates);
            let new = self.state.with(|s| WIDGET.with_state(|w| (self.get_new)(w, s)));
            if let Some(new) = new {
                let _ = self.state.set(new);
            }
        }
    }
    WidgetStateGetStateNode {
        _t: PhantomData,
        child: child.cfg_boxed(),
        state: state.into_var(),
        get_new,
        get_deinit,
    }
}
