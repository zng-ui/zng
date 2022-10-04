# Var/Update Rewrite

* Maybe we can have a handles collection for each widget available in the WidgetContext?
    - Avoids one Vec per node with variables, 
    - May cause two Vec for widgets without vars?
    - If the refactored `EventHandle` has the same internal type as `VarHandle` we can use a single Vec.

* Remove `UiNode::subscriptions`, should be mostly removed already.
    - Refactor WidgetHandle, some other context-var wrappers also.
    - Use `impl_ui_node(struct Node { ..})` syntax everywhere.
* Review "!!:".
* Docs.
* Test.

* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* `Var::display` like `Path::display`,  for easy formatting.
* Review `VarCapabilities` variant names.
* Review `unsafe`, only use when there is no alternative.

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
        handles: Default::default(),
    }
}
```