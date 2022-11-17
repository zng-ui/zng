* Remove `Var::with`, make `Var::read(&self) -> VarReadLock<T>`.
    - More ergonomic, removes a boat load of LLVM lines.
    - Need to return an enum of various types of borrow that deref T?
* Use `app_local!` everywhere.
    - Events.
    - Remove `APP_ON_EXITED`.
* Review `ContextLocal`, default is not app-local?

* Review `AppContextMut`.
* Use `ThreadContext` in `core::task`.
    - It is not just for UI threads?
* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?