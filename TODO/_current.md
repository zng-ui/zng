* Refactor property priority into constants: `#[property(CONTEXT)]`.
    - Is the word priority wrong?
    - Why not call it NestGroup?

* Review property & widget macro docs.
    - Property does not generate mod anymore.
        - Wait for priority const refactor.
    - Widget publishes include and build.

* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Rename `toggle::selection` to `toggle::selector` or rename `Selector` to `Selection`.
* Merge `Property.new/new_when` into a single constructor that uses some kind of provider interface.
    - Like the nightly Any provider.

* Test property generics `value::<bool> = true; when *#is_something { value::<u32> = 32; }`.
* Review `IntoVar` and `IntoVarValue` constrains, we don't need then to be debug/clone anymore?
* Const errors don't show if  there is a compile error, so the when `!foo::ALLOWED_IN_WHEN_EXPR` does not show.
    - Generate a placeholder `__w_0__`?

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.
* Implement all `todo!` code.