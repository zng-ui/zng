# Themes TODO

* Setting button `background_color` breaks `when`, previously the hovered color was still used.
    - This affects examples that set background in `when` only too, see `window`.

* Image example noticeable slower with new dynamic buttons.
    - Only in prebuild runs, 
        - App-process built in release-lto, <1ms render.
        - Prebuild redraw: 74ms
        - Build redraw: 3ms


* Test All.
* Merge.

* Document dynamic constructors in `#[widget]`.
* Make more widgets themable.
* Rename all "theme" sub-modules of widgets to `vis`.
* Create a `ColorVars` in `window!` and derive all widget colors from it.