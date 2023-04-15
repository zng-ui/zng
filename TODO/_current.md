* Layer fade-out.
    - Test `on_layer_remove_requested`.
    - Implement `layer_remove_delay` using `on_layer_remove_requested`.
    - Implement `is_layer_removing`.
        - Using both delay and this flag a fade-out effect can be easily implemented 
          just by setting properties with `#[easing(..)]` an a `when` condition.

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

# Continue Widget Refactor

* Review `DefaultStyle` and `base_colors`.
* Fix widget docs.
    - Mix-in parent deref not included in docs.
        - Caused by "recursion"?
        - See https://github.com/rust-lang/rust/pull/90183#issuecomment-950215290
    - Inject JS in the docs of widget/mix-in structs to format the properties.
    - They can be identified with the `P` tag.
        Also group in the `unset_property` methods.

* Test widget generated macro in crate that does not depend on zero-ui directly.