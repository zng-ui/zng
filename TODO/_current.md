* Window example: called `update_widget` for `close-dialog` without calling `update_inner` for the parent `WgtId(113)`.

* Implement `par_fold_reduce` for `BoxedUiNodeList`.
    - The problem is the custom accumulator `T`, need to smuggle this type pass the `dyn`.

* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - Implement `UiNodeList::info_all`?

* Parallel render.
    - Display list reuse ranges.
        - Right now `finish_reuse_range` returns a very light `ReuseRange` that has
          the indexes of start and end in the parallel list only, no way to update it after `parallel_fold`.
    - Implement in lists.
    - Update transforms of reused branches in parallel (`self_and_descendants_par`).
       
* Review capture-only properties.
    - They must return the child node and trace an error if used.
    - They must generate docs that explain # Capture Only