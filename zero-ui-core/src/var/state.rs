use super::*;

use crate::{context::*, event::*, widget_instance::*, *};

/// New [`StateVar`].
pub fn state_var() -> StateVar {
    var(false)
}

/// Variable type of state properties (`is_*`).
///
/// State variables are `bool` probes that are set by the property.
///
/// Use [`state_var`] to init.
pub type StateVar = ArcVar<bool>;

/// Helper for declaring state properties that depend on a single event.
pub fn event_state<A: EventArgs>(
    child: impl UiNode,
    state: StateVar,
    default: bool,
    event: Event<A>,
    on_event: impl FnMut(&mut WidgetContext, &A) -> Option<bool> + Send + 'static,
) -> impl UiNode {
    #[ui_node(struct EventStateNode<A: EventArgs> {
        child: impl UiNode,
        #[event] event: Event<A>,
        default: bool,
        state: StateVar,
        on_event: impl FnMut(&mut WidgetContext, &A) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);
            self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
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
        state: StateVar,
        on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + Send + 'static,
        merge: impl FnMut(&mut WidgetContext, bool, bool) -> Option<bool> + Send + 'static,
        partial: (bool, bool),
        partial_default: (bool, bool),
    })]
    impl UiNode for EventState2Node {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);

            self.partial = self.partial_default;
            self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
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
                    self.state.set_ne(ctx, value);
                }
            }
        }
    }
    EventState2Node {
        child: child.cfg_boxed(),
        event0,
        event1,
        default,
        state,
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
pub fn event_state3<A0: EventArgs, A1: EventArgs, A2: EventArgs>(
    child: impl UiNode,
    state: StateVar,
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
        state: StateVar,
        on_event0: impl FnMut(&mut WidgetContext, &A0) -> Option<bool> + Send + 'static,
        on_event1: impl FnMut(&mut WidgetContext, &A1) -> Option<bool> + Send + 'static,
        on_event2: impl FnMut(&mut WidgetContext, &A2) -> Option<bool> + Send + 'static,
        partial_default: (bool, bool, bool),
        partial: (bool, bool, bool),
        merge: impl FnMut(&mut WidgetContext, bool, bool, bool) -> Option<bool> + Send + 'static,
    })]
    impl UiNode for EventState3Node {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.auto_subs(ctx);

            self.partial = self.partial_default;
            self.state.set_ne(ctx, self.default);
            self.child.init(ctx);
        }
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.default);
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
                    self.state.set_ne(ctx, value);
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
        state,
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
pub fn bind_state(child: impl UiNode, source: impl IntoVar<bool>, state: StateVar) -> impl UiNode {
    #[ui_node(struct BindStateNode {
        child: impl UiNode,
        source: impl Var<bool>,
        state: StateVar,
        binding: VarHandle,
    })]
    impl UiNode for BindStateNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.state.set_ne(ctx, self.source.get());
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
        state,
        binding: VarHandle::dummy(),
    }
    .cfg_boxed()
}
