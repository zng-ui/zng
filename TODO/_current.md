* Layer fade-out.
    - Fix panic when hovering a tooltip widget within `tooltip_interval`.

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - No `UiNodeList::info_all`?

* Parallel render.
    - Widgets.
        - How to share `&mut FrameBuilder` and `&mut FrameUpdate`?

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.
    - The color should show only in between items of each row, empty space in between rows.
    - The first chunk already does not have correct clips and it is entirely inside the un-culled area.
    - Clips still work for smaller children, only the big background fill is bugged.
        - Only if each child is only affected by a single clip.

* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.