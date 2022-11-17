* Very slow text font size change.
    - See `text` example.
    - We stopped caching `font_kit::Font` because it is not `Send`.
      Now it loads for every use, probably the cause of the perf-hit.
* Merge.

* Refactor ContextVar to don't use RefCell and LocalKey?
    - Use `ContextLocal`, this causes context-var to be Send+Sync.
* Remove `Var::with`, make `Var::read(&self) -> VarReadLock<T>`.
    - More ergonomic, removes a boat load of LLVM lines.
* Use `app_local!` everywhere.
    - Same for `ContextValue<T>`.

* Review `task::respond_ctor`.
* Review `AppContextMut`.
* Use `ThreadContext` in `core::task`.
    - It is not just for UI threads?
* Continue "#Parallel UI" in `./Performance.md`.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?