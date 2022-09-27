# Var/Update Rewrite

* Get `merge_var!` and `when_var!` contextualized.
* Implement "specialization" of mapping vars by returning boxed.
* Implement animation in new var API.
* Review every old var API, do we really don't need `switch_var!` and `map_ref`? 
* Implement delivery-list for update requests.
    - Use it in new var API.
    - Add the current updates list to `UiNode::update` ?
* Implement `ui_node!`, see [#Better Node Macro].
    - Remove `UiNode::subscriptions`.
    - Remove old var, rename `var2`, rewrite everything.
    - Tests.
* Stop propagation when all items in delivery list visited.
* Stop propagation when it is requested.

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
        self.child = child;

        // declare var fields, vars auto-subscribe.
        self.var.foo = foo.into_var();
        self.var.bar = bar.into_var();
        // declar event fields, events auto-subscribe.
        self.event.foo = FOO_EVENT;

        // declare only handle field, auto-sub code is generated, but no field for the event/var is generated.
        // fields above expand to a handle field here too.
        self.event_handle.bar = &BAR_EVENT;
        self.var_handle.zap = &ZAP_VAR;

        // custom field
        self.custom = vec![];
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

    // ---

    struct NodeVars<V_foo, V_bar> {
        foo: V_foo,
        bar: V_bar,
    }
    struct NodeEvents<E_foo> {
        foo: E_foo
    }
    struct Node<C, V, E, C_custom> {
        child,
        var: V,
        event: E,
        custom: C_custom
    }

    Node {
        child,
        var: Vars {
            foo: foo.into_var(),
            bar: bar.into_var(),
        },
        event: NodeEvents {
            foo: FOO_EVENT
        },
        custom: vec![],
    }
    #[impl_ui_node(child)]
    impl<V> Node<V> {
        fn custom(&mut self) {
            println!("{}", self.var.foo.get());
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