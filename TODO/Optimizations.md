# Parallel UI

* How much overhead needed to add `rayon` join support for UiNode methods?
    * Need to make every thing sync.
    * Vars are already *locked* for updates due to their delayed assign, is only reading from `Arc` slower then from `Rc`?
    * Event `stop_propagation` becomes indeterministic.
    * Services must become sync.
    * State must become sync!
* Maybe can always have an AppContext for each UI thread, with a copy of services and such, after each update they merge into
  the main AppContext.

# Build Time

* Use `do profile --build` to profile builds.
* 50% of our build time is in `LLVM_module_codegen_emit_obj`, this is probably code bloat from all the generics.
  - Use https://github.com/dtolnay/cargo-llvm-lines
* Implement and use `mono!` macro, described here https://internals.rust-lang.org/t/explicit-monomorphization-for-compilation-time-reduction/15907/30
* Implement `dyn_closure` for more types.


# Mouse Move Interest

* Let widgets define what sort of mouse event they want, use the hit-test tag, filter events in the view-process.

# Update Mask

* Sub-divide UiNodeList masks.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.

# Cache Everything

General idea, reuse computed data for `UiNode` info, layout and render at
widget boundaries if the widget or inner widgets did not request an update of these types.

## `UiNode::subscriptions` 

Easiest to do, can serve as a test for the others?

## `UiNode::info`

Could probably look the same as `subscriptions` but can an ego-tree be build from sub-trees?

To cache metadata we need to clone-it, `AnyMap` is not cloneable, could `Rc` the map.

## `UiNode::render`

Webrender needs to support this, check how they do `<iframe>`?

Has potential to use add megabytes of memory use, lots of repeating nested content, 
maybe we dynamically change what widget must cache based on use.

## Layout

Most difficult, can depend on context available size, font size, view-port size, can it be done?

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