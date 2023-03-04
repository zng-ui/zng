* Integrate `ThreadContext` with `rayon`.
    - Need to capture and load contexts for all `rayon::join` and `rayon::scope`.
    - No rayon API for this yet, see issue https://github.com/rayon-rs/rayon/issues/915
    - Implement an extension to parallel iterators that manages the context:
        - See https://github.com/wagnerf42/diam/blob/3da688d14020508e555762800ea0121a0d0ca78b/src/adaptors/log.rs
        - Print thread IDs of a rayon for_each run to find points where we need to load the context.


* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.