* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
    - Dropping app with service locked deadlocks.
        - See `test_font` for a case.
    - Review services `write` lock.
        - Some services could be sync and internally use different `app_local!` storage.
            - All `app_local!` services already need to be sync.
        - Specially cache services like `Fonts`.
            - Maybe work on caches to only read-lock initially too.
        - This is a current API `CONFIG.write().read(key)`.
            - Services where all mutable anyway so we used `&mut self` all over.
    - Make the update sender an `app_local!` too.
        - Most services want it.
        - Maybe not, we are using then to validate extensions not inited.

* Review all docs.
    - Mentions of threads in particular.