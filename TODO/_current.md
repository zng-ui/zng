* Implement inline layout.
    - Implement *inline constrains*.
        - Advance and line height?
    - Use `NodeLayout` in nodes.
        - As the output of `UiNode::measure` and `UiNode::layout`.
        - Most nodes just converts to block.
        - Implement inline layout for `text!` and `wrap!`.
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