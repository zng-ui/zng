* Cursor position for tooltip is lagging to far behind.
    - Observed this in Firefox too, maybe there is nothing to do?
    - Make a system call to get the cursor position and compare it with the MOUSE var, just to see if we are not lagging more then "normal".
* Layer fade-out.
    - A property in the layered widget that sets a state that is used by `LAYERS.remove` to animate a fade-out?
    - May want other "exit" animations.
    - We implemented this manually in window example, maybe check that out first.

* Review `Transitionable::chase`, not needed anymore?
* Review Dip units used in computed values.
    - Things like `MOUSE.position` are pretty useless without the scale_factor.
    - Real problem is having to retrieve the scale factor?

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