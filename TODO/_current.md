* Move caret animation to ResolvedText.

* Refactor animate sleep tracking, to allow refactoring AnimationArgs to be an Rc, to allow real `Var::modify` animation.
    - Using clone for now, after merge refactor this.

* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
* Implement all `todo!` code.