* Duplicate widget module docs on the macro when expanding by rust-analyzer.
    - This should enable widget docs on hover.
    - Do the same for the property struct.
* Fix inline align, see `./Layout.md`. 

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
* Review all docs.
    - Mentions of threads in particular.