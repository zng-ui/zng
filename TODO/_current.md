# Var/Update Rewrite

* Implement `ui_node!`.
* Remove `UiNode::subscriptions`, should be mostly removed already.
    - Refactor WidgetHandle, some other context-var wrappers also.
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

# ui_node! Macro

```rust
// current

#[property(context)]
pub fn foo(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<bool>) -> impl UiNode {
    ui_node! {
        self.child: impl UiNode = child;

        self.var.a: impl Var<bool> = a.into_var();
        self.var.b: impl Var<bool> = b.into_var();

        self.event.click: Event<ClickArgs> = CLICK_EVENT;

        self.custom: Vec<bool> = vec![];

        // self.event.handles.push(CHAR_INPUT_EVENT.subscribe(ctx));
        // self.var.handles.push(TEXT_COLOR_VAR.subscribe(ctx));

        fn custom(&self, arg: bool) -> bool {
            println!("custom fn");
            !arg
        }

        #[UiNode]
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            if let Some(args) = self.event.click.on(update) {
                args.propagation().stop();
            }
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if let Some(v) = self.var.a.get_new(ctx) {
                self.custom.push(v);
                self.custom(v);
            }
        }
    }
}
```

* Problems:
    - no format?
        - rustfmt does not format braced macros, see https://github.com/rust-lang/rustfmt/pull/5538
    - no auto-complete.
    - no type help on hover for method inputs.
    - no mut underline highlight.

```rust
// alt syntax
#[property(context)]
pub fn foo(child: impl UiNode, a: impl IntoVar<bool>, b: impl IntoVar<bool>) -> impl UiNode {
    #[impl_ui_node(struct FooNode {
        child: impl UiNode = child,

        var.a: impl Var<bool> = a.into_var(),
        var.b: impl Var<bool> = b.into_var(),

        event.click: Event<ClickArgs> = CLICK_EVENT,

        custom: Vec<bool> = vec![],

        // allow this in custom fn init, only generate handles fields if the user tries to access then?
        // event.handles.push(CHAR_INPUT_EVENT.subscribe(ctx));
        // var.handles.push(TEXT_COLOR_VAR.subscribe(ctx));
    })]
    impl FooNode {
        fn custom(&self, arg: bool) -> bool {
            println!("custom fn");
            !arg
        }

        #[UiNode]
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            if let Some(args) = self.event.click.on(update) {
                args.propagation().stop();
            }
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if let Some(v) = self.var.a.get_new(ctx) {
                self.custom.push(v);
                self.custom(v);
            }
        }
    }
}
```

* Advantages:
    - Format in the impl block.
    - Full rust-analyzer support in the impl block (auto-complete, semantic highlight).
        - Need to test if the generated struct is found by RA, the `ui_node!` macro did not expand this either.
* Disadvantages:
    - More weird syntax?
        - Actually we can have a `macro_rules! ui_node` that expands to this syntax (using the ".." separator trick from `event_args!`), 
          will this break fmt and RA as well?