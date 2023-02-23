* Refactor `Vars` to `VARS` service.
* Refactor `Updates` to `UPDATES` service?
* Integrate `ThreadContext` with `rayon`.
    - Need to capture and load contexts for all `rayon::join` and `rayon::scope`.
    - See issue https://github.com/rayon-rs/rayon/issues/915
* Review `EventSender` and `VarSender`.

* Implement a `WINDOW` context local with window stuff?

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.