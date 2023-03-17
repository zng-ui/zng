* Implement mouse click repeat.
    - Let widgets set their own mouse click mode?
    - WPF RepeatButton allows a config for initial delay + repeat interval.
    - Repeat flag can integrate with gesture click event that already has one from keyboard repeat.

* Parallelize windows?
    - Multiple window updates can happen in parallel.

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

* Parallelize app extensions?
    - The API is careful to not change the order of updates.
    - Maybe extensions can provide their own `Parallel` selection.
        - No if an extension depends on updating after the other the first extension could enable parallel and break this.
    - Maybe extensions can list their dependencies?
        - This requires dynamic code to create lists that must update linearly.
        - Right now we use generics in release builds to create zero-cost dispatch.
    - Do we have extensions that depend on running after others?
        - With multiple priority update methods maybe we don't need it.
        - Review this.

* Review if service locks are blocking parallel execution.
    - `FontFaceLoader::get_system` and `FontFace::load` are noticeable in release build traces.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Refactor `WidgetInfo` to own ref to the tree?
    - Places that used the `WidgetContextPath` can maybe use `WIDGET.item(&self) -> WidgetInfo`.
    - Can change `WINDOW.widget_tree` to returns the tree directly, only one place can panic.

* Review `LocalContext` in disconnected parallel tasks like `task::spawn`.
    - Need to capture the app only?
    - It causes the values to stay shared when going out-of-context in the widget that spawned.
    - Not a problem exactly.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Continue "#Parallel UI" in `./Performance.md`.

* Window without child does not open.
    - No layout request?

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.