* Refactor `Vars` to `VARS` service.
* Refactor `Updates` to `UPDATES` service?
* Integrate `ThreadContext` with `rayon`.
    - Need to capture and load contexts for all `rayon::join` and `rayon::scope`.
    - See issue https://github.com/rayon-rs/rayon/issues/915
* Review `EventSender` and `VarSender`.
* Review `AnyEvent` vs `Event` and `AnyVar` vs `Var`.
    - Now more methods are not generic.

* Implement a `WINDOW` context local with window stuff?
* Review `ScrollContext` and any other "contextual widget service"

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.