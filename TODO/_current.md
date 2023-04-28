* `inspector::show_hit_test` renders incorrect lazy bounds.
    - This does not affect the actual hit-test.
    - But, the `show_hit_test` is founding then at (0, 0) offset, review this.

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

* Review *widgets* that use `into_widget`.
    - `view(..)`.
    - Refactor then to an standard widget.
    - Generics can be work around using BoxedUiNode for `view`.