* Test animations in two windows, it is causing some panics.
    thread 'main' panicked at 'assertion failed: !self.view_is_rendering()'
* Adjust when respawn stops happening, it can enter an infinite loop in panics like the large image.
* Image fill in scroll gets stretched if the scroll size changes.