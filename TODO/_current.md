* Implement unblocking image render.
    - Test it in animation example.
* Implement unblocking icon example loading.

* Review if service locks are blocking parallel execution.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `LocalContext` in disconnected parallel tasks like `task::spawn`.
    - Need to capture the app only?
    - It causes the values to stay shared when going out-of-context in the widget that spawned.
    - Not a problem exactly.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Continue "#Parallel UI" in `./Performance.md`.

* Window without child does not open.
    - No layout request?

* `config` example with two windows sometimes does not update the other window.
    - Try removing save file.

* Review all docs.
    - Mentions of threads in particular.