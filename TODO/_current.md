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
    - They are, and because the service is write-locked a single font loading can block all other already loaded accesses.
    - Need to refactor the service, multiple locks?
        - Everything is still locked if any font needs to load.
    - Firefox just locks a map too.
        - But font load is a "runnable".
    - If we change the font query to return a `ResponseVar`:
        - Will cause more updates as fonts load.
            - Can "batch" font loads, like all requests in the same update cycle load together?
        - Text node need to wait.
            - Layout can't return zero, but size will probably change.
            - Estimate monospace size?
        - Text nodes can hold a window load handle too, this avoids the user every seeing blank spots.
        - Font list is a list of response-vars?
        - Can support web fonts like this, with a download like image service.

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.