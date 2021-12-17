* Test animations in two windows, it is causing some panics.
    thread 'main' panicked at 'assertion failed: !self.view_is_rendering()'
* Adjust when respawn stops happening, it can enter an infinite loop in panics like the large image.
* When reopening an image (like the panorama one in the image example) it doesn't load.
