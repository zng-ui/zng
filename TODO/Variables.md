# Variables TODO

* Changes to enable [# Widget Property Transition]:
    - "Easing-map", a variable that eases between changes in a source variable, like a map that clones and transitions.
    - "Ease-switch", a `switch_var!` that eases between changes (probably just as good as an "easing-map").
    - Problem: dependent variables lazy update on read, with access only to `VarsRead`.
        - Move animation stuff to `VarsRead`, give private access for variables to create animations in `VarsRead` contexts.
    - Can call "easing-map" `chaser`
        - `Var::chaser(&self, duration: Duration, easing: fn(EasingTime) -> EasingStep) -> ChaserVar<T>`

* `Var::repeat` animation.

# Widget Property Transition

* How do we define a transition that gets applied to a widget's property?
    - Use a *fake* attribute, `#[ease(..)]`.
* How do we apply the ease attribute?
    - Use the `ChaserVar` on the `switch_var!` just before passing it to the property?
    - This forces us to call `IntoVar` early, like we do for the Inspector instrumentation, can use the same type validation?


## Transition Attribute

Attribute that configures the transition of a property changed by `when`.

### Usage

```rust
// in widget-decl
properties! {
    #[transition(150.ms(), easing::linear)]// ease applied in the when-generated switch_var!.
    background_color = colors::RED;

    #[transition(150.ms(), easing::linear)]// ease applied to all switch_vars of this property, (error is not all transitionable).
    margin = {
        #[transition(1.secs(), easing::expo)] // ease applied just to this witch_var!, replaces the outer one.
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    }

    when self.is_hovered { // properties switch with ease defined on-top.
        background_color = colors::GREEN;
        margin = 10;
    }
}

// in widget-new
foo! {
    #[transition(300.ms(), easing::linear)] // overwrites the ease for just this instance.
    background_color = colors::RED;

    #[no_transition] // disables ease forjust this instance.
    margin = 0;
}
```

### Name and Args

* Is `ease` a good name?
   - Maybe, but it expands to a `chaser` call, new users can get a bad intuition of `Var::ease` if they learn this attribute first.
   - `#[chaser(..)]` is not good, its cool name in `Var` close to the other animation methods, but here its something new that is
      not easy to guess controls animation.
   - `#[transition(..)]` is a more general term, its the "CSS" term that everyone knows, and we do use the trait `Transitionable`.
      - Slightly longer name, it does have a performance impact so maybe a full word is the Rust way.

* Args are the same as `Var::chaser`.
    - CSS has an extra "delay", investigate how this is implemented there, we could hack it with the easing curve, but it
        is best to use the `sleep` feature available in the raw animation.
    - Review other frameworks, not just CSS.


## Storyboard/Sequencer

A way to coordinate multiple animations together. Starting animations inside other animations already gives then all the same animation handle, need to implement some helpers.

* Animation blending, that is when two animations overlap in time, one affects the value less the other more across the
 overlap period. Not sure if this can be done automatically, maybe we need a sequencer builder that computes this stuff.

# Futures

* Variable futures don't use the waker context and don't provide any `subscriptions`, review this.
* Animation future does not wake once done and variables don't update also, causing it to hang until some other
      app update.

# GAT

* Implement specialized map var types when GATs are released.