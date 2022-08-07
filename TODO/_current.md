* Implement window loading handle.
    - Implement loading handle timeout.
        - Needs a timer.
* Implement loading handle for headless.
* Is a window loading again after respawn?

* Use loading handle the `save_config` property.
* Implement optional loading handle usage of this in the `image!` widget.
* Use loading handle in the window icon.

* Use the `Deadline` type in all timer functions.

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.