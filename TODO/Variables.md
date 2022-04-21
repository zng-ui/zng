# Variables TODO

# Animation

* `Var::is_animating`.
    - Need better track of animation end, current implementation only updates on the next `is_new`.
    - Need to review filtered mapping vars, 
* Cancel animation if set from other source?
 * Idea:
   - An animation takes control of a var once in the first time it sets it.
   - The var can only be updated by it's current controller.
   - Any new animation takes control of the variable.
   - Manually setting the var removes control from any active animation.
 * Idea (extra):
   - Possibility of blending multiple animations(with different weights) into one? Not the default behavior.
* Config animation fps.

## Storyboard/Key-frames

Need to integrate with multiple animations stuff.

### Builder Style

```rust
/// Represents a timed sequence of operations.
#[derive(Clone)]
pub struct Sequence { }
impl Sequence {
    pub fn new() -> Self { }
    
    pub fn wait(self, duration: impl Into<Duration>) -> Self { }

    pub fn ease<T: TransitionValue>(self, var: impl Var<T>, new_value: impl Into<T>) -> Self { }

    /// Calls handler
    pub fn handler<H>(self, handler) -> Self { }
}
```

Not great, limited to methods we provide, but easy to implement time scale.

### Async Style

```rust
// like async_fn
animation!(var1, var2, |ctx, args| {
    ctx.wait(1.secs()).await;

    println!("Startup delay");

    let t = 2.secs();
    var1.ease(&ctx, 10, t, easing::cubic);
    var2.ease(&ctx, 10, t, easing::cubic).await;

    println!("Waited same time, all variables finished animating");

    let a = var1.ease(&ctx, 10, 2.secs(), easing::linear);
    let b = var2.ease(&ctx, 10, 1.secs(), easing::linear);

    task::all!(a, b).await;

    println!("Waited all animations");
})
```

Better, but maybe too powerful, need to optionally replay, change time scale, can we do this and still take real time in the var and timers?


### Async Loop

Can encode repeats too:

```rust
animation!(var1, var2, |ctx, args| {
    let mut next = 10;
    loop {
        var2.ease(&ctx, next, 500.ms(), easing::cubic).await;
        next = if next == 10 { 0 } else { 10 };
    }
})
```

Leaves only the time scale, maybe something in `args` to get the time?