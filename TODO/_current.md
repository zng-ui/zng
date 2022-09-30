# Var/Update Rewrite

* Fix all build errors.
* Remove `UiNode::subscriptions`, should be mostly removed already.
    - Refactor WidgetHandle, some other context-var wrappers also.
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
* Review Command unload, if we only modify a command meta and don't create any handlers it does not register for cleanup.
    - Bug already existed in previous implementation.
    - Have an AppId?
* Implement all `todo!` code.

# Better Node Macro

* We really need a better way to declare nodes, some property nodes have have 20 lines of generics metadata.
    - And now they all have init/deinit to event and var handles.
```rust
#[property(context)]
fn foo(child: impl UiNode, foo: impl IntoVar<bool>, bar: impl IntoVar<bool>) -> impl UiNode {
    ui_node! {
        // delegate, can only be child or children.
        self.child: impl UiNode = child;

        // declare var fields, vars auto-subscribe.
        self.var.foo: impl Var<bool> = foo.into_var();
        self.var.bar: impl Var<bool> = bar.into_var();
        // declar event fields, events auto-subscribe.
        self.event.foo: Event<FooArgs> = FOO_EVENT;

        // declare only handle field, auto-sub code is generated, but no field for the event/var is generated.
        // fields above expand to a handle field here too.
        self.event_handle.bar = &BAR_EVENT; // !!: figure this one out.
        self.var_handle.zap = &ZAP_VAR;

        // custom field
        self.custom: Vec<bool> = vec![];
        // self.not.this = vec![]; // compile error, no nested custom fields.

        // custom methods.
        fn custom(&mut self) {
            println!("{}", self.var.foo.get());
        }

        // UiNode methods, tag already implemented in `#[impl_ui_node]`, only snag is the init/deinit.
        #[UiNode]
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.child.event(ctx, update);

            if self.event.foo.on(update) {
                todo!()
            }

            if BAR_EVENT.on(update) {
                todo!()
            }
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            if let Some(foo) = self.var.foo.get_new() {
                self.custom();
            }

            if let Some(bar) = self.var.bar.get_new() {

            }

            if let Some(zap) = ZAP_VAR.get_new() {

            }
        }

        #[UiNode]
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            if self.var.foo.get() {
                frame.push_color(..);
            }
            self.child.render(ctx, frame);
        }
    }
}
```