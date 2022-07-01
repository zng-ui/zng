* Update webrender.
* Add a "version" value in the WidgetInfoTree that increments for every render that updates any widget bounds.
   - Only update focus enabled_nav if this version changes.

* Implement a quad-tree in the info-tree to speedup spatial queries.
   - Track what widgets are "definitely not fully clipped".
   - Can probably just use a fixed grid?

* Icon example, holding ALT+Down for a bit and releasing causes the focus scroll to only go to one row above the focused item.
* Arrow key scroll in the panorama image is now as smooth as mouse move scroll.

* A frame is generated for the dummy pipeline just after respawn.
* Integrate frame reuse with frame update, see `Optimizations.md`.