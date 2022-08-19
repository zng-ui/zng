# Bug

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
1411 is ERROR_CLASS_DOES_NOT_EXIST

* This error also happens in master, so all pre-build examples where already running in software mode before.
    - master prebuild redraw (software): 10ms
    - theme-use prebuild redraw (software): 74ms

* Two bugs to fix:
    - See what is making Software render so slow in new theme.
    - Fix glutin to use dedicated mode in pre-build.

# Text

* Text Editable
    - Caret.
    - Selection.
* `text_input!`.
    - Inherit from `text!`.
    - Appearance of a text-box.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline
