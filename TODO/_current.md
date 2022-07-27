# Screen culling

* Compute final bounds and transform for each culled widget.
  - This is used to auto-scroll to widget and probably other functions.

* Can "cull" be "auto_hide"?
  - Hidden widgets should be updating transform anyway.
  - We want culled widgets to be focusable, so that they auto-scroll on focus, right now hidden widgets are not focusable.
      - Sites usually have an accessibility link "Skip to Content" that only becomes visible on focus, in CSS
        they have to trick the browser by positioning the element out-of-bounds if not focused.
      - We can implement this explicitly, allow focus on hidden?

# Other

* Fix `skip_render`, right now it does not update transform+bounds for hidden vis.
    - Differentiate from collapse?
* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only. 
* Finish state API, see `State.md`.