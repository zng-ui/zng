* In the window example the `Maximized` button is considerably slower to render the next frame than when maximizing using Windows' button.
    - Refactor view code to be immune to this kind of bugs.
    - Refactor app code to not do layout and render pass if a resize request was sent.

* Reuse hit-test from frame in pending mouse move event.
* The `FrameRendered` is splitting the cursor move coalesce in two.