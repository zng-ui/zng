* Initial `INTERACTIVITY_CHANGED_EVENT` interactivity event generates so many events that it triggers the infinite loop detection code.
    - We need the initial event cause `is_enabled` starts as `false`?
    - Icon example blocks all buttons when the overlay opens, this also causes a massive number of events that have worst performance then
        the previous way using the WIDGET_INFO_CHANGED_EVENT.
    - We did gain perf for disabling single widgets, as only that widget path is targeted now.
    - Maybe change the `InteractivityChangedArgs` to contain a reference of the two trees.
        - The interactivity is pre-checked to create the delivery list.
        - But each listener "checks" again, by needing to provide the widget-id in methods.
        - The trees cache the interactivity so this might be better.
        - Users can "stop_propagation" of this grouped event, maybe weird, maybe not.

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.