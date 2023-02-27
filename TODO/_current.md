* Refactor `Updates` to `UPDATES` service?
    - Refactor all contexts into `context_local!` values.
    - Remove `TestWidgetContext`, can just use an app.
    - Figure out `ContextWidgetPath`, how to build without alloc?
    - Figure out a way to dynamically link a custom `context_local!` to load together with `WIDGET` and `WINDOW`.
        - If possible then we can fully remove `StateMap`.
    - Refactor `WidgetInfo` to own ref to the tree?
        - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
        - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `UPDATES.update_ext()` usage from `task::spawn`.
    - We can't always send an update request because it will cause a double update in the normal blocking update usage.
    - What happens if the app is sleeping when the other thread makes the request?

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.
    - The units `fn layout(&self, metrics, default_closure)` method can also be improved.
        - Could make then use the contextual metrics and return `Option<T>::None` for `Default`.
        - The option is to reduce LLVM lines, `unwrap_or_default()` would be used in most cases.
        - Can't do that, default may be inside an expression.
            - Maybe have an alternative `layout_default`.

* Integrate `ThreadContext` with `rayon`.
    - Need to capture and load contexts for all `rayon::join` and `rayon::scope`.
    - See issue https://github.com/rayon-rs/rayon/issues/915
* Review `EventSender` and `VarSender`.
* Review `AnyEvent` vs `Event` and `AnyVar` vs `Var`.
    - Now more methods are not generic.

* Implement a `WINDOW` context local with window stuff?
* Review `ScrollContext` and any other "contextual widget service" like `ContextBorders` and `ZIndex`.
* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.