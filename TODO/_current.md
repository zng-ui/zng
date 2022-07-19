# Hit Test

* Implement auto_hit_test for clip & space.
  - Need to support render_update.

* Webrender keeps all clips in a single array, each item references a range of clips that affect it, can we do the same for hit-test?
  - This avoids needing to search for the child widget in every parent.
  - How do we update this vec, rebuild from scratch with reuse copy?
  - Each hit-test item references a clip range and is associated with the widget.
  - If we are visiting everything to reuse ranges, can we just set a z-index too?
  - How do we deal with transformed clips?

* Review how parent hit-test clips affect children.
* An HitTestItem::Child item is inserted for each child, even is in most cases we are not making special clips for each child.
* Test everything.
* Merge.

* Track what widgets are close to becoming visible due to scrolling.

# Quad-Tree

* Avoid quad-tree for small amount of items.
    - Implement linear search of inner-bounds for less then 8 or 16?
* Rethink spatial partitioning, the quad-tree is a bad fit for the problem:
   - Most widgets are fully contained by the parent bounds.
   - Most panel widgets can naturally calculate a fixed grid that perfectly fits its content, for all items that are not transformed.
   - We can use a sparse spatial hash map for all widgets that don't fit in the parent.
   - Spatial queries then can them be mostly based on the logical tree structure, with only the weird transformed widgets needing to be updated
     in the hash map.
   - The sparse spatial hash map is much faster to update then the quad-tree, the downside is that it is a grid not a tree, so a large widget can
     be inserted many times, but if we restrict it to only widgets that are transformed out of the expected bounds, those tend to be smaller in number and size.

# Other

* Icon example, holding ALT+Down for a bit and releasing causes the focus scroll to only go to one row above the focused item.
* Arrow key scroll in the panorama image is not as smooth as mouse move scroll.

* Integrate frame reuse with frame update, see `Optimizations.md`.
* Use something like the `FastTransform` from webrender internals in our own transforms.
* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Finish state API, see `State.md`.
* Don't generate scrollbars when they are not a part of `mode`.