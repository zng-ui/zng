* Review layered nodes.
    - No way to remove then (no widget ID).
    - `ArcNode` widgets can also become not-widgets after being inserted.
    - `tooltip = Tip!()` causes a slot node to be left behind every time the tip reopens
      for the same widget when it is still closing.

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