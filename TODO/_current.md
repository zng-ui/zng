* In the icon example, the render does not enter buttons, but just the reuse of each button is noticeable in the trace.
    - Compared with targeted updates & events.
    - If the icons are sub-divided in group widgets the performance is better?
      - Manually chunking buttons optimizes render from `4.7ms` to `1.1ms` average. 
    - How to chunk automatically, we can't really generate new widgets as that messes with the user expectation?
        - One of the benefits of chunking is faster event delivery, if we don't generate actual widgets do we need to add "chunk" to WidgetPath?
    - No, this feature generates widgets, but the user knows/can generate widget ids.
        - API entry in `UiNodeList::chunked(FnMut() -> WidgetId)`?
            - Can't be in UiNodeList, or we need all UiNode methods in UiNodeList, otherwise panels layout will not chunk?
    - Auto chunks are logical sub-divisions?
        - Layout may group in a different way.
        - Can't move widget between chunks, so we can end-up with worst performance because items are in the global out-of-bounds list
            even if they are inside the actual panel area, just because the panel layout from bottom to top.
    - What we really need is for item widgets to participate in parent layout as an "inline run".
        - `wrap!` chunked with `wrap_run!`, causes the items to wrap as if inside `wrap!` but is just the `wrap_run!` delegating layout
            to parent.
        - This helps with text runs to, all "inline" style layout.
        - API entry in `UiNode::children_for_each(&mut self)`?
            - To open?
        - How does layout is communicated?
            - API entry is `UiNode::inline_measure` and `UiNode::inline_lauout`?
            - No, need to implement a different layout.
        - Can just be signals passed in the normal measure/layout?
            - The "first line advance" can be a constrain.
            - The "last line advance" can be something in `WidgetLayout`?
                - Need something in `measure` too. 

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?