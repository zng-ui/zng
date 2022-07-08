* Directional navigation is disabling when focused is disabled.

* Use `nearest_oriented` in `directional_from`.
* Speedup the alt focus query, it is the slowest now.

* Rethink spatial partitioning, the quad-tree is a bad fit for the problem:
   - Most widgets are fully contained by the parent bounds.
   - Most panel widgets can naturally calculate a fixed grid that perfectly fits its content, for all items that are not transformed.
   - We can use a sparse spatial hash map for all widgets that don't fit in the parent.
   - Spatial queries then can them be mostly based on the logical tree structure, with only the weird transformed widgets needing to be updated
     in the hash map.
   - The sparse spatial hash map is much faster to update then the quad-tree, the downside is that it is a grid not a tree, so a large widget can
     be inserted many times, but if we restrict it to only widgets that are transformed out of the expected bounds, those tend to be smaller in number and size.


* Do hit-test in info, having to use IPC to hit-test is pretty bad and now we already have the quad-tree.
   - Review webrender hit-test, it looks like a linear tree walk?, they have 3 clip types, rectangle, rounded rectangle and polygon,
     all supporting transforms, code looks simple to adapt, maybe hardest part is tracking clip chains.
   - Expand `rendered` to have an index of render, so we can z-sort.

* Track what widgets are "definitely fully clipped".
* Track what widgets are close to becoming visible due to scrolling.

* Icon example, holding ALT+Down for a bit and releasing causes the focus scroll to only go to one row above the focused item.
* Arrow key scroll in the panorama image is not as smooth as mouse move scroll.

* Integrate frame reuse with frame update, see `Optimizations.md`.
* Finish state API, see `State.md`.