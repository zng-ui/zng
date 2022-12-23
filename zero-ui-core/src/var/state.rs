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
/// # #[widget($crate::text)]
/// # pub mod text { use super::*; inherit!(crate::widget_base::base); properties! { pub txt(impl IntoVar<Text>); } }
/// # fn main() { 
/// let probe = state_var();
/// # let _ =
/// text! {
///     txt = probe.map(|p| formatx!("is_pressed = {p:?}"));
///     is_pressed = probe;
/// }
/// # ; }
/// ```
pub fn state_var() -> ArcVar<bool> {
    var(false)
}

/// Variable for getter properties (`get_`).
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
/// # #[widget($crate::row)]
/// # pub mod row {
/// #   use super::*;
/// #   inherit!(crate::widget_base::base);
/// #   pub use super::get_index;
/// #   properties! { pub background_color(impl IntoVar<Rgba>); }
/// # }
/// # fn main() { 
/// let probe = getter_var::<usize>();
/// # let _ =
/// row! {
///     background_color = probe.map(|&i| {
///         let g = (i % 255) as u8;
///         rgb(g, g, g)
///     };
///     get_index = probe;
/// }
/// # ; }
/// ```
pub fn getter_var<T: VarValue + Default>() -> ArcVar<T> {
    var(T::default())
}

fn validate_getter_var<T: VarValue>(ctx: &mut WidgetContext, var: &impl Var<T>) {
    #[cfg(debug_assertions)]
    if var.capabilities().is_always_read_only() {
        tracing::error!("`is_`, `has_` or `get_` property inited with read-only var in {:?}", ctx.path);
    }
}

/// Helper for declaring state properties that depend on a single event.
pub fn event_is_state<A: EventArgs>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event: Event<A>,
    on_event: impl FnMut(&mut WidgetContext, &A) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct EventStateNode<A: EventArgs> {
        child: impl UiNode,
        #[event] event: Event<A>,
        default: bool,
        state: impl Var<bool>,
        on_event: impl FnMut(&mut WidgetContext, &A) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            validate_getter_var(ctx, &self.state);
            self.auto_subs(ctx);
            let _ = self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let _ = self.state.set_ne(ctx, self.default);
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = self.event.on(update) {
                if let Some(state) = (self.on_event)(ctx, args) {
                    let _ = self.state.set_ne(ctx, state);
                }
            }
            self.child.event(ctx, update);
        }
    }
    EventStateNode {
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
pub fn event_is_state2<A0: EventArgs, A1: EventArgs>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + Send + 'static,
    merge: impl FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct EventState2Node<A0: EventArgs, A1: EventArgs,> {
        child: impl UiNode,
        #[event] event0: Event<A0>,
        #[event] event1: Event<A1>,
        default: bool,
        state: impl Var<bool>,
        on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + Send + 'static,
        merge: impl FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + Send + 'static,
        partial: (bool, bool),
        partial_default: (bool, bool),
    })]
    impl UiNode for EventState2Node {
        fn init(&mut self, ctx: &mut WidgetContext) {
            validate_getter_var(ctx, &self.state);
            self.auto_subs(ctx);

            self.partial = self.partial_default;
            let _ = self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let _ = self.state.set_ne(ctx, self.default);
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.event0.on(update) {
                if let Some(state) = (self.on_event0)(ctx, args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event1.on(update) {
                if let Some(state) = (self.on_event1)(ctx, args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(ctx, update);

            if updated {
                if let Some(value) = (self.merge)(ctx, self.partial.0, self.partial.1) {
                    let _ = self.state.set_ne(ctx, value);
                }
            }
        }
    }
    EventState2Node {
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

/// Helper for declaring state properties that depend on tree other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_is_state3<A0: EventArgs, A1: EventArgs, A2: EventArgs>(
    child: impl UiNode,
    state: impl IntoVar<bool>,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + Send + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + Send + 'static,
    event2: Event<A2>,
    default2: bool,
    on_event2: impl FnMut(&mut WidgetContext, &A2) -> Option<bool> + Send + 'static,
    merge: impl FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct EventState3Node<A0: EventArgs, A1: EventArgs, A2: EventArgs> {
        child: impl UiNode,
        #[event] event0: Event<A0>,
        #[event] event1: Event<A1>,
        #[event] event2: Event<A2>,
        default: bool,
        state: impl Var<bool>,
        on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + Send + 'static,
        on_event2: impl FnMut(&mut WidgetContext, &A2) -> Option<bool> + Send + 'static,
        partial_default: (bool, bool, bool),
        partial: (bool, bool, bool),
        merge: impl FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventState3Node {
        fn init(&mut self, ctx: &mut WidgetContext) {
            validate_getter_var(ctx, &self.state);
            self.auto_subs(ctx);

            self.partial = self.partial_default;
            let _ = self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            let _ = self.state.set_ne(ctx, self.default);
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.event0.on(update) {
                if let Some(state) = (self.on_event0)(ctx, args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event1.on(update) {
                if let Some(state) = (self.on_event1)(ctx, args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.event2.on(update) {
                if let Some(state) = (self.on_event2)(ctx, args) {
                    if self.partial.2 != state {
                        self.partial.2 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(ctx, update);

            if updated {
                if let Some(value) = (self.merge)(ctx, self.partial.0, self.partial.1, self.partial.2) {
                    let _ = self.state.set_ne(ctx, value);
                }
            }
        }
    }
    EventState3Node {
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

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create composite properties
/// that merge other state properties.
pub fn bind_is_state(child: impl UiNode, source: impl IntoVar<bool>, state: impl IntoVar<bool>) -> impl UiNode {
    #[ui_node(struct BindStateNode {
        child: impl UiNode,
        source: impl Var<bool>,
        state: impl Var<bool>,
        binding: VarHandle,
    })]
    impl UiNode for BindStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            validate_getter_var(ctx, &self.state);
            let _ = self.state.set_ne(ctx, self.source.get());
            self.binding = self.source.bind(&self.state);
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.binding = VarHandle::dummy();
            self.child.deinit(ctx);
        }
    }
    BindStateNode {
        child: child.cfg_boxed(),
        source: source.into_var(),
        state: state.into_var(),
        binding: VarHandle::dummy(),
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that are controlled by values in the widget state map.
///
/// The `predicate` closure is called with the widget state every update.
pub fn widget_state_is_state(
    child: impl UiNode,
    predicate: impl Fn(StateMapRef<state_map::Widget>) -> bool + Send + 'static,
    state: impl IntoVar<bool>,
) -> impl UiNode {
    #[ui_node(struct BindWidgetStateNode {
        child: impl UiNode,
        state: impl Var<bool>,
        predicate: impl Fn(StateMapRef<state_map::Widget>) -> bool + Send + 'static,
    })]
    impl UiNode for BindWidgetStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            validate_getter_var(ctx, &self.state);
            self.child.init(ctx);
            let state = (self.predicate)(ctx.widget_state.as_ref());
            if state != self.state.get() {
                let _ = self.state.set(ctx.vars, state);
            }
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            if self.state.get() {
                let _ = self.state.set(ctx.vars, false);
            }
        }
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            let state = (self.predicate)(ctx.widget_state.as_ref());
            if state != self.state.get() {
                let _ = self.state.set(ctx.vars, state);
            }
        }
    }
    BindWidgetStateNode {
        child: child.cfg_boxed(),
        state: state.into_var(),
        predicate,
    }
}
