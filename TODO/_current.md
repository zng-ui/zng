* Parallel info updates.
    - Refactor lookup validation to just avoid inserting the widget and logging an error.
    - Same validation in `parallel_fold`, except the widget is removed?
        - Nope, can't remove items from the tree.
            - Can implement remove, maybe its just a range skip right?
    - Implement `UiNodeList::info_all`.

* Implement `par_fold_reduce` for `BoxedUiNodeList`.
    - The problem is the custom accumulator `T`, need to smuggle this type pass the `dyn`.

* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.
      
* Review capture-only properties.
    - They must return the child node and trace an error if used.
    - They must generate docs that explain # Capture Only

* Review parallel render.
    - Recursive fold uses the `identity` function more then the core count.