* Icon example fails hit-tests after fast scroll.
    - Need to scroll to end, then back to middle.
    - Issue present with `parallel=false`.
    - Issue already present before parallel rewrite.

* Implement `par_fold_reduce` for `BoxedUiNodeList`.
    - The problem is the custom accumulator `T`, need to smuggle this type pass the `dyn`.

* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.
      
* Review capture-only properties.
    - They must return the child node and trace an error if used.
    - They must generate docs that explain # Capture Only

* Review parallel node operations.
    - Recursive fold uses the `identity` function more then the core count.

* Review `into_widget` and functions that use it.
    - It is an *anonymous* widget, looks weird in inspector.