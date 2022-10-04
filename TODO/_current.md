# Var/Update Rewrite

* Remove `UiNode::subscriptions`, should be mostly removed already.
    - Use `impl_ui_node(struct Node { ..})` syntax everywhere.
    - Refactor RcNode, removed a shared update-mask that was used to signal all slots.
        - Probably needs re-implementation, current API very confusing.
        - Need to (de)init in different widget contexts.
        - Need the "take" stuff to be associated with a RcNode instance, right now the public API is split.
    - Refactor render image "retain", can't see if windows subscribe now (previous impl was iffy).
        - Make it explicit, only if requested.
* Review "!!:".
* Docs.
* Test.

* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* `Var::display` like `Path::display`,  for easy formatting.
* `RcVar::set` and other infallible overrides.
* Review `VarCapabilities` variant names.
* Review `unsafe`, only use when there is no alternative.
* Review nodes that call `(de)init(ctx)`, are they causing a widget handle collection to grow uncontrolledly?

* Implement event handlers using a callback in the event that queues the handlers to run once. 
    - This avoids the linear event update search.
    - This causes lets us unify all event handles to a single `EventHandle` like the `VarHandle`.
* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.

# New Node Syntax

```rust
#[property(context)]
pub fn bar(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<bool>) -> impl UiNode {
    #[impl_ui_node(struct BarNode {
        child: impl UiNode,

        var_a: impl Var<bool>,
        var_b: impl Var<bool>,

        event_click: Event<ClickArgs>,

        custom: Vec<bool>,
    })]
    impl BarNode {
        fn custom(&self, arg: bool) -> bool {
            println!("custom fn");
            !arg
        }

        #[UiNode]
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            if let Some(args) = self.event_click.on(update) {
                args.propagation().stop();
            }
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if let Some(v) = self.var_a.get_new(ctx) {
                self.custom.push(v);
                self.custom(v);
            }
        }
    }
    BarNode {
        child,
        var_a: a.into_var(),
        var_b: b.into_var(),
        event_click: CLICK_EVENT,
        custom: vec![],
    }
}
```