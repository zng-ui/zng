# Variables TODO



# Animation

We can just schedule an easing with Vars and it sets the variable until its done.

```rust
trait Var<T> {
    fn set<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
        where
        Vw: WithVars,
        N: Into<T>;
    fn set_ne<Vw, N>(&self, vars: &Vw, new_value: N) -> Result<(), VarIsReadOnly>
        where
        Vw: WithVars,
        N: Into<T>
        T: PartialEq;

    // new

    /// Animate from the current value to the `new_value` using the `easing_fn` function that defines a curve.
    fn ease<Vw, N, D, E>(&self, vars: &Vw, new_value: N, duration: D, easing_fn: E) -> Result<(), VarIsReadOnly>
        where
        Vw: WithVars,
        N: Into<T>
        D: Into<Duration>,
        E: Fn(EasingTime) -> EasingStep,
        T: TransitionValue;

    /// Animate from the current value to the `new_value`, but only if the current value is not equal to `new_value`.
    fn ease_ne<Vw, N, D, E>(&self, vars: &Vw, new_value: N, duration: D, easing_fn: E) -> Result<(), VarIsReadOnly>
        where
        Vw: WithVars,
        N: Into<T>
        D: Into<Duration>,
        E: Fn(EasingTime) -> EasingStep,
        T: TransitionValue + PartialEq;

    /// Set the value imediatly to `new_value` then animate to `then`.
    fn set_ease<Vw, N, M, D, E>(&self, vars: &Vw, new_value: N, then: M, duration: D, easing_fn: E) -> Result<(), VarIsReadOnly>
        where
        Vw: WithVars,
        N: Into<T>
        E: Fn(EasingTime) -> EasingStep
        T: TransitionValue;

    /// Animate  throught multiple *key-frames* offset (can have a macro like gradient `stops!` here)
    fn ease_keys<Vw, N, D, E>(&self, vars: &Vw, values: Vec<(Factor, T)>, duration: D, easing_fn: E) -> Result<(), VarIsReadOnly>
        where
        Vw: WithVars,
        E: Fn(EasingTime) -> EasingStep
        T: TransitionValue;
}
```

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