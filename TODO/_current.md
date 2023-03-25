* Review boxed `par_each_fold`, can we avoid the lock?
    - All panel widgets will end-up with a boxed list because they are captured from properties.

* Parallel layout for more panels.
    - `stack!`.
        - Detect `StackDirection` full horizontal/vertical and implement parallel only for these?
        - Could do one parallel layout pass and one offset pass.
            - Don't need to store anything, can use the bounds info?
            - No, may contain nodes that are not full widgets.
    - `grid!`.
        - Can make column/row parallel, but is usually a small number and very simple widgets.
        - Cells is more difficult, measure wants to write to the column/row they are on, measure is skipped in some cases too.
        - Layout can be parallel, need to implement `par_each_mut` with associated data in `PanelList`.
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

* Review ugly layout API.
    - Stuff like `LAYOUT.with_inline_measure(|| multiple nested LAYOUT methods)`.

* Review all other `for_each_mut` and `fot` usages, replace with parallel when possible.

* Implement tracing parent propagation in `LocalContext`?
    - https://github.com/wagnerf42/diam/blob/main/src/adaptors/log.rs

* Negative space clips not applied when only `render_update` moves then into view.
    - In "icon" example, set `background_color` for each chunk and scroll using only the keyboard to see.

* Review all docs.
    - Mentions of threads in particular.