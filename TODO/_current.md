* Test app_local event and commands.
    - Stack overflow in -t command.
    - Subtraction overflow in window example.
    - thread panicked while panicking. aborting in -t focus.
    - App scope unloaded before app drop (because it happens in `drop` impl).

* Review `ContextLocal`, default is not app-local?
* Review `CommandHandle`, can reconnect with different app?

* Update webrender to Fx107.

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