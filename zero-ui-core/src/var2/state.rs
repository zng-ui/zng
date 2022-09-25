use super::*;

use crate::{context::*, event::*, *};

/// New [`StateVar`].
pub fn state_var() -> StateVar {
    var(false)
}

/// Variable type of state properties (`is_*`).
///
/// State variables are `bool` probes that are set by the property.
///
/// Use [`state_var`] to init.
pub type StateVar = RcVar<bool>;

/// Helper for declaring state properties that depend on a single event.
pub fn event_state<A: EventArgs>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event: Event<A>,
    on_event: impl FnMut(&mut WidgetContext, &A) -> Option<bool> + 'static,
) -> impl UiNode {
    struct EventStateNode<C, A: EventArgs, S> {
        child: C,
        event: Event<A>,
        default: bool,
        state: StateVar,
        on_event: S,
        handle: Option<EventWidgetHandle>,
    }
    #[impl_ui_node(child)]
    impl<C, A, S> UiNode for EventStateNode<C, A, S>
    where
        C: UiNode,
        A: EventArgs,
        S: FnMut(&mut WidgetContext, &A) -> Option<bool> + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.handle = Some(self.event.subscribe(ctx.path.widget_id()));
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.handle = None;
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = self.event.on(update) {
                if let Some(state) = (self.on_event)(ctx, args) {
                    self.state.set_ne(ctx, state);
                }
            }
            self.child.event(ctx, update);
        }
    }
    EventStateNode {
        child: child.cfg_boxed(),
        event,
        default,
        state,
        on_event,
        handle: None,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on two other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_state2<A0: EventArgs, A1: EventArgs>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + 'static,
    merge: impl FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + 'static,
) -> impl UiNode {
    struct EventState2Node<C, A0: EventArgs, A1: EventArgs, S0, S1, M> {
        child: C,
        events: (Event<A0>, Event<A1>),
        default: bool,
        state: StateVar,
        on_events: (S0, S1),
        partial_default: (bool, bool),
        partial: (bool, bool),
        merge: M,
        handle: Option<[EventWidgetHandle; 2]>,
    }
    #[impl_ui_node(child)]
    impl<C, A0, A1, S0, S1, M> UiNode for EventState2Node<C, A0, A1, S0, S1, M>
    where
        C: UiNode,
        A0: EventArgs,
        A1: EventArgs,
        S0: FnMut(&mut WidgetContext, &A0) -> Option<bool> + 'static,
        S1: FnMut(&mut WidgetContext, &A1) -> Option<bool> + 'static,
        M: FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.partial = self.partial_default;
            self.state.set_ne(ctx, self.default);
            let w = ctx.path.widget_id();
            self.handle = Some([self.events.0.subscribe(w), self.events.1.subscribe(w)]);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.handle = None;
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.events.0.on(update) {
                if let Some(state) = (self.on_events.0)(ctx, args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.events.1.on(update) {
                if let Some(state) = (self.on_events.1)(ctx, args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(ctx, update);

            if updated {
                if let Some(value) = (self.merge)(ctx, self.partial.0, self.partial.1) {
                    self.state.set_ne(ctx, value);
                }
            }
        }
    }
    EventState2Node {
        child: child.cfg_boxed(),
        events: (event0, event1),
        default,
        state,
        on_events: (on_event0, on_event1),
        partial_default: (default0, default1),
        partial: (default0, default1),
        merge,
        handle: None,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on tree other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_state3<A0: EventArgs, A1: EventArgs, A2: EventArgs>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event0: Event<A0>,
    default0: bool,
    on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + 'static,
    event1: Event<A1>,
    default1: bool,
    on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + 'static,
    event2: Event<A2>,
    default2: bool,
    on_event2: impl FnMut(&mut WidgetContext, &A2) -> Option<bool> + 'static,
    merge: impl FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + 'static,
) -> impl UiNode {
    struct EventState3Node<C, A0: EventArgs, A1: EventArgs, A2: EventArgs, S0, S1, S2, M> {
        child: C,
        events: (Event<A0>, Event<A1>, Event<A2>),
        default: bool,
        state: StateVar,
        on_events: (S0, S1, S2),
        partial_default: (bool, bool, bool),
        partial: (bool, bool, bool),
        merge: M,
        handle: Option<[EventWidgetHandle; 3]>,
    }
    #[impl_ui_node(child)]
    impl<C, A0, A1, A2, S0, S1, S2, M> UiNode for EventState3Node<C, A0, A1, A2, S0, S1, S2, M>
    where
        C: UiNode,
        A0: EventArgs,
        A1: EventArgs,
        A2: EventArgs,
        S0: FnMut(&mut WidgetContext, &A0) -> Option<bool> + 'static,
        S1: FnMut(&mut WidgetContext, &A1) -> Option<bool> + 'static,
        S2: FnMut(&mut WidgetContext, &A2) -> Option<bool> + 'static,
        M: FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.partial = self.partial_default;
            self.state.set_ne(ctx, self.default);
            let w = ctx.path.widget_id();
            self.handle = Some([self.events.0.subscribe(w), self.events.1.subscribe(w), self.events.2.subscribe(w)]);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.handle = None;
            self.child.deinit(ctx);
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            let mut updated = false;
            if let Some(args) = self.events.0.on(update) {
                if let Some(state) = (self.on_events.0)(ctx, args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.events.1.on(update) {
                if let Some(state) = (self.on_events.1)(ctx, args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        updated = true;
                    }
                }
            } else if let Some(args) = self.events.2.on(update) {
                if let Some(state) = (self.on_events.2)(ctx, args) {
                    if self.partial.2 != state {
                        self.partial.2 = state;
                        updated = true;
                    }
                }
            }
            self.child.event(ctx, update);

            if updated {
                if let Some(value) = (self.merge)(ctx, self.partial.0, self.partial.1, self.partial.2) {
                    self.state.set_ne(ctx, value);
                }
            }
        }
    }
    EventState3Node {
        child: child.cfg_boxed(),
        events: (event0, event1, event2),
        default,
        state,
        on_events: (on_event0, on_event1, on_event2),
        partial_default: (default0, default1, default2),
        partial: (default0, default1, default2),
        merge,
        handle: None,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that are controlled by a variable.
///
/// On init the `state` variable is set to `source` and bound to it, you can use this to create composite properties
/// that merge other state properties.
pub fn bind_state(child: impl UiNode, source: impl IntoVar<bool>, state: StateVar) -> impl UiNode {
    struct BindStateNode<C, S> {
        child: C,
        source: S,
        state: StateVar,
        binding: Option<VarHandle>,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, S: Var<bool>> UiNode for BindStateNode<C, S> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx.vars, self.source.copy(ctx.vars));
            if self.source.can_update() {
                self.binding = Some(self.source.bind(ctx, &self.state));
            }
            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.binding = None;
            self.child.deinit(ctx);
        }
    }
    BindStateNode {
        child: child.cfg_boxed(),
        source: source.into_var(),
        state,
        binding: None,
    }
    .cfg_boxed()
}
