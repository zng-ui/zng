* Layer fade-out.
    - Implement `layer_remove_delay` using `on_layer_remove_requested`.
    - Implement `is_layer_removing`.
        - Using both delay and this flag a fade-out effect can be easily implemented 
          just by setting properties with `#[easing(..)]` an a `when` condition.

* Review `Transitionable::chase`, not needed anymore?
* Review Dip units used in computed values.
    - Things like `MOUSE.position` are pretty useless without the scale_factor.
    - Real problem is having to retrieve the scale factor?

* External layout and render updates.
    - Work the same way as normal updates, with the `WidgetUpdates` list, but in the layout and render cycle.
    - Use this to implement special subscriptions that automatically layout/render a widget, saving an update
      cycle.

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - No `UiNodeList::info_all`?

* Parallel render.
    - Widgets.
        - How to share `&mut FrameBuilder` and `&mut FrameUpdate`?

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.

# Continue Widget Refactor

* Link to fetched `WidgetBase` does not work in `Button`.

* Review `Link` widget.
    - Link should be a `Button` style?
        - Like `Toggle` has different styles.
    - Or it should be a clickable `Text`.