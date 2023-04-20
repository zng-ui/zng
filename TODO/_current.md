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

* Review order of properties in docs.
    - Inner module impl are placed first in the docs.
    - Even if the module is declared after the main impl.
    - Right now we have less important properties defined in inner modules.
    - Some reorder can be done with mix-ins, but some properties don't make cense as mix-ins.
    - See `Scroll` widget for an example.

* Review all docs.
    - Mentions of threads in particular.

* Direct layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.