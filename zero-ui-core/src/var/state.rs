use crate::{context::*, event::*, var::*, widget_info::*, *};

/// Helper for declaring state properties that depend on a single event.
pub fn event_state<E: Event>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event: E,
    on_event: impl FnMut(&mut WidgetContext, &E::Args) -> Option<bool> + 'static,
) -> impl UiNode {
    struct EventStateNode<C, E, S> {
        child: C,
        event: E,
        default: bool,
        state: StateVar,
        on_event: S,
    }
    #[impl_ui_node(child)]
    impl<C, E, S> UiNode for EventStateNode<C, E, S>
    where
        C: UiNode,
        E: Event,
        S: FnMut(&mut WidgetContext, &E::Args) -> Option<bool> + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.child.deinit(ctx);
        }
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.event);
            self.child.subscriptions(ctx, subs);
        }
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.event.update(args) {
                if let Some(state) = (self.on_event)(ctx, args) {
                    self.state.set_ne(ctx, state);
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }
    }
    EventStateNode {
        child: child.cfg_boxed(),
        event,
        default,
        state,
        on_event,
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on two other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_state2<E0: Event, E1: Event>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event0: E0,
    default0: bool,
    on_event0: impl FnMut(&mut WidgetContext, &E0::Args) -> Option<bool> + 'static,
    event1: E1,
    default1: bool,
    on_event1: impl FnMut(&mut WidgetContext, &E1::Args) -> Option<bool> + 'static,
    merge: impl FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + 'static,
) -> impl UiNode {
    struct EventState2Node<C, E0, E1, S0, S1, M> {
        child: C,
        events: (E0, E1),
        default: bool,
        state: StateVar,
        on_events: (S0, S1),
        partial_default: (bool, bool),
        partial: (bool, bool),
        merge: M,
    }
    #[impl_ui_node(child)]
    impl<C, E0, E1, S0, S1, M> UiNode for EventState2Node<C, E0, E1, S0, S1, M>
    where
        C: UiNode,
        E0: Event,
        E1: Event,
        S0: FnMut(&mut WidgetContext, &E0::Args) -> Option<bool> + 'static,
        S1: FnMut(&mut WidgetContext, &E1::Args) -> Option<bool> + 'static,
        M: FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.partial = self.partial_default;
            self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.child.deinit(ctx);
        }
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.events.0).event(self.events.1);
            self.child.subscriptions(ctx, subs);
        }
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let mut update = false;
            if let Some(args) = self.events.0.update(args) {
                if let Some(state) = (self.on_events.0)(ctx, args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        update = true;
                    }
                }
                self.child.event(ctx, args);
            } else if let Some(args) = self.events.1.update(args) {
                if let Some(state) = (self.on_events.1)(ctx, args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        update = true;
                    }
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if update {
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
    }
    .cfg_boxed()
}

/// Helper for declaring state properties that depend on tree other event states.
#[allow(clippy::too_many_arguments)]
pub fn event_state3<E0: Event, E1: Event, E2: Event>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event0: E0,
    default0: bool,
    on_event0: impl FnMut(&mut WidgetContext, &E0::Args) -> Option<bool> + 'static,
    event1: E1,
    default1: bool,
    on_event1: impl FnMut(&mut WidgetContext, &E1::Args) -> Option<bool> + 'static,
    event2: E2,
    default2: bool,
    on_event2: impl FnMut(&mut WidgetContext, &E2::Args) -> Option<bool> + 'static,
    merge: impl FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + 'static,
) -> impl UiNode {
    struct EventState3Node<C, E0, E1, E2, S0, S1, S2, M> {
        child: C,
        events: (E0, E1, E2),
        default: bool,
        state: StateVar,
        on_events: (S0, S1, S2),
        partial_default: (bool, bool, bool),
        partial: (bool, bool, bool),
        merge: M,
    }
    #[impl_ui_node(child)]
    impl<C, E0, E1, E2, S0, S1, S2, M> UiNode for EventState3Node<C, E0, E1, E2, S0, S1, S2, M>
    where
        C: UiNode,
        E0: Event,
        E1: Event,
        E2: Event,
        S0: FnMut(&mut WidgetContext, &E0::Args) -> Option<bool> + 'static,
        S1: FnMut(&mut WidgetContext, &E1::Args) -> Option<bool> + 'static,
        S2: FnMut(&mut WidgetContext, &E2::Args) -> Option<bool> + 'static,
        M: FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + 'static,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.partial = self.partial_default;
            self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
            self.child.deinit(ctx);
        }
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            subs.event(self.events.0).event(self.events.1).event(self.events.2);
            self.child.subscriptions(ctx, subs);
        }
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let mut update = false;
            if let Some(args) = self.events.0.update(args) {
                if let Some(state) = (self.on_events.0)(ctx, args) {
                    if self.partial.0 != state {
                        self.partial.0 = state;
                        update = true;
                    }
                }
                self.child.event(ctx, args);
            } else if let Some(args) = self.events.1.update(args) {
                if let Some(state) = (self.on_events.1)(ctx, args) {
                    if self.partial.1 != state {
                        self.partial.1 = state;
                        update = true;
                    }
                }
                self.child.event(ctx, args);
            } else if let Some(args) = self.events.2.update(args) {
                if let Some(state) = (self.on_events.2)(ctx, args) {
                    if self.partial.2 != state {
                        self.partial.2 = state;
                        update = true;
                    }
                }
                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }

            if update {
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
        binding: Option<VarBindingHandle>,
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
