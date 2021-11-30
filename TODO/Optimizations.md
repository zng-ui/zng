# Build Time

* Very slow build time in release mode without `dyn_widget` (window example up-to 18 minutes compile time and 25GB memory usage).
    Might be related to https://github.com/rust-lang/rust/issues/75992

# Mouse Move Interest

* Let widgets define what sort of mouse event they want, use the hit-test tag, filter events in the view-process.

# Update Mask

* Each update source, like vars, are assigned a bin 0 to 255.
* Each update has a 255 bit mask for bins that were updated.
* Each? widget also has a 255 bit mask of all update sources descendent nodes are interested in.
* The widget can then eliminate a call to update for most cases when the update does not affect its content.

```rust
#[impl_ui_node(child)]
impl<C: UiNode> UiNode for MyNode<C> {
      fn subscriptions(&mut self, ctx: &mut WidgetContext, interest: &mut WidgetInterest) {
            interest.var(&self.var0);
            interest.var(&self.var1);

            interest.event(KeyInputEvent);

            self.child.interest(interest);
      }

      fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(true) = self.var0.copy_new(ctx) {
                  self.var1 = var("new!");     
                  ctx.updates.subscriptions();
            } else let Some(new) = if self.var1.get_new(ctx) {
                  println!("{}", new);
            }
      }
}
```

# Separate Meta Creation from Frame

```rust
trait UiNode {
      fn info(&self, ctx: &mut RenderContext, frame: &mut WidgetInfoBuilder);
}
```

# Implement Event Matcher Macro

```rust
fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
      event_match! {
            pre/*|pos*/ args => self.child.event(ctx, args),
            KeyInputEvent => {

            },
            MouseMoveEvent => {

            },
      }
}
```

Expands to:

```rust
fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
      if let Some(args) = KeyInputEvent.update(args) {
             self.child.event(ctx, args); // pre
             {

             }
             // self.child.event(ctx, args); // pos
      } else {
             self.child.event(ctx, args);
      }
}
```

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.