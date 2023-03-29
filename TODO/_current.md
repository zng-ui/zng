* Parallel layout for more panels.
    - `wrap!`.
        - Can't measure in parallel, mutated row, constrains of each item affected by previous item.
        - Can sort bidi in parallel? Yes, but right now we reuse heap work memory `bidi_levels` and others.
        - Layout builds rows again? Right now can't be parallel because of this, maybe we can review the wrap layout after
          "Review ugly layout API".

* Parallel info updates.
    - How to share the `&mut WidgetInfoBuilder`?
    - No `UiNodeList::info_all`?

* Parallel render.
    - Widgets.
        - How to share `&mut FrameBuilder` and `&mut FrameUpdate`?

* Review if service locks are blocking parallel execution.
    - `FontFaceLoader::get_system` and `FontFace::load` are noticeable in release build traces.
    - Refactor `FONTS.list/find` to only use read lock, until cache miss, then retry with write lock.
    - Allow multiple fonts to load at the same time, somehow.
        - Could return `ResponseVar<Font>` for get and `ResponseVar<FontList>`.
            - The font list one updates multiple times as fonts in the list load.
            - How does `text!` layout work when there is not font loaded yet?
                - Also we can end-up using more resources shaping text for fallback fonts that are only used one frame.
                - Can just await the full `FontList` result, we just want to unblock the UI threads.
            - Implement a `FONTS.default_list()` that returns a `FontFaceList` with just a default fallback font?

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.