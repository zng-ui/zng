# CORE

* Crash respawn deadlocking.

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