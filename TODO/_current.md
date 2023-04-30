* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

* Finish test edit & selection.
    - No char event is emitted for tab?
* Implement localization.