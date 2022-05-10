# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Need to make every thing sync.
    * Vars are already *locked* for updates due to their delayed assign, is only reading from `Arc` slower then from `Rc`?
    * Event `stop_propagation` becomes indeterministic.
    * Services must become sync.
    * State must become sync!
* Maybe can always have an AppContext for each UI thread, with a copy of services and such, after each update they merge into
  the main AppContext.

# Mouse Move Interest

* Let widgets define what sort of mouse event they want, use the hit-test tag, filter events in the view-process?

# Update Mask

* Sub-divide UiNodeList masks.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.

# Cache Everything

General idea, reuse computed data for `UiNode` info, layout and render at
widget boundaries if the widget or inner widgets did not request an update of these types.

## `UiNode::measure` and `UiNode::arrange`

* Already started implementing this, see `LayoutMask`.

## `UiNode::render`

Webrender needs to support this? Can we implement our own display list? If so, we can record the inserted range of display list,
keep the old display list around, then copy from it to the new display list. Maybe even have the ranges expand in the view-process?

* See `DisplayListBuilder::start_item_group`, `finish_item_group` and `push_reuse_items`.
* `set_cache_size` is a part of this too? Yes needs to be set to the count of item groups.
* Does not allow nesting, can we auto-split nested items?

If each widget here is a "reuse item", we can auto generate WR keys like:
  - widget_0 = start key0
    -child_1 = end key0, start key1
      -leaf3 = end key1, start key2
      -leaf4 = end key2, start key3
     child_1 = end key4, start key5 // child_1 added more items after content.
    -child_2 = end key6, start key7
      -leaf5 = end key8, start key9

We can store the key range for each widget, if it did not invalidate render it can generate all keys and push reuse:
- widget_0 = key0..=key9
-  child_1 = key0..=key5
-  child_2 = key6..=key9
-    leaf3 = key1..=key2

* Keys are `u16` are we generating to many keys?
  - If we hit max `65535` we cause a full frame rebuild?
  - If we hit max in a single frame, just stop caching for the rest, the window is probably exploding anyway.
* If keys are just ranges, how to update unchanged items after a remove?
  - If we insert a new leaf after lead3 what key will it get?

# Image Render

* Try reusing renderer.

# View Open

* Try to detect unsupported render mode without glutin.
* Try to implement async context creation in default view crate.
    - Problem, glutin needs the event-loop window target to build a context (it is not send and must be in main).
    - Can use `build_raw_context` that only requires a window handle, so we create the winit window blocking then offload
      everything to a thread.
    - gleam uses a `Rc<dyn Gl>` for the OpenGL functions.
    - There are obscure bugs with sending OpenGL contexts across threads, maybe review using `surfman` again.