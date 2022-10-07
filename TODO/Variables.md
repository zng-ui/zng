# Variables TODO

* `Var::repeat` animation.

# Widget Property Transition

* How do we define a transition that gets applied to a widget's property?
    - Use a *fake* attribute, `#[easing(..)]`.
* How do we apply the ease attribute?
    - Use the `Var::ease` on the `when_var!` just before passing it to the property?
    - This forces us to call `IntoVar` early, like we do for the Inspector instrumentation, can use the same type validation?
    - Also causes contextualization early.


## Transition Attribute

Attribute that configures the transition of a property changed by `when`.

### Usage

```rust
// in widget-decl
properties! {
    #[transition(150.ms(), easing::linear)]// ease applied in the when-generated when_var!.
    background_color = colors::RED;

    #[transition(150.ms(), easing::linear)]// ease applied to all when_vars of this property, (error is not all transitionable).
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