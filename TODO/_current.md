* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
        - See Rayon parallel extend first, maybe there is a way to parallel-collect the tree (it is backed by a vector).
    - Implement `UiNodeList::info_all`?

* Parallel render.
    - How to share `&mut FrameBuilder` and `&mut FrameUpdate`?
    - Need to preserve display list order too.
    - Not for updates, can just have multiple `FrameUpdate` instances that are aggregated in the end.
        - Right now the `FrameUpdate` memory is reused, still want this.
        - Rayon has a parallel extend, see how it works. 