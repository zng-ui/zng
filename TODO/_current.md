* `match_node` refactor.
    - Review big layout widget nodes with methods.
        - Refactor to be a struct operated by the matcher?
        - Helps with rust-analyzer.

* Implement `on(_pre)_node_op` event properties for widgets.
    - Use the `Discriminant<UiNodeOp>` in the args, plus the count.
    - Implement `on_*` properties for each op. 

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - No `UiNodeList::info_all`?

* Parallel render.
    - Widgets.
        - How to share `&mut FrameBuilder` and `&mut FrameUpdate`?



* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.