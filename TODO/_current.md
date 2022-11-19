* Review "overridden" animations.
    - Do they keep running?
    - Right now vars ignore assigns depending on how recent the assigner is, but the animation keeps running.
        - We call this "importance"?
    - Let animations check if they are still having an effect?
        - Can't be automatic, animation can skip assigning any var as part of their behavior.
    - Expose the `AnimateModifyInfo` of vars?
* Review `AppContextMut`.
* Use `ThreadContext` in `core::task`.
    - It is not just for UI threads?
* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?