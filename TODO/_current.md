* Merge.

* Fix grid without columns and rows.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

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

* Review `thread_local!` has a `const` variant that is more performant on init, can we do something similar
    for `app_local!` and `context_local!`?

* Review `AnyEvent` vs `Event` and `AnyVar` vs `Var`.
    - Now more methods are not generic.

* Review `WindowVars`, `ScrollContext` and any other "contextual widget service" like `ContextBorders` and `ZIndex`.
    - For window stuff can have an extension of `WINDOW`?
        - As a trait `WINDOW_Ext` or as a `struct FULL_WINDOW;` that deref to `WINDOW`.
        - This is the demonstration of how to extend services.
* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.