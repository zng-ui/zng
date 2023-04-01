* Refactor app `WINDOWS.open` to take in a future.
    - Requests need to be inserted back when not done in one poll?
        - No, need a new partial window thing, because we need the `WINDOW` context to run the future inside.
    - Focus requests on async windows also need to be refactored.

* Refactor `FONTS.register` to return a response var.
    - response vars can be plugged into an UI more easily then a raw future, and can be awaited just as easy as a future.

* Shorter name for `async_clone_move!`?
    * `ac_move!(foo, { })`.
    * `c_move!(foo, || { })`.
    OR
    * `clone_move!(foo, async { })`. // async
    * `clone_movd!(foo, || { })`. // closure

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