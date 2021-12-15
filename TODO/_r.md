* Test animations in two windows, it is causing some panics.
    thread 'main' panicked at 'assertion failed: !self.view_is_rendering()'
* Adjust when respawn stops happening, it can enter an infinite loop in panics like the large image.
* Build tests harness does not build. (error is this: https://github.com/rust-lang/cargo/issues/6915)
    restructure project using crates for tests and examples
* When reopening an image (like the panorama one in the image example) it doesn't load.
