* Implement some placeholder error value for properties that cannot be used in `when` expr.
    - Like a "value var" of `UiNodeInWhenExprError` is used if the user reference an UiNode.
* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
    - Use `WhenUiNodeBuilder` and `WhenUiNodeListBuilder`.
    - Implement `WhenRcWidgetHandler`.
* Implement all `todo!` code.