# Events TODO

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