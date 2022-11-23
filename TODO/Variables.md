# Variables TODO

* `Var::repeat` animation.

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

## Easing Attribute

* Conditional on when?
    - Right now can't instantly lit a button then ease-out on mouse leave.
    - In CSS we can set the "mouse-out" transition in the default selector and the "mouse-in" transition in the hover selector.

## Other Property Attributes

* Trace?