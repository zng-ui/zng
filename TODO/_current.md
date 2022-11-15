* New idea for contextual values, `DataContext`.
    - Causes linear search of parent IDs.
    - Replace with something better.

* We are trying to solve 3 problems here:
    - `app_local!` that works like `thread_local!`, but across all UI threads and with the lifetime of the app.
        - This one can be solved using the `DataContext`, can call it `AppScope`.
        - The `AppLocalKey` is a map of `AppId`, and registers cleanup in `AppScope`.
    - ContextVar context ID for `ContextualizedVar`.
        - This is just a `ContextValue<ContextId>`?
    - Propagation of `ContextVar` and `ContextValue` across UI threads.
        - UI threads can be inserted at any point (`rayon::join`).
        - Can we do something with `ThreadId`?
        - Can we always load contexts in all UI threads.
            - No, we can use the same context var in multiple contexts in parallel.

* Refactor variables to use global lock.
    - CowVar can't use global lock.
    - ContextualizedVar, needs an outer RwLock.
    - The contextualized-var is used heavily and had no performance impact.
        - So the VarLock only really saves alloc space?
* Refactor ContextVar to don't use RefCell and LocalKey?
    - Try using the experimental `DataContext` to implement context-vars, contextualized-var and value.
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