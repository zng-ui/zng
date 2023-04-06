* Auto convert `impl UiNode` to `ArcNode` inside `WidgetGenerator`.
    - Maybe we just don't want tooltip to be a generator.
    - Generators are for property values that can end-up in more then one context.
        - We don't even call it `tooltip_gen`.
    - Try tooltip taking a node directly.
        - The node goes on an `ArcNode`.
        - Taken on layer init.

* Implement tooltip.
    - Initial show delay.
    - Show duration.
    - Between show delay.
    - Improve layer anchor position.
        - Follow cursor mode.
        - Cursor on init mode.
        - Define offsets using the stack idea of `place` and `origin` points.

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