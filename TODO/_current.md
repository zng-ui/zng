* Dropping app with service locked deadlocks.
    - See `test_font` for a case.

* Refactor services into `app_local!` backed structs, with associated functions.
    - `IMAGES`
    - `WINDOWS`

* Refactor the update sender to an `app_local!` too?
    - It is the most common dependency of services.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.