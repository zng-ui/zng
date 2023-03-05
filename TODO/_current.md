* Implement `UiNodeList::par_each` and `UiNodeList::par_each_mut`.
    - For `par_each` items must be `Sync`.
        - Because `par_iter` requires it.
        - `Vec<BoxedUiNode>` can't implement it?
        - `par_iter_mut` only requires `Send`.
            - Can't do parallel in `info`, `measure` and `render`+`render_update`?
            - Measure is the worst, `layout` can.
        - Can make `PanelList` use an internal mutex.
    - Closure cannot return `bool` to interrupt, there is not such feature in rayon's `for_each`.
    - Closure must be `Fn(usize, NODE)`, not `FnMut` as-well.
    - Final design: 
        - `fn par_each_mut(&mut self, f: impl Fn(usize, &mut BoxedUiNode) + Send + Sync)`.
            - Implemented correctly in all.
        - `fn par_each(&self, f: impl Fn(usize, &BoxedUiNode) + Send + Sync)`.
            - Redirects to `for_each` in `Vec<BoxedUiNode>`.
            - In `PanelList`, other wrappers redirects to `par_each_mut` acquired by lock.
                - In `UiNodeVec` this is a problem because we currently deref to `Vec<BoxedUiNode>`.
                    - So we can't have a `Mutex<Vec<..>>`.
            - Can we constrain `par_each` to `Self: Sync`.
                - No, we want to use `par_each` in default impl.
    - Lists can redirect to `for_each` if node count is low?

* Implement parallel image render.
    - Test it in animation example.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `ThreadContext` in disconnected parallel tasks like `task::spawn`.

* Continue "#Parallel UI" in `./Performance.md`.

* Review all docs.
    - Mentions of threads in particular.