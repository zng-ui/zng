# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.

# Render

* Modify display list to not include glyphs and text color in the same item.
  - Allows reuse of glyphs.
* Modify webrender to not (de)serialize it's display list.
  - Measure first, the DisplayList::build step is a bit slow, but it may not be due to the iteration.
  - If we do this, need to figure out how we will still apply patches from Firefox.

# Layout

* Add property for selecting the "sample" child in panels that need to measure.

# Image Render

* Try reusing renderer.

# View Open

* Try to detect unsupported render mode without glutin.
* Try to implement async context creation in default view crate.
    - Problem, glutin needs the event-loop window target to build a context (it is not send and must be in main).
      - glutin-v2 will not need this?
    - gleam uses a `Rc<dyn Gl>` for the OpenGL functions.
    - There are obscure bugs with sending OpenGL contexts across threads, maybe review using `surfman` again.

# Tracing

* The `tracing` trace is pretty slow, maybe because we need to allocate debug string for each entry.
  - Already offloading everything possible to another thread.
* Try `minitrace-rust` see if it is faster/more accurate than `tracing`.
  - Or some other alternative crate?
  - Browsers collect trace by ID, ideally our "ID" would be a static str but the tracing API does not allow it.

# Code Bloat

* The main crate generates a massive amount of "llvm-lines".
* The final executables are pretty big as well.
* Probably a lot of type name strings?

# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Event `propagation` becomes indeterministic?
      - Could make event notification linear, most nodes are not visited.
    * Services must become sync.
      - Turn services into command channels?
    * State must become sync!
      - No more mutable references?
        - Use concurrent map.
        - No entry API.
      - Could keep the &mut for widgets, forcing each widget to be in a single thread.
        - No parallel event handlers in this case.

* What we want to enable:
  - Allow background init of UI.
    - The icon example stops responding on the first init/info/layout/render.
    - This will require more than Send+Sync UI, the var update and services expect the entire app to work lock-step.
      - If the background UI causes a var update it is observed in the entire app.
      - The background UI also needs to observe app updates.
      - Does not sound like a full background, more of a delayed render?
  - Better performance when many UI items need to compute, if layout is invalidated for many widgets we want to use rayon join to work
    in multiple branches of an ui_list at the same time.
    - Rayon join done at the widget level, `par_for_each`.
    - Init, update, event just works.
    - Layout just works?
    - Render needs "nested display list", to avoid double alloc (insert).
    - Info needs double alloc, one for the partial tree in a thread branch, other for when it is inserted in the actual tree.
      - Probably not an issue.

* Review `AppContextMut`.
* Use `ThreadContext` in `core::task`.
    - It is not just for UI threads?

* Rayon:
  - We can run the app in a rayon thread-pool made for the app.
  - Rayon automatically uses the thread-pool it is in.
  - But we don't load the `ThreadContext` for rayon internal jobs.
    - Maybe we can use a trace API to inject the context, see https://github.com/rayon-rs/rayon/issues/915.
    - We could have an parallel iter adapter that loads the context? see https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs
  - So `core::task` spawned from the app end-up running in the UI threads?
    - The idea was to avoid blocking the UI at all costs, need a different thread-pool for `task`?

* How we can start:
  - We can implement parallel var and event hooks.
  - And parallel var modify + animations.
  - Just need to figure out the thread pool.