* Image `block_window_load` not working right.
    - Loading deadline should be per handle clone, right now the shortest timeout invalidate all other handles.

* Use loading handle in the window icon.

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.