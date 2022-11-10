* Implement when assign for `UiNode`, `UiNodeList` and `WidgetHandler`.
    - Use `WhenUiNodeBuilder` and `WhenUiNodeListBuilder`.
    - Implement `AnyWhenWidgetHandler`.
        - Right now we don't have `AnyWidgetHandler`, the `PropertyArgs::widget_handler` returns `&dyn Any`.
* Implement all `todo!` code.