* Implement window persistence using config.
    - Started in `save_state` property.
    - Need to implement a way to delay the window open.
        - This is useful for other things too.
    - Don't let windows reopen outside the monitor area.

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.