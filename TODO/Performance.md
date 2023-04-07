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

# Tracing

* The `tracing` trace is pretty slow, maybe because we need to allocate debug string for each entry.
  - Already offloading everything possible to another thread.
* Try `minitrace-rust` see if it is faster/more accurate than `tracing`.
  - Or some other alternative crate?
  - Browsers collect trace by ID, ideally our "ID" would be a static str but the tracing API does not allow it.

# Code Bloat

* The main crate generates a massive amount of "llvm-lines".

# Const

* Use `const` in our function and methods.
  - Wait until clippy has a lint for this?