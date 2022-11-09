* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.
* Implement all `todo!` code.