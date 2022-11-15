* New idea for contextual values, `DataContext`.
    - Good:
        - Encapsulate cleanup if used to implement events.
        - Can be made multi-thread.
        - Same impl for ContextVar, ContextValue, ContextualizedVar.
    - Bad:
        - Can't depend on the call stack to parent values?
        - If we create a new context for each context-var assign we can end-up with a large amount of contexts to check.
            - Can we mitigate this?
            - Why don't we have the context-var be a `DataContext`?
                - Because future `rayon::join` needs to explicitly call `DataContext::with_context` to propagate the context in
                  other threads.
            - Can we make the `DataContext` load all "var contexts" only for `rayon::join`.
                - Then most of the time, it is only a stack.
    - TODO:
        - Make it `Send+Sync` after it is shown to work.

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