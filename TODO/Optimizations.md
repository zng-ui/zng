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

# Render

* Auto-virtualization of widgets, auto avoid rendering widgets that are not close to scroll borders.
* Very slow frame update for large text? Do a scroll-to-end in the example to see.

# Layout

* Add property for selecting the "sample" child in panels that need to measure.
* Implement "auto splitting/grid" for widgets with many children.
    - The `icon` example would be faster if the buttons where split into groups, this idea is to 
        do this splitting automatically internally, without asking the user.

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