# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.

# Render

* Modify display list to not include glyphs and text color in the same item.
  - Allows reuse of glyphs.
* See if we can improve perf for reused render.
  - In the icon example, the render does not enter buttons, but just the reuse of each button is noticeable in the trace.
    - Compared with targeted updates & events.
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
    * Need to make everything `Send + Sync`.
    * `VarValue: Send + Sync` too?
      - Yes, `RwLock` requires it.
    * Can we have a global lock in `Vars`?
      - Every var that can MODIFY checks this lock.
      - `Vars` exclusive locks to modify?
        - Need to clone to modify, can clone on `get_mut`, then exclusive lock to update the value.
        - Can have a `set`, that avoids cloning for replacement.
        - Can just have modify closure take a `&mut Cow`.
        - Can implement this first, to remove the RefCell?
          - Need to be a `RwLock` from the start, a shared `RefCell` needs to be placed in a thread-local, because `get` does not requests `&Vars`.
          - **Can** use this to get a sense of the perf impact of locks, smaller refactor than making everything Send+Sync.
            - **TODO**
            - Refactor `Var::modify` to work using `&mut Cow<T>`.
            - Refactor vars to not use `RefCell`, but use `UnsafeCell` and shared var lock.
              - Lock only for value?
              - Hooks need mut access to register.
```rust
pub mod shared_var_lock {
    use std::cell::UnsafeCell;

    use parking_lot::RwLock;

    use super::VarValue;

    static VAR_LOCK: RwLock<()> = RwLock::new(());

    pub struct VarMutex<T: VarValue> {
        value: UnsafeCell<T>,
    }

    impl<T: VarValue> VarMutex<T> {
        pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
            let _lock = VAR_LOCK.read();
            f(unsafe { &*self.value.get() })
        }
    }
}

```
      - Can remove var buffer, listener does the same thing.
        - Can get from any thread actually.
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