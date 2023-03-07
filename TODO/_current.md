* Parallel init causes text to use wrong font. 
    - Context read wrong value.
    - See `issue.md`.
* Parallel init slower then linear in `icon` example.
    - Fine tuning parallel to only work in the icon chunks still slower.
    - Single thread average init of `icon!` is 0.1ms. Parallel average is 0.8ms.
            - Stuck in locks?
            - Fix bugs first.
    - Try implementing `context_local!` to use a `thread_local!` fast path?

* Implement parallel image render.
    - Test it in animation example.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.
    - Need to capture the app only?

* Implement tracing parent propagation in `ThreadContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.