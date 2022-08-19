# Themes TODO

* Setting button `background_color` breaks `when`, previously the hovered color was still used.
    - This affects examples that set background in `when` only too, see `window`.

* Image example noticeable slower with new dynamic buttons.
    - Only in prebuild runs, 
        - App-process built in release-lto, <1ms render.
        - Prebuild redraw: 74ms
        - Build redraw: 3ms
        - Prebuild runs in Software mode.
```txt
Render mode selection log:

[Integrated]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
[Dedicated]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
[Dedicated (generic)]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
```


* Test All.
* Merge.

* Document dynamic constructors in `#[widget]`.
* Make more widgets themable.
* Rename all "theme" sub-modules of widgets to `vis`.
* Create a `ColorVars` in `window!` and derive all widget colors from it.