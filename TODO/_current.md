* Use `par_each`, test performance.
    - Trying it in `icon` example now.
    - Actually loses performance and texts don't init right.
    - Fix bugs caused by parallel, context vars not propagating correctly?
    - Try having parallel just in the main collection so only chunks are parallel.

* Implement parallel image render.
    - Test it in animation example.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Implement tracing parent propagation in `ThreadContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.