* Review usage of "$crate" in widget macros, is this the reason rust-analyzer does not work in widgets?
    - We already improved a bit by setting the span for the `#[widget({args})]` to `call_site`, but after
      passing it to the other macros this is lost?
    - We need recreate the current bug in the test crate, to be sure.
    - If we can't really cause rust-analyzer to work, maybe we can implement a fake `__widget_macro` that materializes some
      code for `is_rust_analyzer` only, that causes the property value expressions to be interactive.

# Other

* Review easing animation the same value.
     - `ease_ne` causes weird effect animating `rgb(0.1, 0.1, 0.1)` to same value?

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only. 
* Finish state API, see `State.md`.