# Compile Time

* Refactor code to stop using TypeId as unique identifiers.
  - Context var: `struct FooVar: bool = true;` becomes `static FOO_VAR: ContextVar<bool> = ContextVar::new_unique(|| true);`.
    - On first read the default value is inited and an `unique_id!` generated to represent the var.
    - `ContextVar<T>` is a `Var<T>` struct, no more trait, no more `context_var!`.
    - Get becomes `FOO_VAR.get(vars)`.
  - State key: `struct FooKey: u32` becomes `static FOO_KEY: StateKey<u32> = StateKey::new_unique()`.
    - On first usage initializes to an `unique_id!`.
    - `StateKey<u32>` is a struct and is `Copy`.

# Update Mask

* Review the icon example with all icon to see what happens with lots of false positives.
  - Maybe we need a better distribution or a linear search after the flag matches?
  - Can we have delivery lists for vars?
* Move update skipping optimization to context, right now the `Widget` implementer handles some parts of it.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.

# Render

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

# Inspector

* Icon example changing the icon font, deinit + instantiate + init~>render all icon buttons:
  - debug + "inspector" (default) changes in 18s!
  - release-lto + "inspector" changes in 4s!
  - release-lto + "dyn_widget" (default) changes in 320ms. 
  - release-lto + "dyn_node" changes in 525ms.

This is are some rough timings, done while some other stuff was happening, but they indicate that the "inspector" nodes
are pretty slow.

Need to optimize the inspector nodes a bit, maybe some lazy stuff?

# Profiler

* Try `tracing-chrometrace`, see if it is faster than `profile/util`.

# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Need to make everything Send+Sync.
    * Vars are already *locked* for updates due to their delayed assign, is only reading from `Arc` slower then from `Rc`?
    * Event `propagation` becomes indeterministic.
    * Services must become sync.
    * State must become sync!
* Maybe can always have an AppContext for each UI thread, with a copy of services and such, after each update they merge into
  the main AppContext.

* If nodes where at least `Send` we could init~>layout a large tree in a background thread, then swap it in, for very large trees
   the initial cycle is the slowest, after only the parts the user interacts with change.