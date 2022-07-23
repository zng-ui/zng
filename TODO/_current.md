* Implement virtualization, at least "auto-virtualization" on the level of widgets to automatically avoid rendering widgets that are not close
to scroll borders.

* Icon example, directional nav wraps around if the next item up is fully clipped, instead of scrolling.
    - Can we make the focus nav know that the focused target will be scrolled to?

* Integrate frame reuse with frame update, see `Optimizations.md`.
* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Finish state API, see `State.md`.