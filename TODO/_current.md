* Implement inline layout.
    - Use `NodeArea` in nodes.
        - As the output of `UiNode::measure` and `UiNode::layout`.
            - Tried implementing this, very noisy, most nodes right now inflate/deflate the inner size, they
                all now need to handle the inline value.
        - Try to implement inline offsets as a contextual value?
            - Or maybe as a WidgetNodeContext style accessor.
* Implement `LayoutDirection` for `flow!`.
* Use nested `wrap!` to chunk the icon example as a demo of the performance benefits of logical subdivision.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?