* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only. 

# Screen culling

- Firefox has a "VisibleRect" in the display builder, if is set to the parent scroll viewport, it gets clipped by the root viewport
  and then expanded by `"layout.framevisibility.numscrollportheights"`, in the directions it can scroll.
- By default expands to 1 viewport height, and 0 width.
- The "frame visibility" only updates after `"layout.framevisibility.amountscrollbeforeupdatevertical"` (default=vis-rect/2) scrolls, 
  so the display list does not get rebuild for each new element getting in range.
- So basically:
  - Have a "effective visible rect" of viewport clipped and inflated by 1.vwh in the directions is can scroll.
  - Only update the "effective visible status" of a widget when has scrolled an amount (half the viewport height?)

* Fix `skip_render`, right now it does not update transform+bounds for hidden vis.
    - Differentiate from collapse?
* Culled is the same as `Visibility::Hidden`, ensure everything behaves like this.

## Requirements

* Avoid flooding the display-list with too many items.
* Avoid turning too many scroll render-updates into full renders.
    - Update by "pages", Firefox does this?
* Cull before widget can be reused.
    - Outer-bounds (push_widget), not inner-bounds (push_inner).
* Still compute final bounds and transform for each culled widget.
    - This is used to auto-scroll to widget and probably other functions.
* Can "cull" be "auto_hide"?
    - Hidden widgets should be updating transform anyway.
    - We want culled widgets to be focusable, so that they auto-scroll on focus, right now hidden widgets are not focusable.
        - Sites usually have an accessibility link "Skip to Content" that only becomes visible on focus, in CSS
          they have to trick the browser by positioning the element out-of-bounds if not focused.
        - We can implement this explicitly, allow focus on hidden?

# Other

* Finish state API, see `State.md`.