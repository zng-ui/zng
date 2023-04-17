* Layer fade-out.
    - Tooltip does not receive `on_layer_remove_requested` because of internal anon widget around the tip node.
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

* Fix widget docs.
    - Refactor Mix.
    - Fetched parent methods (need to cleanup associated functions and generate a Deref<Target=T> where Deref is linked).

    - Have a fancy tooltip that shows how to call the property in the macro.
        - Same style as the notable trait?

* Make more properties impl widgets (like all the base properties).
* Implement `TextMix<P>` or even more segmented mix-ins, use then in `Link` and other text widgets?