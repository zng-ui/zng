# Variables TODO

* `Var::repeat` animation.

# Widget Property Easing

* How do we define a transition that gets applied to a widget's property?
    - Property attribute, `#[easing(..)]`.
    - Applies `Var::easing`.
    - Using a new widget builder API:
        - `push_property_build_action`.
        - Need to be implemented as `PropertyBuildAction`.
            - Can't be, need to box action.
        - Actually tricky to do because we need more bound restrictions (`Transitionable`).
            - If we can't figure out a way to downcast to `Transitionable` from `VarValue` we need something generated in properties to give `T`.
        - The build action is passed to the property, it gives the `T`.


## Usage

```rust
// in widget-decl
properties! {
    #[easing(150.ms(), linear)]// easing applied after the when_var is build, auto `use core::{units::TimeUnits, var::easing::*}`.
    background_color = colors::RED;

    #[easing(150.ms(), linear)]// ease applied to all of this property, (error is not all transitionable).
    margin = {
        #[easing(1.secs(), expo)] // ease applied just to this var? Need to implement property member direct access first
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    }

    when *#is_hovered { // color animates to green when this is `true`.
        background_color = colors::GREEN;
        margin = 10;
    }
}

// in widget-new
foo! {
    #[easing(300.ms(), linear)] // override the ease for just this instance.
    background_color = colors::RED;

    // use the inherited animation.
    margin = 0;
    // OR remove the animation.
    #[easing(unset)]
    margin = 0;
    // OR don't apply because there is no property instance.
    margin = unset!;
}
```

## Storyboard/Sequencer

A way to coordinate multiple animations together. Starting animations inside other animations already gives then all the same animation handle, need to implement some helpers.

* Animation blending, that is when two animations overlap in time, one affects the value less the other more across the
 overlap period. Not sure if this can be done automatically, maybe we need a sequencer builder that computes this stuff.

 # Read

 * Try to implement `Var::read(&self) -> VarReadLock<'_, T>`.
    - Have tried before, biggest problem was nested locks, like in `ContextVar<T>` and `ContextualizedVar<T, V>`.
    - Second biggest problem was the "type-erased" lock for `BoxedVar<T>`.
        - Without any kind of alloc, `Var::read` should be at worst just as efficient as `RwLock::read`.
    - Trying using "ouroboros" crate, generated a lot of code and the var type leaked into the lock type.

## Other Property Attributes

* Trace?