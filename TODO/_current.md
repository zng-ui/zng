* Review capture-only properties.
  - [x] Use a different doc tag.
  - [x] Generate a doc sections that explains capture only.
        - Remove manual docs that mention the same.
  - [x] Review signature, no reason to have the child node, if we did not have it users can't even write wrong code.
        - Can be surprising for property writers maybe, because `child: impl UiNode` becomes an input.
        - Better surprise them that are already interacting with the `property` attribute then surprise a property function user?
  - [x] Update property capture docs.
  - [ ] Test all.


* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

