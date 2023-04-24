* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - Implement `UiNodeList::info_all`?

* Parallel render.
    - Implement `ParallelBuilder<FrameBuilder>`.
    - Update transforms of reused branches in parallel (`self_and_descendants_par`).
       