* Implement virtualization, at least "auto-virtualization" on the level of widgets to automatically avoid rendering widgets that are not close
to scroll borders.

* Icon example, holding ALT+Down for a bit and releasing causes the focus scroll to only go to one row above the focused item.
* Arrow key scroll in the panorama image is not as smooth as mouse move scroll.

* Integrate frame reuse with frame update, see `Optimizations.md`.
* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Finish state API, see `State.md`.