# Variables TODO

# Animation

* Config animations fps.
* Timed assigns, like setting to specific value after a delay, with `is_animating` flagged while it waits.
    - Can work with values that are not animatable.
* Variable that starts animating on first `get`.
* Modify animate (just called `animate`?).

## Storyboard/Sequencer

A way to coordinate multiple animations together, most open design is to just use an async handler:

```rust
// like async_hn
animation!(var1, var2, |ctx, args| {
    ctx.wait(1.secs()).await;

    println!("Startup delay");

    let t = 2.secs();
    var1.ease(&ctx, 10, t, easing::cubic);
    var2.ease(&ctx, 10, t, easing::cubic);
    ctx.wait(t).await;

    println!("Waited same time, all variables finished animating");

    var1.ease(&ctx, 10, 2.secs(), easing::linear);
    var2.ease(&ctx, 10, 1.secs(), easing::linear);

    task::all!(var1.wait_animation(&ctx), var2.wait_animation(&ctx)).await;

    println!("Waited all animations, different times");
})
```

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

But its so generic, how do we have any external control over it:

* Can we adjust the time scale?
* Possibility of blending multiple animations(with different weights) into one?
* 

# Widget Property Transition

* How do we define a transition that gets applied to an widget's property?

# Futures

* Variable futures don't use the waker context and don't provide any `subscriptions`, review how is this working in `WidgetTask` tasks.