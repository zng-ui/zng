# Bug

* Fix pre-build view-process is always software mode because it fails to create `Dedicated`.
```txt
Render mode selection log:

[Integrated]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
[Dedicated]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
[Dedicated (generic)]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
1411 is ERROR_CLASS_DOES_NOT_EXIST
```
- Error caused by `GetModuleHandle(NULL)` call in glutin's `load_extra_functions`.
    - This call always returns the executable handle, but window is created in a DLL in pre-build.
    - Winit has a recent pull request that fixes this: <https://github.com/rust-windowing/winit/pull/2301>
    - Glutin is undergoing a large rewrite that removes dependency in `winit` and removes the `load_extra_functions`.
        - Need to monitor this draft <https://github.com/rust-windowing/glutin/pull/1435> to see what the new Windows impl will be like.

* Software mode rendering in the `image` example is much slower in the `theme-use` branch.
    - Bug is not caused by button theme, using the old button in the new branch was still slow.
    - Bug is not caused by any change in image example code, old image code was still slow.
    - Bug caused by window `background_color` becoming dynamic because it is bound theme now.
        - Dynamic colors become a frame binding, webrender skips some caches because of frame bindings.
            - This causes a full frame redraw in software, the image example opens maximized, more pixels causes the impact to become noticeable.
        - Firefox only generates frame-bindings for animations.
            - We removed an implementation of this, because webrender frame update was faster than a render and render only gained ~2ms in tests.
            - Software rendering is much more affected by this, maybe we can have the "animating" flag in each binding and ignore it if the backend 
              is not software.
            - Need to review exactly when Firefox decides to create a frame binding.
                - What if the value is animated in JS for example.
        - We can ignore all bindings if the backend is software?
            - Causes even slower animations?

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
