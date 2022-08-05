* Review "fatal" errors in ConfigBackend.
    - We want the `last_error` to don't show subsequent errors after the panic?

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.