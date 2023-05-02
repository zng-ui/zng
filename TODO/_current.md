* Direct updates.
    - !!: TODOs
    - Test all.
    - Merge.

* Make `WindowCtx` and `WidgetCtx` mut, avoid mutex.
    - I think this are not requested mut because some node OPs uses to not be mut.
* Use `Atomic<T>` in places we use `Mutex<Copy>` or `RwLock<Copy>`.
    - `WidgetCtxData`.

* Finish test edit & selection.
    - No char event is emitted for tab?
    - Implement cursor position.
    - Implement selection.

* Implement localization.
    - Similar to `CONFIG`, maybe even backed by it.
    - Review localization standard formats.
        - Translators use an app to edit?