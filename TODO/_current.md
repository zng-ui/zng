* Direct updates.
    - Review/refactor window update flags.
        - `layout_requested`, `pending_render`.
        - `WIDGET` now already requests all pending updates for the root widget and
           the other widgets are updated by a flag internal to `WIDGET` too.
        - So we may be able to just check `enter_window` in the delivery list?
            - Window requests some updates itself, right now this is a request with `None` target and setting the window flag.
            - Could change to a request to the `WindowId` directly?
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