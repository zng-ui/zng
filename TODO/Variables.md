# Variables TODO

# Animation

* Variable that starts animating on first `get`.
* `Var::repeat`.
* Fps per animation.
* Animation *sleep* until time, like a timer.
* Default FPS from monitor.

```rust
trait Var {
    /// Create a function that applies a easing transition to this variable.
    fn ease_fn<N, F>(&self, new_value: N, easing: F) -> Result<Box<dyn Fn(EasingStep)>, VarIsReadOnly>;
}

impl CompositeAnimation {
    /// Add an animation that plays from `start` to `end` of the composite animation.
    /// 
    /// The `animation` closure is called with a normalized step [0..=1] when the composite
    /// animation is in the [start..=end] range.
    pub fn with(mut self, animation: Box<dyn Fn(EasingStep)>, start: Factor, end: Factor) -> Self {
        todo!()
    }

    pub fn play(self, vars: &Vars, duration: Duration, easing: F) {
        vars.animate(move |vars, args| {
            let step = anim.elapsed_stop(duration);
            for a in &self.animations {
                if a.range.contains(step) {
                    a.function(step - a.range.start)
                }
            }
        })
    }
}
```

* How to Blend
    - Can we just blend the EasingSteps?
        - No, the easing function can set anything.
    - How to cross-fade two values?
        - Its just a `set_ease` of sorts, set_ease(output1, output2).
        - Can we have a `Vars::with_blend(range)` animations inside get blended?
            - Needs to affect the animation ID to have multiple at the same time.
            - Better only support this in a composite animation?

        - Need to annotate each animation fn too so the contextual blender knows how to cross-fade.
            - Not if we only allow it in composite animation.

```rust
impl<T> Blender<T> {
    /// Calls to `Var::modify` by animations in the blend range are redirect here.
    pub fn push_modify(&mut self, modify: ModifyFn, weight: Factor) {
        todo!()
    }

    /// During var apply updates.
    pub fn apply(mut self, vars: &Vars) {
        let value = self.var.get_clone(vars);

        let mut values = Vec::with_capacity(self.modifies.len());
        for (modify, weight) in self.modifies {
            let mut value = value.clone();
            if modify(&mut value) {
                values.push((value, weight))
            }
        }

        if !values.is_empty() {
            let mut value = value;
            for (value, weight) in values {

            }

            self.var.set(vars, value);
        }
    }
}
```

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

* There are multiple animations IDs here.
* Possibility of blending multiple animations(with different weights) into one.

# Widget Property Transition

* How do we define a transition that gets applied to an widget's property?

# Futures

* Variable futures don't use the waker context and don't provide any `subscriptions`, review how is this working in `WidgetTask` tasks.