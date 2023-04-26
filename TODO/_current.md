* Parallel render.
    - `render_index` not compatible with parallel render.
        - We used to count on the fact that each widget is rendered in order, now this is only true
          in each parallel segment.
        - 
    - Update transforms of reused branches in parallel (`self_and_descendants_par`).


* Implement `par_fold_reduce` for `BoxedUiNodeList`.
    - The problem is the custom accumulator `T`, need to smuggle this type pass the `dyn`.

* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - Implement `UiNodeList::info_all`?

       
* Review capture-only properties.
    - They must return the child node and trace an error if used.
    - They must generate docs that explain # Capture Only

* Review parallel render.
    - Recursive fold uses the `identity` function more then the core count.