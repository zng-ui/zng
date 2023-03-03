* Implement `trait Layout { fn layout(&self) -> T }` and `trait LayoutXY { fn layout(&self, for_x: bool) -> Px }`.
    - For every unit that currently has a `layout` function.
    - For every `Var<T: Layout>` using `Var::with` instead of cloning.

* Test all.
* Merge.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

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