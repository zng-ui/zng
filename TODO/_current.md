* Refactor ContextVar to don't use RefCell and LocalKey?
    - Use `ContextLocal`, this causes context-var to be Send+Sync.
* Make `VarValue: Send + Sync`.
* Make `AnyVar: Send + Sync`.
* Remove `Var::with`, make `Var::read(&self) -> VarReadLock<T>`.
    - More ergonomic, removes a boat load of LLVM lines.

* Can't change only Var to be Send+Sync.
    - Image related vars contaminate the entire UiNode because they can have "render" closure that output UiNode and must be send.

* Merge.

* Use `app_local!` everywhere.
    - Same for `ContextValue<T>`.
* Use `ThreadContext` in `core::task`.
    - It is not just for UI threads?

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?