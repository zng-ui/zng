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
    - gleam uses a `Rc<dyn Gl>` for the OpenGL functions.
    - There are obscure bugs with sending OpenGL contexts across threads, maybe review using `surfman` again.

# Code Bloat

* The main crate generates a massive amount of "llvm-lines".

# Const

* Use `const` in our function and methods.
  - Wait until clippy has a lint for this?

# Vars

* `merge_var!` calls the closure for each input that updated in the same cycle.
  - This is needed because the merge_var can be the input of another binding, and bindings need to 
    update as soon as possible to preserve order of call, see test `binding_update_order`.

# Low Memory

* Implement `LowMemory` event for desktop.
  - Winit implements for mobile.
  - Firefox implements "memory pressure" for desktop, we can based on that.