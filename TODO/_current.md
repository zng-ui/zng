* `container! { sticky_width = true; padding = 10; }` does not work, expands width in every layout.
    - Caused by `with_sub_size` always adding the "removed" size to min.
    - Need to always remove it from min?
    - Will still cause problems with zero min?
* Review PxConstrains::min in every panel, should be zero? 
* Fix inline align, see `./Layout.md`. 

* Rename `ui_list!` to `ui_vec!`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.
* Review all docs.
    - Mentions of threads in particular.