* Merge.
* Implement a way to delay the window open.
    - Use the new function in the `save_config` property.
    - Add an optional usage of this in the `image!` widget.

* Don't let windows open outside the monitor area.
    - This actually causes a bug if it is very out of the area.

* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.