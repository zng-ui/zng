# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Need to make every thing sync.
    * Vars are already *locked* for updates due to their delayed assign, is only reading from `Arc` slower then from `Rc`?
    * Event `propagation` becomes indeterministic.
    * Services must become sync.
    * State must become sync!
* Maybe can always have an AppContext for each UI thread, with a copy of services and such, after each update they merge into
  the main AppContext.

# Update Mask

* Sub-divide UiNodeList masks.
* Review the icon example with all icon to see what happens with lots of false positives.
  - Maybe we need a better distribution or a linear search after the flag matches?
  - Can we have delivery lists for vars?
* Move update skipping optimization to context, right now the `Widget` implementer handles some parts of it.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.

# Better render reuse

* Already started implementing this, see `ReuseGroup`.
  - Webrender reuse depends on space/clip ids and these invalidate all items after an insert remove.
  - Worst, we can't cache creation of space/clips because it will just mess-up the id count for all subsequent items.
  - Maybe we should stop using the webrender display list.
    - If we had more access to the display list internals we could save by ranges for each widget, then send range refs to
      the previous display list bytes that is retained in the view-process.
  - Investigate why the space/clip ids are generated in the client side.
    - Its so you can push items to parent spaces out-of-order, like position absolute?

* Optimize using the icon example with fully loaded icons.
  - Display list building is slow.
  - Webrender rendering is very slow!, Firefox is much better, are we missing on some culling in Firefox code?

# Webrender frame update

* Very slow frame update for large text? Do a scroll-to-end in the example to see.

# Layout

* Add property for selecting the "sample" child in panels that need to measure.

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

# Debug Profiler

* Tracing is very slow, investigate how to fix.
  - Comparison of release mode with/out "inspector" and profiler shows that a 7ms frame turns into a 13ms frame.
  - Some of it is due to all the boxing enabled by "inspector.
  - Can we use the inspector collected metadata to batch generate tracing spans after the frame is send?