* Test animations in two windows, it is causing some panics.
    thread 'main' panicked at 'assertion failed: !self.view_is_rendering()'
* Adjust when respawn stops happening, it can enter an infinite loop in panics like the large image.
* When reopening an image (like the panorama one in the image example) it doesn't load.

* In the `window` example, if you change from `Fullscreen` to `Exclusive`, or backwards, the position and size of the screen are lost.
    This appears to be happening because winit restores the position and size when going back to `Normal` is by saving it every time for a `set_fullscreen(Some(_))` and restoring it on `set_fullscreen(None)` calls.
    https://github.com/rust-windowing/winit/blob/11a44081df97b82108be63a925d8c479bddfdc4d/src/platform_impl/windows/window.rs#L480