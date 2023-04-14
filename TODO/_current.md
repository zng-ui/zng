# Widget & Property Refactor

* Docs not generated for nested mix-ins. 
* Rename `properties!` to `widget_set!`.
    - Move property syntax docs to this too.
    - Reference this when talking about the generated widget macro syntax.
* Better error for `defaults` and `widget_set!`.
    - Match to compile_error when the first tokens are `path = x` or `when`.

* Review names of widget items that have the widget prefix on the name.
    `ImageErrorArgs` could be `image::ErrorArgs`?
* Refactor `#[widget]`.
    - Test build error for parent not a widget.
    - Where is `widget_new!` available for the widget macro?
* Review docs.
    - No more property rename.
    - `#[widget]` docs.
* Test all.
* Merge.

* Update webrender to `60af5fde8115ea5f088c0c2ae07faeae95675200` fx112.
* Review `DefaultStyle` and `base_colors`.

# Other

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