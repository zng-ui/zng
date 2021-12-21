* Implement monitor changed event.
  - when monitor changes: See WindowVars::monitor()
  - actual_monitor: Computed by intersection between window and monitors? (the monitor area that contains more than half of the window?)

* Implement and test window respawn (need to test NVIDIA actual driver version change (two blinks)).
* Implement software rendering using https://github.com/servo/webrender/tree/master/swgl
  - Use webrender wrench as reference implementation.
  - What native OpenGL version do we use for software?, need to support textures only.
  - Allow API to select software mode? Helps with testing if nothing else.
  - Headless software does not need a native backend.

* Review screenshot, are we using webrender "async" screenshot correctly?