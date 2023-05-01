* Direct updates.
    - Implement delivery list and reuse invalidation in `WidgetLayout`, `WidgetInfoBuilder`, `FrameBuilder` and `FrameUpdate`.
        - We can just use the flags in `WIDGET` when it requests updates, no need to push the widget ID, just do
          the `UPDATES` requests once at the root widget.
        - This leaves the updates with ID to only the external sources, like subscriptions.
        - Actually, in the root widget use the root ID, this means the window impl don't need an special API, and we don't need to
          register search, the root path can be generated in the `WIDGET`.
    - How does info delivery list work for items that are just inited and not part of any info tree yet?
        - Info may need to keep both ways of tracking delivery.
        - Info also needs to be refactored in window?
            - Right now info is rebuild after any node OP that requests it (in `ctrl_in_ctx`).
            - We could keep this, document that info updates as soon as possible for windows.
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