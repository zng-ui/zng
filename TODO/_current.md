* Direct updates.
    - [x] Design API in `WIDGET`.
    - [x] Design API in `UPDATES`.
            - Missing `_when` overloads.
    - [x] Design API in var.
            - Missing `subscribe_when`.
    - [ ] Design API in event.
            - Missing `subscribe_when`.
    - [x] Implement `_when` var.
    - [ ] Refactor info invalidation to include an `WidgetUpdates` like list.
        - Right now we check for info updates after every node OP?
        - In `ctrl_in_ctx`, need to move to update only.
        - Need to still support the old update flag because of new inited widgets (they are not in the info tree searched by the delivery list).
    - [ ] Refactor layout invalidation to include an `WidgetUpdates` like list.
        - Need to move the `WidgetUpdates` list to the `WidgetLayout`.
        - Can't have a lifetime in `WidgetLayout` cause of `par_fold_reduce`.
        - Have window request an `Arc<WidgetUpdates>`, the service then takes it from the `&mut WidgetUpdates` for the duration of the call.
        - Right now it is `&WidgetUpdates`.
    - [ ] Refactor render invalidation to include an `WidgetUpdates` like list.
        - Render update too, but it can be upgraded to full render.
    - [ ] Refactor widgets to use new APIs.
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