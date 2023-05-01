* Direct updates.
    - Remove old update tracking in WIDGET.
        - `take_layout` and others.
    - Implement delivery list and reuse invalidation in `WidgetLayout`, `WidgetInfoBuilder`, `FrameBuilder` and `FrameUpdate`.
    - How does info delivery list work for items that are just inited and not part of any info tree yet?
        - Info may need to keep both ways of tracking delivery.
        - Info also needs to be refactored in window?
            - Right now info is rebuild after any node OP that requests it (in `ctrl_in_ctx`).
            - We could keep this, document that info updates as soon as possible for windows.
    - Test all.
    - Merge.

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