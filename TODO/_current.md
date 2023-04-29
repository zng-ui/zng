* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.
      
* Review capture-only properties.
    - They must return the child node and trace an error if used.
    - They must generate docs that explain # Capture Only
