* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Merge `Property.new/new_when` into a single constructor that uses some kind of provider interface.
    - Like the nightly Any provider.

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.
* Implement all `todo!` code.