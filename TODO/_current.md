* Font list task awaits for each font var to be "done", but the done future awaits `is_new`, and that only works per cycle of UI, but we
    are in a wait or spawn thread.

* Don't render text with empty font.
* Hold window load handle until all fonts requested on init load?
    - Holding a handle for each text seems to much.
* Markdown example, slow scroll wheel and a lazy error.
    - Error is new, not in master.

* Review !!: TODO.
* Test all.
* Merge.

* Refactor text shaping cache to avoid write locks.

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

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.