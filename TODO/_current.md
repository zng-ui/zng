* Refactor variables to use global lock.
    - CowVar can't use global lock.
    - ContextualizedVar, needs an outer RwLock.
    - The contextualized-var is used heavily and had no performance impact.
        - So the VarLock only really saves alloc space?
* Refactor ContextVar to don't use RefCell and LocalKey?
    - How does parallel context var works?
        - Use https://crates.io/crates/execution-context ?
        - Using `flow_local` for every thing also solves or cleanup problem.
* Review if all lock usages are as free of deadlock as the VarLock impl.
    - Mostly don't hold exclusive lock calling closures (modify and hooks).
* Make `VarValue: Send + Sync`.
* Make `AnyVar: Send + Sync`.
* Merge

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?