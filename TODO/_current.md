* Layout offset version changes when an intermediary value is used.
    - If only `y` is set during child layout, then `x` using `with_outer` the version updates.
    - Need a different way to signal change?
        - Can store the previous offsets in the bounds info swapped every time the bounds
          is encountered for the first time.
            - How do parents get signaled in this case?
        - Could do like var updates and delay the "commit" to just after the layout.
            - Needs a vec, the variable one looks like its working fine.
                - But its not every var updating every `update`.
            - How do parents get signaled in this case?
            - Could have a vec but only for the children of the parent, when it goes out of scope commits children and
              then can known if needs to update parent because children.

* Layout context viewport.
    - The image example tries to do this manually, but there is some flickering due to Px rounding on the scroll vs on the offset.
    - CSS has the "position" property, that has sticky, webrender has something for this that we can use?
    - Current idea, have a `layout_parent = LayoutParent::Viewport`.

* Review render_update optimization, need to update children if parent transform changes.
* Implement render optimization, see `Optimizations.md`.
* Review layout double-pass of stacks.
* Fix text final size, either clip or return accurate size.

* Scrolling, see `Scrolling.md`.
* Animation, see `Variables.md`.