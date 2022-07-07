* Implement quad-tree updating.
   - Do update after the frame is send, not in finalize.
   - Maybe we can separate widgets that move a lot to a "dynamic" list, instead of rebuilding the quad-tree.
   - Almost every widget is moved in the `icon` example after every scroll, rebuilding the quad-tree from zero for every render_update is pretty bad.
      - Can we scroll the tree?
         - Not really, the logical hierarchy is not represented in the quad-tree.
      - Can we have different quad-trees at the widget level?
         - If every widget marked scrollable has a relative tree, the main tree can contains just the bounds of the inner tree?
            - We then check recursively.
         - Widget quad-trees have an offset.
      - Can we speedup the quad-tree in general?
         - Probably, we do a a lot of sparse allocation now, could use an arena.
   - Widget info events? Like WidgetMoved and WidgetVisibilityChanged.
      - Can help with the properties that try to track this too, like `is_visible`.
* Do hit-test in info, having to use IPC to hit-test is pretty bad and now we already have the quad-tree.
   - Review webrender hit-test, it looks like a linear tree walk?, they have 3 clip types, rectangle, rounded rectangle and polygon,
     all supporting transforms, code looks simple to adapt, maybe hardest part is tracking clip chains.
   - Expand `rendered` to have an index of render, so we can z-sort.
* Track what widgets are "definitely fully clipped".
* Track what widgets are close to becoming visible due to scrolling.
* Use `nearest_oriented` in `directional_from`.

* Add a "version" value in the WidgetInfoTree that increments for every render that updates any widget bounds.
   - Review this after widget events, we may be able to just use those.
   - Only update focus enabled_nav if this version changes.
      - Speedup the alt focus query, it is the slowest now.

* Icon example, holding ALT+Down for a bit and releasing causes the focus scroll to only go to one row above the focused item.
* Arrow key scroll in the panorama image is not as smooth as mouse move scroll.

* A frame is generated for the dummy pipeline just after respawn.
* Integrate frame reuse with frame update, see `Optimizations.md`.
* Finish state API, see `State.md`.