# Mixed Space

* Test everything.
* Implement "auto splitting/grid" for widgets with many children.
    - The `icon` example would be faster if the buttons where split into groups, this idea is to 
        do this splitting automatically internally, without asking the user.
    - The icon example UX is actually improved if we split into groups of the first letter [1|2|A|B|..].
    - This performance boost from splitting also works in the `UiNode` tree?
        - Right now if only one icon button needs layout all icon buttons are visited to check if they need update, if the
          example was split this improves too.
        - Maybe we should print a warning encouraging the user to split widgets.

# Other

* Implement virtualization, at least "auto-virtualization" on the level of widgets to automatically avoid rendering widgets that are not close
to scroll borders.

* Icon example, directional nav wraps around if the next item up is fully clipped, instead of scrolling.
    - Can we make the focus nav know that the focused target will be scrolled to?

* Integrate frame reuse with frame update, see `Optimizations.md`.
* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Finish state API, see `State.md`.