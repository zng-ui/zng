* Reuse hit-test from frame in pending mouse move event.
* The `FrameRendered` is splitting the cursor move coalesce in two.
* Avoid layout/render just after requesting state change? need to know the state actually changed.