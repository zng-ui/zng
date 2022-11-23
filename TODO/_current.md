* Impl `Transitionable` for more units, needs review:
    - alignment
    - align.
    - byte.
    - grid.
    - line.
    - point.
    - rect.
    - resolution.
    - size.
    - text.
    - time.
    - transform.
    - vector.
* Impl `Transitionable` for composite types that have Transitionable parts.
    - BorderSides.
    - Refactor transitionable to not work only from += -= *= factor?
        - Makes no sense to impl ops::Add to border side just to add the colors.

* Sort property build actions by importance?
    - Right now we just have one, `easing` but can be many.

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