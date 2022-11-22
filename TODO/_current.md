* Improve `Transitionable`.
    -Targets of easing are limited right now, `BorderSides`, `SideOffsets` are not supported.
    - Maybe we want a more manual trait?

* Sort property build actions by importance?
    - Right now we just have one, `easing` but can be many.

* Rename all `Rc` prefixed stuff to `Arc`.
    - `RcVar`, `RcWidgetHandler`, etc.

* `is_hovered` in `window!` causes continuous activation/deactivation of when state.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?