* Review `WindowVars`, `SizePropertyLength` and any other "contextual widget service".
    - `WindowVars` are also returned by [`WINDOWS.vars`].
    - Implement `WINDOWS.with_context(id, f)` to run a closure in a `WINDOW` context.
    - But keep the `WINDOWS.vars` and `WINDOWS.widget_tree`, they are convenient?

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.
    - The units `fn layout(&self, metrics, default_closure)` method can also be improved.
        - Could make then use the contextual metrics and return `Option<T>::None` for `Default`.
        - The option is to reduce LLVM lines, `unwrap_or_default()` would be used in most cases.
        - Can't do that, default may be inside an expression.
            - Maybe have an alternative `layout_default`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Integrate `ThreadContext` with `rayon`.
    - Need to capture and load contexts for all `rayon::join` and `rayon::scope`.
    - See issue https://github.com/rayon-rs/rayon/issues/915
* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.