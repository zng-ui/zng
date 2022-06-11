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
  - Can we use the inspector collected metadata to batch generate tracing spans after the frame is send?