* Implement tooltip.
    - Initial show delay.
    - Show duration.
    - Between show delay.
    - Improve layer anchor position.
        - Follow cursor mode.
        - Cursor on init mode.
        - Define offsets using the stack idea of `place` and `origin` points.

* Finish  `with_inline_visual` and `inline = true`.

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