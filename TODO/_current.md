* Review proc-macros in rust-analyzer.
    - any_all, OK.
    - derive_service, FIXED.
    - expr_var, OK.
    - hex_color, OK.
    - impl_ui_node, OK.
    - lang, FIXED.
    - merge_var, ?
    - property, ?
    - static_list, ?
    - when_var, ?
    - widget, ?
    - widget_new, ?

# Other

* Review easing animation the same value.
     - `ease_ne` causes weird effect animating `rgb(0.1, 0.1, 0.1)` to same value?

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only. 
* Finish state API, see `State.md`.