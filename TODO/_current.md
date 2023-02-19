* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
    - Dropping app with service locked deadlocks.
        - See `test_font` for a case.

* Review all docs.
    - Mentions of threads in particular.