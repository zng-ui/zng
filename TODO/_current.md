* Dropping app with service locked deadlocks.
    - See `test_font` for a case.

* Refactor services into `app_local!` backed structs, with associated functions.
    - Right now they are publicly `app_local!`, ideally they become a direct static instance with `&self` methods only
      that use internal `app_local!` stuff to communicate with app extensions.
    - Make services validate if their backing extension is actually running.

* Refactor the update sender to an `app_local!` too?
    - It is the most common dependency of services.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.