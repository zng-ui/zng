# Mixed-Space

* Track what widgets are fully contained by their parent inner-bounds.
* Use the logical tree-filter iterator to find most hits.
* Chain in a linear search on all out-of-bounds widgets of the tree.

## Perf

Even with the mostly linear search needed in the `icon` example, the overall performance was faster
than the cost of building the quad-tree, and the hit-test it self faster than webrender, but slower than the quad-tree.

Other examples that sub-divide the items more show much better performance than the quad-tree hit-test.

# Other

* Implement virtualization, at least "auto-virtualization" on the level of widgets to automatically avoid rendering widgets that are not close
to scroll borders.

* Icon example, directional nav wraps around if the next item up is fully clipped, instead of scrolling.
    - Can we make the focus nav know that the focused target will be scrolled to?

* Integrate frame reuse with frame update, see `Optimizations.md`.
* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Finish state API, see `State.md`.