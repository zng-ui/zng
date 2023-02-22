* Rename `_IMPL`, `Impl` to `_SV`, `Service`.
    - Implement basic `service` proc-macro, test if derived code auto-completes in rust-analyzer.
    - Refactor manually impl services to use this.

* Refactor the update sender to an `app_local!` too?
    - It is the most common dependency of services.
    - For the user, `Vars`, `Events`, `Timers`, `Updates` all look like services.
        - Refactor `Timers` into a `TIMERS` service, to see if we like it.
            - Done.
        - After, do `Events` and `Vars`, no more need for context in many places.
            - `Vars::get` works in any thread, but `Vars::set` panics outside app threads.
                - Confusing, right now this is clear because `ctx.vars` needs to be borrowed.
            - Most apps are a single app per process, review `single_app` feature in `./Performance.md`.

    - Why stop there, we could have `static WINDOW: WindowContext` and `static WIDGET: WidgetContext`.
        - This actually changes things, plus causes cloning in `WidgetContextPath`.
        - `WINDOW.vars().title()` is more easy to use, and similar to `WINDOWS.vars({id}).title()`.
        - Lets try having a `WINDOW: ContextWindow`, with `id`, deref to `WindowVars` plus `WINDOWS` helpers like `close`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.