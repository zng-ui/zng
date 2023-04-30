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
    - [ ] Refactor layout invalidation to include an `WidgetUpdates` like list.
    - [ ] Refactor render invalidation to include an `WidgetUpdates` like list.
        - Render update too, but it can be upgraded to full render.
    - [ ] Refactor widgets to use new APIs.
    - Test all.
    - Merge.

* Finish test edit & selection.
    - No char event is emitted for tab?
    - Implement cursor position.
    - Implement selection.

* Implement localization.
    - Similar to `CONFIG`, maybe even backed by it.
    - Review localization standard formats.
        - Translators use an app to edit?