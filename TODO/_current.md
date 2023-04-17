* Layer fade-out.
    - Tooltip does not receive `on_layer_remove_requested` because of internal anon widget around the tip node.
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

* Fix widget docs.
    - Refactor Mix.
        - Collect mix parents, generate a visual section for each.
            - Show `loading..` or something while fetching, the important bit is the link to parent that can
              be used in offline mode, because fetch will not work in that case.
        - Fetch inner mixes because Rust-Doc does not auto include these.
        - Fetch needs to be applied in inherit order, to continue the PROPERTIES override check.
        - Not all parents are linked directly, the inner-most type of the mix-ins may need to fetch too.
        - Avoid fetching the same mix-min parent more then once too, a widget can inherited from `FooMix<Bar>` when
          `Bar` already is `FooMix<WidgetBase>`.

    - Have a fancy tooltip that shows how to call the property in the macro.
        - Same style as the notable trait?

* Implement `TextMix<P>` or even more segmented mix-ins, use then in `Link` and other text widgets?