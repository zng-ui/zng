* Parallelize windows?
    - Parallel event handling.
    - Parallelization configurable like `Parallel`.
        - But for window, `UPDATE`, `INFO`, `INIT` and `DEINIT` are  the same?
        - `ParallelWin { UPDATE, EVENT, LAYOUT, RENDER }`.

* Parallelize more methods.
    - `info`: how to share the `&mut WidgetInfoBuilder`?
        - Or can we build in parallel and then merge?
    - `measure` and `layout`: needs to be done in panels, also how to parallelize access to mutable associated data in `PanelList`?
        - Also how to share the `&mut WidgetLayout`?
    - `event` and `update`: do we need these?
        - For rare broadcast events?
        - How to share the distribution list?
    - `render`: how to share the frame builder?
        - Can we build partial frames and merge?
    - `render_update`: do we need it?
        - We don't have an example that generates massive updates, but it is possible.
        - Also review if we avoid sending updates for culled widgets, we should avoid doing that.
        - To implement parallel we can just have multiple update builders and merge then?
            - Simpler than merging other builders.
            - Just need to figure out how to reuse then, right now we reuse alloc between updates.

* Review if service locks are blocking parallel execution.
    - `FontFaceLoader::get_system` and `FontFace::load` are noticeable in release build traces.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Continue "#Parallel UI" in `./Performance.md`.

* Window without child does not open.
    - No layout request?

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.