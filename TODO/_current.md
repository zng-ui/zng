* Implement window loading handle.
    - Add `Windows::loading_handle(WindowId) -> Option<Handle>`.
    - Add `is_loaded` var.
        - Events too?
    - Implement loading handle timeout.

* Use loading handle the `save_config` property.
* Implement optional loading handle usage of this in the `image!` widget.

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.