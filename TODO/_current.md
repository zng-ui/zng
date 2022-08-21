* In `animation` example, starting the animation and rapidly hovering buttons causes severe lag.
    - The problem is not in rendering, a release-lto with GPU in same-process profiled always rendered in under 2ms.
    - For most of the lag noting is logged, all idle, then winit starts awaking a lot, but no event is send, then all mouse move
        events are received coalesced into one event.
    - Problem is in the timeout impl, if we don't use `recv_deadline_sp` there is no lag, 
        - Probably `thread::yield_now()`, need to test.

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
