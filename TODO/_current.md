# Winit Update

* Try new `with_msg_hook` instead of injecting code to implement windows settings and Alt+F4 handling.

# Other

* Review easing animation the same value.
     - `ease_ne` causes weird effect animating `rgb(0.1, 0.1, 0.1)` to same value?
* Don't update enabled_nav if focus command have no enabled listeners.
    - This saves some compute, most apps don't have an indicator for these commands.
    - We already handle invalid calls so it will not cause an error.
    - The disabled call is different? (only activates highlight)

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only. 
* Finish state API, see `State.md`.